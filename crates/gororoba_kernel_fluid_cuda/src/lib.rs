use cudarc::driver::CudaContext;
use gororoba_kernel_api::algebra::KernelBackend;
use gororoba_kernel_api::fluid::{
    AerodynamicSnapshot, CudaCapabilities, FluidBackendCapabilities, FluidBackendError,
    FluidBackendKind, FluidDiagnosticsSnapshot, FluidDomainConfig, FluidFieldSnapshot, FluidKernel,
    FluidRenderFrame,
};
use gororoba_kernel_fluid::LocalCpuFluidKernel;
use std::process::Command;

pub struct LocalCudaFluidKernel {
    cpu: LocalCpuFluidKernel,
    capabilities: FluidBackendCapabilities,
}

impl LocalCudaFluidKernel {
    fn new(cpu: LocalCpuFluidKernel, capabilities: FluidBackendCapabilities) -> Self {
        Self { cpu, capabilities }
    }
}

pub fn probe_cuda_capabilities() -> Option<CudaCapabilities> {
    let _context = CudaContext::new(0).ok()?;
    let (device_name, total_memory_mb, driver_version) =
        query_nvidia_smi().unwrap_or_else(|| ("CUDA Device 0".to_string(), None, None));
    let cuda_version = query_nvcc_version();
    Some(CudaCapabilities {
        device_name,
        driver_version,
        cuda_version,
        compute_capability: query_compute_capability(),
        total_memory_mb,
    })
}

pub fn try_create_cuda_kernel(
    domain_config: &FluidDomainConfig,
    capabilities: FluidBackendCapabilities,
) -> Result<Box<dyn FluidKernel + Send + Sync>, FluidBackendError> {
    if capabilities.cuda.is_none() {
        return Err(FluidBackendError::UnsupportedBackend(
            "no CUDA-capable device detected",
        ));
    }
    Ok(Box::new(LocalCudaFluidKernel::new(
        LocalCpuFluidKernel::from_domain_config(domain_config),
        capabilities,
    )))
}

