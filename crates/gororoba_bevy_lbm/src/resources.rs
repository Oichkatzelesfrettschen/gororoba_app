// LBM engine resources wrapping open_gororoba solvers.
//
// LbmCpuEngine wraps lbm_3d::LbmSolver3D for CPU-based simulation.
// Solid boundaries use proper bounce-back (distribution function
// reflection) rather than the viscosity-field workaround.

use bevy::prelude::*;
use lbm_3d::boundary::BounceBackBoundary;
use lbm_3d::solver::{BgkCollision, LbmSolver3D};

use crate::components::VoxelGrid;
use crate::soa_solver::LbmSolverSoA;

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
    /// Solid voxel mask for bounce-back boundary conditions.
    /// Applied after every streaming step to enforce no-slip walls.
    pub voxel_mask: Vec<bool>,
    /// Cached bounce-back operator (avoids re-creating D3Q19 lattice).
    bounce_back: BounceBackBoundary,
    /// Freestream velocity for inlet boundary condition (lattice units).
    pub freestream_velocity: [f64; 3],
    /// Freestream density for inlet boundary condition.
    pub freestream_density: f64,
    /// High-performance SoA solver (f32 perturbation formulation).
    /// When present, evolve/readback methods delegate here instead of
    /// using the upstream f64 AoS solver.
    pub soa: Option<LbmSolverSoA>,
}

/// Configuration for creating a new solver instance.
pub struct SolverConfig {
    pub nx: usize,
    pub ny: usize,
    pub nz: usize,
    pub tau: f64,
    pub rho_init: f64,
    pub u_init: [f64; 3],
    /// When true, creates a high-performance SoA f32 solver alongside
    /// the upstream f64 solver. The SoA solver handles evolution and
    /// readback; the f64 solver remains for compatibility.
    pub use_soa: bool,
}

