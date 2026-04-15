use bevy::prelude::*;
use gororoba_kernel_api::fluid::{
    AerodynamicSnapshot, FluidBackendCapabilities, FluidBackendError, FluidBackendKind,
    FluidBackendPreference, FluidDiagnosticsSnapshot, FluidDomainConfig, FluidExecutionConfig,
    FluidFieldSnapshot, FluidKernel, GridShape3,
};
use gororoba_kernel_fluid::{LocalCpuFluidKernel, VoxelMask3};

#[cfg(feature = "fluid-cuda")]
use gororoba_kernel_fluid_cuda::{probe_cuda_capabilities, try_create_cuda_kernel};
#[cfg(feature = "fluid-vulkan")]
use gororoba_kernel_fluid_vulkan::{probe_vulkan_capabilities, try_create_vulkan_kernel};

use crate::components::VoxelGrid;

#[derive(Resource, Default)]
pub struct FluidSimulationEngine {
    pub solvers: Vec<(Entity, SolverInstance)>,
}

pub type LbmCpuEngine = FluidSimulationEngine;

pub struct SolverInstance {
    pub kernel: Box<dyn FluidKernel + Send + Sync>,
    pub capabilities: FluidBackendCapabilities,
}

pub struct SolverConfig {
    pub nx: usize,
    pub ny: usize,
    pub nz: usize,
    pub tau: f64,
    pub rho_init: f64,
    pub u_init: [f64; 3],
    pub force: [f64; 3],
    pub substeps: usize,
    pub execution: FluidExecutionConfig,
}

impl FluidSimulationEngine {
    pub fn create_solver(
        &mut self,
        entity: Entity,
        config: &SolverConfig,
    ) -> Result<(), FluidBackendError> {
        self.solvers.retain(|(existing, _)| *existing != entity);

        let domain_config = FluidDomainConfig {
            grid: GridShape3 {
                nx: config.nx,
                ny: config.ny,
                nz: config.nz,
            },
            tau: config.tau as f32,
            rho_init: config.rho_init as f32,
            u_init: [
                config.u_init[0] as f32,
                config.u_init[1] as f32,
                config.u_init[2] as f32,
            ],
            force: [
                config.force[0] as f32,
                config.force[1] as f32,
                config.force[2] as f32,
            ],
            substeps: config.substeps,
            execution: config.execution,
            boundaries: Default::default(),
        };

        let capabilities = probe_backend_capabilities();
        let mut kernel = build_kernel(&domain_config, &capabilities)?;
        let force_field = vec![domain_config.force; domain_config.grid.cell_count()];
        kernel.set_force_field(&force_field)?;

        self.solvers.push((
            entity,
            SolverInstance {
                kernel,
                capabilities,
            },
        ));

        Ok(())
    }

    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut SolverInstance> {
        self.solvers
            .iter_mut()
            .find(|(existing, _)| *existing == entity)
            .map(|(_, solver)| solver)
    }

    pub fn get(&self, entity: Entity) -> Option<&SolverInstance> {
        self.solvers
            .iter()
            .find(|(existing, _)| *existing == entity)
            .map(|(_, solver)| solver)
    }

    pub fn remove(&mut self, entity: Entity) {
        self.solvers.retain(|(existing, _)| *existing != entity);
    }
}

impl SolverInstance {
    fn mask_from_voxels(voxels: &VoxelGrid) -> VoxelMask3 {
        VoxelMask3::from_cells(voxels.nx, voxels.ny, voxels.nz, voxels.cells.clone())
    }

    pub fn domain_config(&self) -> &FluidDomainConfig {
        self.kernel.domain_config()
    }

    pub fn selected_backend(&self) -> FluidBackendKind {
        self.kernel.selected_backend()
    }

    pub fn diagnostics(&self) -> FluidDiagnosticsSnapshot {
        self.kernel.diagnostics()
    }

    pub fn field_snapshot(&self) -> FluidFieldSnapshot {
        self.kernel.field_snapshot()
    }

    pub fn read_density_field(&self) -> Vec<f32> {
        self.field_snapshot().density
    }

    pub fn read_velocity_field(&self) -> Vec<f32> {
        self.field_snapshot().velocity_xyz
    }

    pub fn get_macroscopic(&self, x: usize, y: usize, z: usize) -> (f32, [f32; 3]) {
        let cfg = self.domain_config();
        let idx = z * (cfg.grid.nx * cfg.grid.ny) + y * cfg.grid.nx + x;
        let snapshot = self.field_snapshot();
        let velocity_base = idx * 3;
        (
            snapshot.density[idx],
            [
                snapshot.velocity_xyz[velocity_base],
                snapshot.velocity_xyz[velocity_base + 1],
                snapshot.velocity_xyz[velocity_base + 2],
            ],
        )
    }

