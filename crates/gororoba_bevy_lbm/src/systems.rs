// Simulation step, readback, and mesh update systems.
//
// simulation_step_system runs in FixedUpdate (deterministic physics).
// diagnostics_system runs in Update (reporting to HUD).

use bevy::prelude::*;

use crate::components::{FluidDomain, SimulationDiagnostics, SimulationParams, VoxelGrid};
use crate::resources::{FluidSimulationEngine, SolverConfig};

/// Initialize solvers for newly-spawned FluidDomain entities.
///
/// When a FluidDomain entity is added with SimulationParams, this system
/// creates the corresponding LbmSolver3D in the CPU engine.
#[allow(clippy::type_complexity)]
pub fn solver_init_system(
    mut engine: ResMut<FluidSimulationEngine>,
    query: Query<(Entity, &VoxelGrid, &SimulationParams), (With<FluidDomain>, Added<FluidDomain>)>,
) {
    for (entity, voxels, params) in &query {
        let config = SolverConfig {
            nx: voxels.nx,
            ny: voxels.ny,
            nz: voxels.nz,
            tau: params.tau,
            rho_init: params.rho_init,
            u_init: params.u_init,
            force: params.force,
            substeps: params.substeps,
            execution: params.execution,
        };
        if let Err(error) = engine.create_solver(entity, &config) {
            warn!("Failed to create fluid solver for {entity:?}: {error}");
            continue;
        }

        // Inject voxel boundaries after creation.
        if let Some(inst) = engine.get_mut(entity) {
            inst.inject_boundary_from_voxels(voxels);
        }
    }
}

/// Advance the LBM simulation by the configured number of substeps.
///
/// Uses `evolve_with_boundaries()` which applies bounce-back boundary
/// conditions after every streaming step, enforcing no-slip walls at
/// solid cells. Runs in FixedUpdate to decouple physics from framerate.
pub fn simulation_step_system(
    mut engine: ResMut<FluidSimulationEngine>,
    query: Query<(Entity, &SimulationParams), With<FluidDomain>>,
) {
    for (entity, params) in &query {
        if let Some(inst) = engine.get_mut(entity) {
            inst.evolve_with_boundaries(params.substeps);
        }
    }
}

/// Update diagnostic components from solver state.
///
/// Runs in Update for HUD display.
pub fn diagnostics_system(
    engine: Res<FluidSimulationEngine>,
    mut query: Query<(Entity, &mut SimulationDiagnostics), With<FluidDomain>>,
) {
    for (entity, mut diag) in &mut query {
        if let Some(inst) = engine.get(entity) {
            let snapshot = inst.diagnostics();
            diag.timestep = snapshot.timestep;
            diag.total_mass = snapshot.total_mass;
            diag.max_velocity = snapshot.max_velocity;
            diag.mean_velocity = snapshot.mean_velocity;
            diag.stable = snapshot.stable;
        }
    }
}

/// Re-inject voxel boundaries when VoxelGrid changes.
#[allow(clippy::type_complexity)]
pub fn boundary_update_system(
    mut engine: ResMut<FluidSimulationEngine>,
    query: Query<(Entity, &VoxelGrid), (With<FluidDomain>, Changed<VoxelGrid>)>,
) {
    for (entity, voxels) in &query {
        if let Some(inst) = engine.get_mut(entity) {
            inst.inject_boundary_from_voxels(voxels);
        }
    }
}

/// Clean up solvers when FluidDomain entities are despawned.
pub fn solver_cleanup_system(
    mut engine: ResMut<FluidSimulationEngine>,
    mut removals: RemovedComponents<FluidDomain>,
) {
    for entity in removals.read() {
        engine.remove(entity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::SolverConfig;

    fn test_config(nx: usize, u_init: [f64; 3]) -> SolverConfig {
        SolverConfig {
            nx,
            ny: nx,
            nz: nx,
            tau: 0.8,
            rho_init: 1.0,
            u_init,
            force: [0.0; 3],
            substeps: 1,
            execution: Default::default(),
        }
    }

    #[test]
    fn simulation_step_evolves() {
        let mut engine = FluidSimulationEngine::default();
        let entity = Entity::from_bits(1);
        engine
            .create_solver(entity, &test_config(8, [0.01, 0.0, 0.0]))
            .unwrap();

        let inst = engine.get_mut(entity).unwrap();
        let t0 = inst.diagnostics().timestep;
        inst.evolve_with_boundaries(5);
        assert_eq!(inst.diagnostics().timestep, t0 + 5);
    }

    #[test]
    fn diagnostics_from_solver() {
        let mut engine = FluidSimulationEngine::default();
        let entity = Entity::from_bits(1);
        engine
            .create_solver(entity, &test_config(8, [0.0; 3]))
            .unwrap();

        let inst = engine.get(entity).unwrap();
        let diagnostics = inst.diagnostics();
        assert!(diagnostics.stable);
        assert!(diagnostics.total_mass > 0.0);
    }

    #[test]
    fn voxel_boundary_injection() {
        let mut engine = FluidSimulationEngine::default();
        let entity = Entity::from_bits(1);
        engine
            .create_solver(entity, &test_config(8, [0.0; 3]))
            .unwrap();

        let mut voxels = VoxelGrid::new(8, 8, 8);
        // Place a solid block in the middle.
        for x in 3..5 {
            for y in 3..5 {
                for z in 3..5 {
                    voxels.set(x, y, z, true);
                }
            }
        }

        let inst = engine.get_mut(entity).unwrap();
        inst.inject_boundary_from_voxels(&voxels);
        // Should not panic; viscosity field should be set.
    }
}
