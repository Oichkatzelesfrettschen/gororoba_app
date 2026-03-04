// MERA step, Casimir energy, and measurement systems.
//
// quantum_init_system initializes quantum instances for new domains.
// mera_step_system runs MERA entropy estimation (FixedUpdate).
// casimir_system computes Casimir energies (FixedUpdate).
// diagnostics_system updates diagnostic components (Update).

use bevy::prelude::*;

use casimir_core::energy::WorldlineCasimirConfig;

use crate::components::{
    CasimirFieldConfig, CasimirParams, CasimirPlate, PlateGeometry, QuantumDiagnostics,
    QuantumDomain, QuantumParams, SpinLattice,
};
use crate::resources::{QuantumConfig, QuantumEngine};

/// Initialize quantum instances for newly-spawned QuantumDomain entities.
#[allow(clippy::type_complexity)]
pub fn quantum_init_system(
    mut engine: ResMut<QuantumEngine>,
    query: Query<(Entity, &SpinLattice), (With<QuantumDomain>, Added<QuantumDomain>)>,
) {
    for (entity, lattice) in &query {
        let config = QuantumConfig {
            n_sites: lattice.n_sites,
            local_dim: lattice.local_dim,
            seed: lattice.seed,
        };
        engine.create_instance(entity, &config);
    }
}

/// Run MERA entropy estimation.
///
/// Updates the cached entropy value in the quantum instance.
/// Runs in FixedUpdate for deterministic results.
pub fn mera_step_system(
    mut engine: ResMut<QuantumEngine>,
    query: Query<(Entity, &SpinLattice, &QuantumParams), With<QuantumDomain>>,
) {
    for (entity, lattice, params) in &query {
        if let Some(inst) = engine.get_mut(entity) {
            inst.estimate_entropy(params.subsystem_size, lattice.seed);
        }
    }
}

/// Compute Casimir energies for plate configurations.
///
/// Evaluates the Casimir energy at the plate position using the
/// worldline Monte Carlo method. Runs in FixedUpdate.
pub fn casimir_system(
    mut engine: ResMut<QuantumEngine>,
    query: Query<(Entity, &CasimirPlate, &CasimirParams), With<QuantumDomain>>,
) {
    for (entity, plate, params) in &query {
        if let Some(inst) = engine.get_mut(entity) {
            let config = WorldlineCasimirConfig {
                n_loop_points: params.n_loop_points,
                n_loops: params.n_loops,
                t_min: params.t_min,
                t_max: params.t_max,
                n_t_points: params.n_t_points,
                seed: params.seed,
            };
            inst.compute_casimir(&plate.geometry, plate.position, &config);
        }
    }
}

/// Compute 3D Casimir energy density field when CasimirFieldConfig is present.
///
/// Only recomputes when `dirty` is true, to avoid expensive Monte Carlo
/// computation every frame. Runs in FixedUpdate after casimir_system.
pub fn casimir_field_system(
    mut engine: ResMut<QuantumEngine>,
    mut query: Query<
        (
            Entity,
            &CasimirPlate,
            &CasimirParams,
            &mut CasimirFieldConfig,
        ),
        With<QuantumDomain>,
    >,
) {
    for (entity, plate, params, mut field_cfg) in &mut query {
        if !field_cfg.dirty {
            continue;
        }

        let PlateGeometry::ParallelPlates { separation } = plate.geometry else {
            continue;
        };

        if let Some(inst) = engine.get_mut(entity) {
            let config = WorldlineCasimirConfig {
                n_loop_points: params.n_loop_points,
                n_loops: params.n_loops,
                t_min: params.t_min,
                t_max: params.t_max,
                n_t_points: params.n_t_points,
                seed: params.seed,
            };
            inst.compute_casimir_field_3d_parallel_plates(
                separation,
                field_cfg.bounds,
                field_cfg.resolution,
                &config,
            );
            field_cfg.dirty = false;
        }
    }
}

/// Update diagnostic components from quantum engine state.
pub fn diagnostics_system(
    engine: Res<QuantumEngine>,
    mut query: Query<(Entity, &mut QuantumDiagnostics), With<QuantumDomain>>,
) {
    for (entity, mut diag) in &mut query {
        if let Some(inst) = engine.get(entity) {
            diag.entanglement_entropy = inst.entropy;
            diag.mera_layers = inst.layer_count();

            if let Some(ref result) = inst.casimir_result {
                diag.casimir_energy = result.energy;
                diag.casimir_error = result.error;
            }
        }
    }
}

/// Clean up quantum instances when QuantumDomain entities are despawned.
pub fn quantum_cleanup_system(
    mut engine: ResMut<QuantumEngine>,
    mut removals: RemovedComponents<QuantumDomain>,
) {
    for entity in removals.read() {
        engine.remove(entity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::QuantumConfig;

    #[test]
    fn quantum_init_creates_instance() {
        let mut engine = QuantumEngine::default();
        let entity = Entity::from_bits(1);
        let config = QuantumConfig {
            n_sites: 16,
            local_dim: 2,
            seed: 42,
        };
        engine.create_instance(entity, &config);

        let inst = engine.get(entity).unwrap();
        assert_eq!(inst.n_sites, 16);
        assert!(inst.layer_count() > 0);
    }

    #[test]
    fn casimir_field_3d_via_instance() {
        let mut engine = QuantumEngine::default();
        let entity = Entity::from_bits(1);
        let config = QuantumConfig {
            n_sites: 8,
            local_dim: 2,
            seed: 42,
        };
        engine.create_instance(entity, &config);

        let inst = engine.get_mut(entity).unwrap();
        let mc_config = WorldlineCasimirConfig {
            n_loop_points: 16,
            n_loops: 50,
            t_min: 0.01,
            t_max: 3.0,
            n_t_points: 4,
            seed: 42,
        };
        inst.compute_casimir_field_3d_parallel_plates(
            1.0,
            (-1.0, 1.0, 0.0, 1.0, -1.0, 1.0),
            (2, 2, 2),
            &mc_config,
        );

        let field = inst.casimir_field_3d.as_ref().unwrap();
        assert_eq!(field.data.len(), 8);
        assert!(field.data.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn mera_entropy_nonnegative() {
        let mut engine = QuantumEngine::default();
        let entity = Entity::from_bits(1);
        let config = QuantumConfig {
            n_sites: 16,
            local_dim: 2,
            seed: 42,
        };
        engine.create_instance(entity, &config);

        let inst = engine.get_mut(entity).unwrap();
        inst.estimate_entropy(4, 42);
        assert!(inst.entropy >= 0.0);
    }
}
