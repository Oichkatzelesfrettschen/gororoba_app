// LBM engine resources wrapping open_gororoba solvers.
//
// LbmCpuEngine wraps lbm_3d::LbmSolver3D for CPU-based simulation.
// LbmGpuEngine wraps lbm_vulkan::GororobaEngine for GPU compute.

use bevy::prelude::*;
use lbm_3d::boundary::BounceBackBoundary;
use lbm_3d::solver::LbmSolver3D;

use crate::components::VoxelGrid;

/// CPU-based LBM engine wrapping lbm_3d::LbmSolver3D.
///
/// This is the primary solver: pure Rust, rayon-parallel, no GPU needed.
/// Each FluidDomain entity gets its own solver instance managed here.
#[derive(Resource, Default)]
pub struct LbmCpuEngine {
    /// Active solver instances keyed by entity ID.
    pub solvers: Vec<(Entity, SolverInstance)>,
}

pub struct SolverInstance {
    pub solver: LbmSolver3D,
    pub nx: usize,
    pub ny: usize,
    pub nz: usize,
}

/// Configuration for creating a new solver instance.
pub struct SolverConfig {
    pub nx: usize,
    pub ny: usize,
    pub nz: usize,
    pub tau: f64,
    pub rho_init: f64,
    pub u_init: [f64; 3],
}

impl LbmCpuEngine {
    /// Create a new solver for the given entity and configuration.
    pub fn create_solver(&mut self, entity: Entity, config: &SolverConfig) {
        // Remove existing solver for this entity if any.
        self.solvers.retain(|(e, _)| *e != entity);

        let mut solver = LbmSolver3D::new(config.nx, config.ny, config.nz, config.tau);
        solver.initialize_uniform(config.rho_init, config.u_init);
        self.solvers.push((
            entity,
            SolverInstance {
                solver,
                nx: config.nx,
                ny: config.ny,
                nz: config.nz,
            },
        ));
    }

    /// Get mutable reference to a solver by entity.
    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut SolverInstance> {
        self.solvers
            .iter_mut()
            .find(|(e, _)| *e == entity)
            .map(|(_, s)| s)
    }

    /// Get reference to a solver by entity.
    pub fn get(&self, entity: Entity) -> Option<&SolverInstance> {
        self.solvers
            .iter()
            .find(|(e, _)| *e == entity)
            .map(|(_, s)| s)
    }

    /// Remove solver for entity.
    pub fn remove(&mut self, entity: Entity) {
        self.solvers.retain(|(e, _)| *e != entity);
    }
}

impl SolverInstance {
    /// Read the density field as a flat Vec<f32>.
    pub fn read_density_field(&self) -> Vec<f32> {
        let n = self.nx * self.ny * self.nz;
        let mut out = Vec::with_capacity(n);
        for z in 0..self.nz {
            for y in 0..self.ny {
                for x in 0..self.nx {
                    let (rho, _) = self.solver.get_macroscopic(x, y, z);
                    out.push(rho as f32);
                }
            }
        }
        out
    }

    /// Read the velocity field as flat Vec<f32> with [vx, vy, vz] per cell.
    pub fn read_velocity_field(&self) -> Vec<f32> {
        let n = self.nx * self.ny * self.nz;
        let mut out = Vec::with_capacity(n * 3);
        for z in 0..self.nz {
            for y in 0..self.ny {
                for x in 0..self.nx {
                    let (_, u) = self.solver.get_macroscopic(x, y, z);
                    out.push(u[0] as f32);
                    out.push(u[1] as f32);
                    out.push(u[2] as f32);
                }
            }
        }
        out
    }

    /// Inject bounce-back boundaries from a voxel grid.
    ///
    /// For each solid voxel, sets a high viscosity (effectively solid)
    /// in the tau field. This is a practical workaround until lbm_3d
    /// exposes per-node bounce-back injection.
    pub fn inject_boundary_from_voxels(&mut self, voxels: &VoxelGrid) {
        assert_eq!(voxels.nx, self.nx);
        assert_eq!(voxels.ny, self.ny);
        assert_eq!(voxels.nz, self.nz);

        let n = self.nx * self.ny * self.nz;
        let base_tau = self.solver.collider.viscosity() * 3.0 + 0.5;
        // Solid cells get very high tau (extremely viscous = effectively solid).
        // Fluid cells keep the original tau.
        let tau_field: Vec<f64> = (0..n)
            .map(|i| if voxels.cells[i] { 1e6 } else { base_tau })
            .collect();

        if let Err(e) = self.solver.set_viscosity_field(tau_field) {
            warn!("Failed to inject voxel boundaries: {e}");
        }
    }