impl LbmCpuEngine {
    /// Create a new solver for the given entity and configuration.
    pub fn create_solver(&mut self, entity: Entity, config: &SolverConfig) {
        // Remove existing solver for this entity if any.
        self.solvers.retain(|(e, _)| *e != entity);

        let mut solver = LbmSolver3D::new(config.nx, config.ny, config.nz, config.tau);
        solver.initialize_uniform(config.rho_init, config.u_init);
        let n = config.nx * config.ny * config.nz;

        let soa = if config.use_soa {
            let mut s = LbmSolverSoA::new(config.nx, config.ny, config.nz, config.tau as f32);
            s.initialize_uniform(
                config.rho_init as f32,
                [
                    config.u_init[0] as f32,
                    config.u_init[1] as f32,
                    config.u_init[2] as f32,
                ],
            );
            Some(s)
        } else {
            None
        };

        self.solvers.push((
            entity,
            SolverInstance {
                solver,
                nx: config.nx,
                ny: config.ny,
                nz: config.nz,
                voxel_mask: vec![false; n],
                bounce_back: BounceBackBoundary::new(),
                freestream_velocity: config.u_init,
                freestream_density: config.rho_init,
                soa,
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
        if let Some(soa) = &self.soa {
            return soa.read_density_field();
        }
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
        if let Some(soa) = &self.soa {
            return soa.read_velocity_field();
        }
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

    /// Get macroscopic quantities at a specific grid point.
    ///
    /// Delegates to SoA solver when present, otherwise uses upstream f64.
    /// Returns (rho, [ux, uy, uz]) as f64 for compatibility.
    pub fn get_macroscopic(&self, x: usize, y: usize, z: usize) -> (f64, [f64; 3]) {
        if let Some(soa) = &self.soa {
            let (rho, u) = soa.get_macroscopic(x, y, z);
            return (rho as f64, [u[0] as f64, u[1] as f64, u[2] as f64]);
        }
        self.solver.get_macroscopic(x, y, z)
    }

    /// Store the voxel mask and apply initial bounce-back.
    ///
    /// Solid cells get proper bounce-back boundary conditions: their
    /// distribution functions are reflected so that no-slip walls appear
    /// wherever the voxel grid is true. This is applied after every
    /// streaming step via `evolve_with_boundaries()`.
    pub fn inject_boundary_from_voxels(&mut self, voxels: &VoxelGrid) {
        assert_eq!(voxels.nx, self.nx);
        assert_eq!(voxels.ny, self.ny);
        assert_eq!(voxels.nz, self.nz);

        // Store the voxel mask for per-step bounce-back.
        self.voxel_mask = voxels.cells.clone();

        // Apply bounce-back once on the initial distribution to enforce
        // no-slip at solid cells from the very first timestep.
        self.bounce_back.inject_boundary_from_voxels(
            &mut self.solver.f,
            &self.voxel_mask,
            self.nx,
            self.ny,
            self.nz,
        );

        // Mirror to SoA solver if active.
        if let Some(soa) = &mut self.soa {
            soa.inject_boundary_from_voxels(voxels);
        }
    }

    /// Advance the simulation by `substeps` timesteps, applying
    /// bounce-back boundary conditions after every streaming step.
    ///
    /// Boundary application order per substep:
    /// 1. Collision + streaming (solver internals)
    /// 2. Solid voxel bounce-back (obstacle no-slip)
    /// 3. Equilibrium inlet + zero-gradient outlet (sustains freestream)
    /// 4. Wall bounce-back on Y and Z planes (overwrites corners)
    ///
    /// After all substeps, macroscopic fields are recomputed so that
    /// velocity/density readbacks reflect the applied BCs.
    pub fn evolve_with_boundaries(&mut self, substeps: usize) {
        // Delegate to SoA solver when active.
        if let Some(soa) = &mut self.soa {
            soa.evolve_with_boundaries(substeps);
            return;
        }

        let has_solids = self.voxel_mask.iter().any(|&s| s);
        let has_freestream = self.freestream_velocity.iter().any(|v| v.abs() > 1e-12);
        for _ in 0..substeps {
            let _ = self.solver.phase1_collision();
            let _ = self.solver.phase2_streaming();
            if has_solids {
                self.bounce_back.inject_boundary_from_voxels(
                    &mut self.solver.f,
                    &self.voxel_mask,
                    self.nx,
                    self.ny,
                    self.nz,
                );
            }
            if has_freestream {
                self.apply_inlet_outlet_bc();
                self.apply_bounce_back_planes();
            }
        }
        // Sync macroscopic fields so readbacks reflect BC modifications.
        if has_freestream && substeps > 0 {
            self.solver.compute_macroscopic();
        }
    }

    /// Apply equilibrium inlet (x=0) and zero-gradient outlet (x=nx-1).
    ///
    /// Inlet resets distributions to freestream equilibrium, sustaining
    /// the flow. Outlet copies from the adjacent interior slice, letting
    /// wakes exit without acoustic reflections.
    ///
    /// Y and Z indices exclude wall rows (0 and max) to avoid corner
    /// conflicts with bounce-back planes.
    pub fn apply_inlet_outlet_bc(&mut self) {
        let f_eq = BgkCollision::initialize_with_velocity(
            self.freestream_density,
            self.freestream_velocity,
            &self.bounce_back.lattice,
        );

        // Inlet face (x=0): equilibrium at freestream.
        for z in 1..self.nz - 1 {
            for y in 1..self.ny - 1 {
                let idx = z * (self.nx * self.ny) + y * self.nx; // x=0
                let base = idx * 19;
                self.solver.f[base..base + 19].copy_from_slice(&f_eq);
            }
        }

        // Outlet face (x=nx-1): density-corrected Neumann condition.
        // Copy distributions from x=nx-2, then rescale to maintain the
        // nominal freestream density. Without this correction the naive
        // zero-gradient copy accumulates a global mass drift that causes
        // the simulation to diverge after thousands of steps.
        let rho0 = self.freestream_density;
        for z in 1..self.nz - 1 {
            for y in 1..self.ny - 1 {
                let src_idx = z * (self.nx * self.ny) + y * self.nx + (self.nx - 2);
                let dst_idx = z * (self.nx * self.ny) + y * self.nx + (self.nx - 1);
                let src_base = src_idx * 19;
                let dst_base = dst_idx * 19;

                // Compute local density at the source cell.
                let local_rho: f64 = self.solver.f[src_base..src_base + 19].iter().sum();

                // Scale factor: clamp denominator to avoid division by
                // zero if the source cell has degenerate density.
                let scale = if local_rho.abs() > 1e-12 {
                    rho0 / local_rho
                } else {
                    1.0
                };

                // Copy and rescale each distribution.
                for q in 0..19 {
                    self.solver.f[dst_base + q] = self.solver.f[src_base + q] * scale;
                }
            }
        }
    }

    /// Apply bounce-back on Y and Z wall planes (closed wind tunnel).
    ///
    /// MinZ/MaxZ walls prevent the periodic Z-axis from acting as an
    /// open boundary. Applied after inlet/outlet so wall BCs take
    /// precedence at corner intersections.
    pub fn apply_bounce_back_planes(&mut self) {
        for plane in [
            lbm_3d::boundary::BoundaryPlane::MinY,
            lbm_3d::boundary::BoundaryPlane::MaxY,
            lbm_3d::boundary::BoundaryPlane::MinZ,
            lbm_3d::boundary::BoundaryPlane::MaxZ,
        ] {
            self.bounce_back
                .apply_on_plane(&mut self.solver.f, self.nx, self.ny, self.nz, plane);
        }
    }

    /// Compute aerodynamic diagnostics from the velocity and density fields.
    ///
    /// Uses the momentum exchange method: for each solid boundary cell,
    /// sum the momentum transferred from adjacent fluid cells through
    /// the D3Q19 lattice directions.
    pub fn compute_drag_lift(&self, voxels: &VoxelGrid) -> (f64, f64) {
        if let Some(soa) = &self.soa {
            return soa.compute_drag_lift(voxels);
        }
        let mut drag = 0.0; // Force in freestream direction (x).
        let mut lift = 0.0; // Force perpendicular to freestream (y).

        let lattice = lbm_3d::lattice::D3Q19Lattice::new();

        for z in 1..self.nz - 1 {
            for y in 1..self.ny - 1 {
                for x in 1..self.nx - 1 {
                    if !voxels.get(x, y, z) {
                        continue;
                    }
                    // For each lattice direction, check if the neighbor is fluid.
                    for i in 1..19 {
                        let v = lattice.velocity(i);
                        let nbx = x as i32 + v[0];
                        let nby = y as i32 + v[1];
                        let nbz = z as i32 + v[2];
                        if nbx < 0
                            || nby < 0
                            || nbz < 0
                            || nbx >= self.nx as i32
                            || nby >= self.ny as i32
                            || nbz >= self.nz as i32
                        {
                            continue;
                        }
                        let (nbx, nby, nbz) = (nbx as usize, nby as usize, nbz as usize);
                        if voxels.get(nbx, nby, nbz) {
                            continue; // Both solid, skip.
                        }
                        // Fluid neighbor: proper momentum exchange.
                        // F = sum_i (f_i(x_f) + f_opp(x_s)) * c_i
                        // where x_f is the fluid neighbor and x_s is the solid cell.
                        let opp = lattice.opposite_direction(i);
                        // Linearize: z * (nx * ny) + y * nx + x
                        let f_idx = nbz * (self.nx * self.ny) + nby * self.nx + nbx;
                        let s_idx = z * (self.nx * self.ny) + y * self.nx + x;
                        let f_fluid = self.solver.f[f_idx * 19 + i];
                        let f_solid = self.solver.f[s_idx * 19 + opp];
                        let momentum = f_fluid + f_solid;
                        let ci = lattice.velocity(i);
                        drag += momentum * ci[0] as f64;
                        lift += momentum * ci[1] as f64;
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
            use_soa: false,
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

    #[test]
    fn bounce_back_creates_drag() {
        // Verify that flow around a solid obstacle produces nonzero drag.
        let mut engine = LbmCpuEngine::default();
        let entity = Entity::from_bits(1);
        let config = SolverConfig {
            nx: 16,
            ny: 16,
            nz: 16,
            tau: 0.8,
            rho_init: 1.0,
            u_init: [0.05, 0.0, 0.0],
            use_soa: false,
        };
        engine.create_solver(entity, &config);

        // Place a small solid block in the center.
        let mut voxels = VoxelGrid::new(16, 16, 16);
        for x in 6..10 {
            for y in 6..10 {
                for z in 6..10 {
                    voxels.set(x, y, z, true);
                }
            }
        }

        let inst = engine.get_mut(entity).unwrap();
        inst.inject_boundary_from_voxels(&voxels);

        // Run enough steps for the flow to develop.
        inst.evolve_with_boundaries(100);

        let (drag, _lift) = inst.compute_drag_lift(&voxels);
        assert!(
            drag.abs() > 1e-6,
            "Expected nonzero drag from solid obstacle, got {drag}"
        );
    }

    fn freestream_config(nx: usize, ny: usize, nz: usize) -> SolverConfig {
        SolverConfig {
            nx,
            ny,
            nz,
            tau: 0.8,
            rho_init: 1.0,
            u_init: [0.05, 0.0, 0.0],
            use_soa: false,
        }
    }

    #[test]
    fn inlet_outlet_maintains_freestream() {
        // With inlet/outlet BCs the freestream velocity should persist
        // at the domain center rather than decaying to rest.
        let mut engine = LbmCpuEngine::default();
        let entity = Entity::from_bits(10);
        engine.create_solver(entity, &freestream_config(32, 16, 16));

        let inst = engine.get_mut(entity).unwrap();
        inst.evolve_with_boundaries(200);

        let (_, u) = inst.solver.get_macroscopic(16, 8, 8);
        let u_mag = (u[0] * u[0] + u[1] * u[1] + u[2] * u[2]).sqrt();
        assert!(
            u_mag > 0.01,
            "Expected sustained freestream at center, got |u|={u_mag}"
        );
    }

    #[test]
    fn sustained_drag_with_inlet_outlet() {
        // With inlet/outlet BCs and a solid obstacle, drag should
        // remain nonzero after many steps (flow does not decay).
        let mut engine = LbmCpuEngine::default();
        let entity = Entity::from_bits(11);
        engine.create_solver(entity, &freestream_config(32, 16, 16));

        let mut voxels = VoxelGrid::new(32, 16, 16);
        for x in 12..16 {
            for y in 6..10 {
                for z in 6..10 {
                    voxels.set(x, y, z, true);
                }
            }
        }

        let inst = engine.get_mut(entity).unwrap();
        inst.inject_boundary_from_voxels(&voxels);
        inst.evolve_with_boundaries(600);

        let (drag, _lift) = inst.compute_drag_lift(&voxels);
        assert!(
            drag.abs() > 1e-6,
            "Expected sustained nonzero drag with inlet/outlet BCs, got {drag}"
        );
    }

    #[test]
    fn long_running_stability() {
        // Verify the density-corrected outlet prevents mass drift and NaN
        // divergence over thousands of steps. Before the correction, the
        // naive zero-gradient outlet caused simulation blowup at ~2000 steps.
        let mut engine = LbmCpuEngine::default();
        let entity = Entity::from_bits(13);
        engine.create_solver(entity, &freestream_config(32, 16, 16));

        // Place a 4x4x4 solid obstacle in the flow path.
        let mut voxels = VoxelGrid::new(32, 16, 16);
        for x in 12..16 {
            for y in 6..10 {
                for z in 6..10 {
                    voxels.set(x, y, z, true);
                }
            }
        }

        let inst = engine.get_mut(entity).unwrap();
        inst.inject_boundary_from_voxels(&voxels);

        // Run 3000 steps in batches, checking for NaN at each checkpoint.
        for batch in 0..30 {
            inst.evolve_with_boundaries(100);
            let (rho, u) = inst.solver.get_macroscopic(16, 8, 8);
            assert!(
                rho.is_finite() && u[0].is_finite() && u[1].is_finite() && u[2].is_finite(),
                "Simulation diverged at step {} (batch {batch}): rho={rho}, u={u:?}",
                (batch + 1) * 100
            );
        }

        // After 3000 steps the freestream should still be sustained.
        // Probe upstream of the obstacle (x=4) where blockage effects
        // are minimal. The wake behind the obstacle (x>=16) can have
        // low velocity on this small domain.
        let (rho, u) = inst.solver.get_macroscopic(4, 8, 8);
        let u_mag = (u[0] * u[0] + u[1] * u[1] + u[2] * u[2]).sqrt();
        assert!(
            u_mag > 0.005,
            "Expected sustained flow after 3000 steps, got |u|={u_mag}, rho={rho}"
        );

        // Drag should be nonzero (flow is not stagnant).
        let (drag, _lift) = inst.compute_drag_lift(&voxels);
        assert!(
            drag.abs() > 1e-6,
            "Expected nonzero drag after 3000 steps, got {drag}"
        );
    }

    #[test]
    fn outlet_density_corrected() {
        // The density-corrected Neumann outlet rescales copied distributions
        // so the outlet face density equals freestream_density. Verify this
        // invariant holds after evolution.
        let mut engine = LbmCpuEngine::default();
        let entity = Entity::from_bits(12);
        engine.create_solver(entity, &freestream_config(16, 8, 8));

        let inst = engine.get_mut(entity).unwrap();
        let rho0 = inst.freestream_density;
        inst.evolve_with_boundaries(50);

        let nx = inst.nx;
        let ny = inst.ny;
        let nz = inst.nz;
        // Check that outlet face density matches freestream density.
        for z in 1..nz - 1 {
            for y in 1..ny - 1 {
                let dst_idx = z * (nx * ny) + y * nx + (nx - 1);
                let dst_base = dst_idx * 19;
                let outlet_rho: f64 = inst.solver.f[dst_base..dst_base + 19].iter().sum();
                assert!(
                    (outlet_rho - rho0).abs() < 1e-10,
                    "Outlet density at y={y},z={z}: {outlet_rho}, expected {rho0}"
                );
            }
        }
    }
}