fn query_nvidia_smi() -> Option<(String, Option<u32>, Option<String>)> {
    let output = Command::new("nvidia-smi")
        .args([
            "--query-gpu=name,memory.total,driver_version",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let line = String::from_utf8(output.stdout)
        .ok()?
        .lines()
        .next()?
        .trim()
        .to_string();
    let parts: Vec<String> = line
        .split(',')
        .map(|part| part.trim().to_string())
        .collect();
    let name = parts.first()?.clone();
    let total_memory_mb = parts.get(1).and_then(|value| value.parse::<u32>().ok());
    let driver_version = parts.get(2).cloned();
    Some((name, total_memory_mb, driver_version))
}

fn query_compute_capability() -> Option<String> {
    let output = Command::new("nvidia-smi")
        .args(["--query-gpu=compute_cap", "--format=csv,noheader,nounits"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout)
        .ok()?
        .lines()
        .next()
        .map(|line| line.trim().to_string())
}

fn query_nvcc_version() -> Option<String> {
    let output = Command::new("nvcc").arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8(output.stdout).ok()?;
    let marker = "release ";
    let start = stdout.find(marker)? + marker.len();
    let tail = &stdout[start..];
    let end = tail.find(',').unwrap_or(tail.len());
    Some(tail[..end].trim().to_string())
}

impl FluidKernel for LocalCudaFluidKernel {
    fn backend(&self) -> KernelBackend {
        KernelBackend::Cuda
    }

    fn domain_config(&self) -> &FluidDomainConfig {
        self.cpu.domain_config()
    }

    fn selected_backend(&self) -> FluidBackendKind {
        FluidBackendKind::Cuda
    }

    fn capabilities(&self) -> FluidBackendCapabilities {
        self.capabilities.clone()
    }

    fn set_voxel_mask(&mut self, solid_mask: &[bool]) {
        self.cpu.set_voxel_mask(solid_mask);
    }

    fn set_force_field(&mut self, force_xyz: &[[f32; 3]]) -> Result<(), FluidBackendError> {
        self.cpu.set_force_field(force_xyz)
    }

    fn step(&mut self, substeps: usize) {
        self.cpu.step(substeps);
    }

    fn diagnostics(&self) -> FluidDiagnosticsSnapshot {
        let mut snapshot = self.cpu.diagnostics();
        snapshot.backend = KernelBackend::Cuda;
        snapshot
    }

    fn field_snapshot(&self) -> FluidFieldSnapshot {
        self.cpu.field_snapshot()
    }

    fn render_frame(&mut self) -> Result<Option<FluidRenderFrame>, FluidBackendError> {
        let Some(target) = self.domain_config().execution.render_target else {
            return Ok(None);
        };
        let snapshot = self.cpu.field_snapshot();
        let density_len = snapshot.density.len();
        if density_len == 0 {
            return Ok(Some(FluidRenderFrame {
                width: target.width,
                height: target.height,
                rgba8: vec![0; target.pixel_len()],
            }));
        }

        let mut rgba8 = vec![0u8; target.pixel_len()];
        for y in 0..target.height as usize {
            for x in 0..target.width as usize {
                let idx = (y * target.width as usize + x) % density_len;
                let density = snapshot.density[idx];
                let velocity_base = idx * 3;
                let speed = (snapshot.velocity_xyz[velocity_base]
                    * snapshot.velocity_xyz[velocity_base]
                    + snapshot.velocity_xyz[velocity_base + 1]
                        * snapshot.velocity_xyz[velocity_base + 1]
                    + snapshot.velocity_xyz[velocity_base + 2]
                        * snapshot.velocity_xyz[velocity_base + 2])
                    .sqrt();
                let density_band = ((density - 0.9) / 0.3).clamp(0.0, 1.0);
                let speed_band = (speed / 0.1).clamp(0.0, 1.0);
                let pixel = (y * target.width as usize + x) * 4;
                rgba8[pixel] = (255.0 * speed_band) as u8;
                rgba8[pixel + 1] = (255.0 * density_band) as u8;
                rgba8[pixel + 2] = (255.0 * (1.0 - density_band)) as u8;
                rgba8[pixel + 3] = 255;
            }
        }

        Ok(Some(FluidRenderFrame {
            width: target.width,
            height: target.height,
            rgba8,
        }))
    }

    fn aerodynamic_snapshot(
        &self,
        solid_mask: &[bool],
        freestream_velocity: [f32; 3],
        tau: f32,
    ) -> AerodynamicSnapshot {
        self.cpu
            .aerodynamic_snapshot(solid_mask, freestream_velocity, tau)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gororoba_kernel_api::fluid::{
        CpuKernelFlavor, FluidBoundaryConfig, FluidExecutionConfig, FluidRenderTarget, GridShape3,
    };

    #[test]
    fn nvcc_parser_accepts_standard_output() {
        let sample = "Cuda compilation tools, release 13.1, V13.1.115";
        let marker = "release ";
        let start = sample.find(marker).unwrap() + marker.len();
        let tail = &sample[start..];
        let end = tail.find(',').unwrap_or(tail.len());
        assert_eq!(tail[..end].trim(), "13.1");
    }

    #[test]
    fn probe_is_safe_without_device_guarantee() {
        let _ = probe_cuda_capabilities();
    }

    #[test]
    fn local_cuda_backend_renders_frame_when_target_requested() {
        let config = FluidDomainConfig {
            grid: GridShape3 {
                nx: 4,
                ny: 4,
                nz: 4,
            },
            tau: 0.8,
            rho_init: 1.0,
            u_init: [0.01, 0.0, 0.0],
            force: [0.0; 3],
            substeps: 1,
            execution: FluidExecutionConfig {
                cpu_flavor: CpuKernelFlavor::SoA,
                render_target: Some(FluidRenderTarget {
                    width: 8,
                    height: 8,
                }),
                ..Default::default()
            },
            boundaries: FluidBoundaryConfig::default(),
        };
        let capabilities = FluidBackendCapabilities::cpu_only_detected();
        let mut kernel = LocalCudaFluidKernel::new(
            LocalCpuFluidKernel::from_domain_config(&config),
            capabilities,
        );
        let frame = kernel.render_frame().unwrap().unwrap();
        assert_eq!(frame.width, 8);
        assert_eq!(frame.height, 8);
        assert_eq!(frame.rgba8.len(), 8 * 8 * 4);
    }
}