    pub fn inject_boundary_from_voxels(&mut self, voxels: &VoxelGrid) {
        let mask = Self::mask_from_voxels(voxels);
        self.kernel.set_voxel_mask(&mask.cells);
    }

    pub fn evolve_with_boundaries(&mut self, substeps: usize) {
        self.kernel.step(substeps);
    }

    pub fn compute_drag_lift(&self, voxels: &VoxelGrid) -> (f64, f64) {
        let snapshot = self.kernel.aerodynamic_snapshot(
            &voxels.cells,
            self.domain_config().u_init,
            self.domain_config().tau,
        );
        (snapshot.drag, snapshot.lift)
    }

    pub fn aerodynamic_snapshot(&self, voxels: &VoxelGrid) -> AerodynamicSnapshot {
        self.kernel.aerodynamic_snapshot(
            &voxels.cells,
            self.domain_config().u_init,
            self.domain_config().tau,
        )
    }
}

fn build_kernel(
    domain_config: &FluidDomainConfig,
    capabilities: &FluidBackendCapabilities,
) -> Result<Box<dyn FluidKernel + Send + Sync>, FluidBackendError> {
    match domain_config.execution.preference {
        FluidBackendPreference::CpuOnly => Ok(Box::new(LocalCpuFluidKernel::from_domain_config(
            domain_config,
        ))),
        FluidBackendPreference::CudaPreferred => {
            try_cuda_then_fallback(domain_config, capabilities, false)
        }
        FluidBackendPreference::VulkanPreferred => {
            try_vulkan_then_fallback(domain_config, capabilities, false)
        }
        FluidBackendPreference::Auto => try_cuda_then_fallback(domain_config, capabilities, true),
    }
}

fn try_cuda_then_fallback(
    domain_config: &FluidDomainConfig,
    capabilities: &FluidBackendCapabilities,
    allow_vulkan_fallback: bool,
) -> Result<Box<dyn FluidKernel + Send + Sync>, FluidBackendError> {
    if capabilities.cuda.is_some() {
        #[cfg(feature = "fluid-cuda")]
        if let Ok(kernel) = try_create_cuda_kernel(domain_config, capabilities.clone()) {
            return Ok(kernel);
        }
    }

    if allow_vulkan_fallback {
        return try_vulkan_then_fallback(domain_config, capabilities, true);
    }

    Ok(Box::new(LocalCpuFluidKernel::from_domain_config(
        domain_config,
    )))
}

fn try_vulkan_then_fallback(
    domain_config: &FluidDomainConfig,
    capabilities: &FluidBackendCapabilities,
    allow_cpu_fallback: bool,
) -> Result<Box<dyn FluidKernel + Send + Sync>, FluidBackendError> {
    if capabilities.vulkan.is_some() {
        #[cfg(feature = "fluid-vulkan")]
        if let Ok(kernel) = try_create_vulkan_kernel(domain_config, capabilities.clone()) {
            return Ok(kernel);
        }
    }

    if allow_cpu_fallback {
        return Ok(Box::new(LocalCpuFluidKernel::from_domain_config(
            domain_config,
        )));
    }

    Ok(Box::new(LocalCpuFluidKernel::from_domain_config(
        domain_config,
    )))
}

fn probe_backend_capabilities() -> FluidBackendCapabilities {
    #[allow(unused_mut)]
    let mut caps = FluidBackendCapabilities::cpu_only_detected();

    #[cfg(feature = "fluid-vulkan")]
    {
        caps.vulkan = probe_vulkan_capabilities();
    }

    #[cfg(feature = "fluid-cuda")]
    {
        caps.cuda = probe_cuda_capabilities();
    }

    caps
}

#[cfg(test)]
mod tests {
    use super::*;
    use gororoba_kernel_api::fluid::CpuKernelFlavor;

    fn test_config(nx: usize, cpu_flavor: CpuKernelFlavor) -> SolverConfig {
        SolverConfig {
            nx,
            ny: nx,
            nz: nx,
            tau: 0.8,
            rho_init: 1.0,
            u_init: [0.01, 0.0, 0.0],
            force: [0.0; 3],
            substeps: 1,
            execution: FluidExecutionConfig {
                cpu_flavor,
                ..Default::default()
            },
        }
    }

