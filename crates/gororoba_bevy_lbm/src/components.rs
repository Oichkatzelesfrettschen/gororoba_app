// VoxelGrid, FluidCell, BoundaryCondition components.
//
// These define the ECS data model for LBM simulations in Bevy.

use bevy::prelude::*;

/// A 3D voxel grid used for hull/obstacle definition.
///
/// Each cell is either solid (true) or fluid (false). The grid maps to
/// the LBM simulation domain: solid voxels become bounce-back boundary
/// conditions in the solver.
#[derive(Component)]
pub struct VoxelGrid {
    pub nx: usize,
    pub ny: usize,
    pub nz: usize,
    /// Flattened [nx * ny * nz] array, row-major (x varies fastest).
    /// true = solid obstacle, false = fluid.
    pub cells: Vec<bool>,
}

impl VoxelGrid {
    pub fn new(nx: usize, ny: usize, nz: usize) -> Self {
        Self {
            nx,
            ny,
            nz,
            cells: vec![false; nx * ny * nz],
        }
    }

    fn index(&self, x: usize, y: usize, z: usize) -> usize {
        z * self.ny * self.nx + y * self.nx + x
    }

    pub fn set(&mut self, x: usize, y: usize, z: usize, solid: bool) {
        let idx = self.index(x, y, z);
        self.cells[idx] = solid;
    }

    pub fn get(&self, x: usize, y: usize, z: usize) -> bool {
        self.cells[self.index(x, y, z)]
    }

    /// Count of solid cells.
    pub fn solid_count(&self) -> usize {
        self.cells.iter().filter(|&&c| c).count()
    }
}

/// Marker component for entities that represent a fluid simulation domain.
#[derive(Component, Default)]
pub struct FluidDomain;

/// Boundary condition type for a simulation face.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundaryType {
    /// Periodic wrap-around (default for all faces).
    Periodic,
    /// No-slip bounce-back wall.
    BounceBack,
    /// Zou-He velocity inlet/outlet.
    ZouHe,
}

/// Configures boundary conditions for the six faces of the simulation domain.
#[derive(Component)]
pub struct BoundaryConditions {
    pub x_neg: BoundaryType,
    pub x_pos: BoundaryType,
    pub y_neg: BoundaryType,
    pub y_pos: BoundaryType,
    pub z_neg: BoundaryType,
    pub z_pos: BoundaryType,
}

impl Default for BoundaryConditions {
    fn default() -> Self {
        Self {
            x_neg: BoundaryType::Periodic,
            x_pos: BoundaryType::Periodic,
            y_neg: BoundaryType::BounceBack,
            y_pos: BoundaryType::BounceBack,
            z_neg: BoundaryType::Periodic,
            z_pos: BoundaryType::Periodic,
        }
    }
}

/// Per-domain simulation parameters.
#[derive(Component)]
pub struct SimulationParams {
    /// Relaxation time (controls viscosity). Must be > 0.5 for stability.
    pub tau: f64,
    /// Initial uniform density.
    pub rho_init: f64,
    /// Initial uniform velocity [ux, uy, uz].
    pub u_init: [f64; 3],
    /// External body force (e.g. gravity, pressure gradient).
    pub force: [f64; 3],
    /// Steps per FixedUpdate tick. Higher = faster simulation, more CPU.
    pub substeps: usize,
    /// Use high-performance SoA f32 solver (perturbation formulation).
    pub use_soa: bool,
}

impl Default for SimulationParams {
    fn default() -> Self {
        Self {
            tau: 0.8,
            rho_init: 1.0,
            u_init: [0.0, 0.0, 0.0],
            force: [0.0, 0.0, 0.0],
            substeps: 1,
            use_soa: false,
        }
    }
}

/// Diagnostic output from the solver, updated each simulation step.
#[derive(Component, Default)]
pub struct SimulationDiagnostics {
    pub timestep: usize,
    pub total_mass: f64,
    pub max_velocity: f64,
    pub mean_velocity: f64,
    pub stable: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voxel_grid_set_get() {
        let mut grid = VoxelGrid::new(4, 4, 4);
        assert!(!grid.get(1, 2, 3));
        grid.set(1, 2, 3, true);
        assert!(grid.get(1, 2, 3));
        assert_eq!(grid.solid_count(), 1);
    }

    #[test]
    fn boundary_conditions_default() {
        let bc = BoundaryConditions::default();
        assert_eq!(bc.y_neg, BoundaryType::BounceBack);
        assert_eq!(bc.x_neg, BoundaryType::Periodic);
    }

    #[test]
    fn simulation_params_default_stable() {
        let params = SimulationParams::default();
        assert!(params.tau > 0.5, "tau must be > 0.5 for LBM stability");
    }
}