    /// Apply bounce-back on top and bottom faces using lbm_3d's boundary module.
    pub fn apply_bounce_back_planes(&mut self) {
        let bb = BounceBackBoundary::new();
        // Apply on Y-min and Y-max planes (floor and ceiling).
        bb.apply_on_plane(
            &mut self.solver.f,
            self.nx,
            self.ny,
            self.nz,
            lbm_3d::boundary::BoundaryPlane::MinY,
        );
        bb.apply_on_plane(
            &mut self.solver.f,
            self.nx,
            self.ny,
            self.nz,
            lbm_3d::boundary::BoundaryPlane::MaxY,
        );
    }

    /// Compute aerodynamic diagnostics from the velocity and density fields.
    pub fn compute_drag_lift(
        &self,
        voxels: &VoxelGrid,
        freestream_velocity: [f64; 3],
    ) -> (f64, f64) {
        // Momentum exchange method: sum forces on solid boundary nodes.
        let mut drag = 0.0; // Force in freestream direction (x).
        let mut lift = 0.0; // Force perpendicular to freestream (y).

        let lattice = lbm_3d::lattice::D3Q19Lattice::new();

        for z in 1..self.nz - 1 {
            for y in 1..self.ny - 1 {
                for x in 1..self.nx - 1 {
                    if !voxels.get(x, y, z) {
                        continue;
                    }
                    // Check neighbors: for each fluid neighbor, accumulate
                    // momentum exchange.
                    for i in 1..19 {
                        let v = lattice.velocity(i);
                        let nx = x as i32 + v[0];
                        let ny = y as i32 + v[1];
                        let nz = z as i32 + v[2];
                        if nx < 0
                            || ny < 0
                            || nz < 0
                            || nx >= self.nx as i32
                            || ny >= self.ny as i32
                            || nz >= self.nz as i32
                        {
                            continue;
                        }
                        let (nx, ny, nz) = (nx as usize, ny as usize, nz as usize);
                        if voxels.get(nx, ny, nz) {
                            continue; // Both solid, skip.
                        }
                        // Fluid neighbor: momentum exchange contribution.
                        let (rho, u) = self.solver.get_macroscopic(nx, ny, nz);
                        let du = [
                            u[0] - freestream_velocity[0],
                            u[1] - freestream_velocity[1],
                            u[2] - freestream_velocity[2],
                        ];
                        drag += rho * du[0] * lattice.weight(i);
                        lift += rho * du[1] * lattice.weight(i);
                    }
                }
            }
        }

        (drag, lift)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config(nx: usize, ny: usize, nz: usize) -> SolverConfig {
        SolverConfig {
            nx,
            ny,
            nz,
            tau: 0.8,
            rho_init: 1.0,
            u_init: [0.0, 0.0, 0.0],
        }
    }

    #[test]
    fn cpu_engine_create_and_lookup() {
        let mut engine = LbmCpuEngine::default();
        let entity = Entity::from_bits(42);
        engine.create_solver(entity, &test_config(8, 8, 8));

        assert!(engine.get(entity).is_some());
        assert!(engine.get(Entity::from_bits(99)).is_none());

        let inst = engine.get(entity).unwrap();
        assert_eq!(inst.nx, 8);
    }

    #[test]
    fn cpu_engine_remove() {
        let mut engine = LbmCpuEngine::default();
        let entity = Entity::from_bits(1);
        engine.create_solver(entity, &test_config(4, 4, 4));
        engine.remove(entity);
        assert!(engine.get(entity).is_none());
    }

    #[test]
    fn density_field_readback() {
        let mut engine = LbmCpuEngine::default();
        let entity = Entity::from_bits(1);
        engine.create_solver(entity, &test_config(4, 4, 4));

        let inst = engine.get(entity).unwrap();
        let rho = inst.read_density_field();
        assert_eq!(rho.len(), 64); // 4*4*4
        // All cells initialized to rho=1.0
        for &r in &rho {
            assert!((r - 1.0).abs() < 1e-4, "Expected rho ~1.0, got {r}");
        }
    }
}