    #[test]
    fn create_solver_uses_cpu_soa_by_default() {
        let mut engine = FluidSimulationEngine::default();
        let entity = Entity::from_bits(1);
        engine
            .create_solver(entity, &test_config(8, CpuKernelFlavor::SoA))
            .unwrap();
        let solver = engine.get(entity).unwrap();
        assert_eq!(solver.selected_backend(), FluidBackendKind::CpuSoA);
    }

    #[test]
    fn create_solver_supports_scalar_cpu_flavor() {
        let mut engine = FluidSimulationEngine::default();
        let entity = Entity::from_bits(2);
        engine
            .create_solver(entity, &test_config(8, CpuKernelFlavor::Scalar))
            .unwrap();
        let solver = engine.get(entity).unwrap();
        assert_eq!(solver.selected_backend(), FluidBackendKind::CpuScalar);
    }

    #[test]
    fn solver_instance_exposes_fluid_kernel_snapshots() {
        let mut engine = FluidSimulationEngine::default();
        let entity = Entity::from_bits(3);
        engine
            .create_solver(entity, &test_config(6, CpuKernelFlavor::SoA))
            .unwrap();
        let solver = engine.get(entity).unwrap();
        let diagnostics = solver.diagnostics();
        let snapshot = solver.field_snapshot();
        assert_eq!(
            diagnostics.backend,
            gororoba_kernel_api::algebra::KernelBackend::Cpu
        );
        assert_eq!(snapshot.density.len(), 6 * 6 * 6);
        assert_eq!(snapshot.velocity_xyz.len(), 6 * 6 * 6 * 3);
    }

    #[cfg(feature = "fluid-vulkan")]
    #[test]
    fn build_kernel_selects_vulkan_backend_when_requested() {
        let domain_config = FluidDomainConfig {
            grid: GridShape3 {
                nx: 4,
                ny: 4,
                nz: 4,
            },
            tau: 0.8,
            rho_init: 1.0,
            u_init: [0.0; 3],
            force: [0.0; 3],
            substeps: 1,
            execution: FluidExecutionConfig {
                preference: FluidBackendPreference::VulkanPreferred,
                cpu_flavor: CpuKernelFlavor::SoA,
                ..Default::default()
            },
            boundaries: FluidBoundaryConfig::default(),
        };
        let capabilities = FluidBackendCapabilities {
            cpu: CpuCapabilities::detect(),
            vulkan: Some(VulkanCapabilities {
                device_name: "test-vulkan".to_string(),
                driver: Some("1".to_string()),
                api_version: Some("1.3.0".to_string()),
                vram_mb: Some(4096),
                supports_fp16: true,
                supports_fp64: true,
                max_compute_shared_memory_size: Some(32768),
            }),
            cuda: None,
        };

        let kernel = build_kernel(&domain_config, &capabilities).unwrap();
        assert_eq!(kernel.selected_backend(), FluidBackendKind::Vulkan);
    }

    #[cfg(feature = "fluid-cuda")]
    #[test]
    fn build_kernel_prefers_cuda_in_auto_mode_when_available() {
        let domain_config = FluidDomainConfig {
            grid: GridShape3 {
                nx: 4,
                ny: 4,
                nz: 4,
            },
            tau: 0.8,
            rho_init: 1.0,
            u_init: [0.0; 3],
            force: [0.0; 3],
            substeps: 1,
            execution: FluidExecutionConfig {
                preference: FluidBackendPreference::Auto,
                cpu_flavor: CpuKernelFlavor::SoA,
                ..Default::default()
            },
            boundaries: FluidBoundaryConfig::default(),
        };
        let capabilities = FluidBackendCapabilities {
            cpu: CpuCapabilities::detect(),
            vulkan: Some(VulkanCapabilities {
                device_name: "test-vulkan".to_string(),
                driver: Some("1".to_string()),
                api_version: Some("1.3.0".to_string()),
                vram_mb: Some(4096),
                supports_fp16: true,
                supports_fp64: true,
                max_compute_shared_memory_size: Some(32768),
            }),
            cuda: Some(CudaCapabilities {
                device_name: "test-cuda".to_string(),
                driver_version: Some("1".to_string()),
                cuda_version: Some("13.1".to_string()),
                compute_capability: Some("8.9".to_string()),
                total_memory_mb: Some(12288),
            }),
        };

        let kernel = build_kernel(&domain_config, &capabilities).unwrap();
        assert_eq!(kernel.selected_backend(), FluidBackendKind::Cuda);
    }
}
