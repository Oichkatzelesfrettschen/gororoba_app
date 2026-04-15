// MERA step, Casimir energy, and measurement systems backed by the local quantum kernel.

use bevy::prelude::*;
use gororoba_kernel_api::quantum::{
    CasimirFieldRequest, CasimirWorldlineConfig, QuantumKernel, SpinLatticeConfig,
};

use crate::components::{
    CasimirFieldConfig, CasimirParams, CasimirPlate, PlateGeometry, QuantumDiagnostics,
    QuantumDomain, QuantumParams, SpinLattice,
};
use crate::resources::{QuantumConfig, QuantumEngine};

#[allow(clippy::type_complexity)]
pub fn quantum_init_system(
    mut engine: ResMut<QuantumEngine>,
    query: Query<
        (Entity, &SpinLattice, &QuantumParams),
        (With<QuantumDomain>, Added<QuantumDomain>),
    >,
) {
    for (entity, lattice, params) in &query {
        let config = QuantumConfig {
            lattice: SpinLatticeConfig {
                n_sites: lattice.n_sites,
                local_dim: lattice.local_dim,
                seed: lattice.seed,
            },
            subsystem_size: params.subsystem_size,
        };
        engine.create_instance(entity, &config);
    }
}

pub fn mera_step_system(
    mut engine: ResMut<QuantumEngine>,
    query: Query<(Entity, &QuantumParams), With<QuantumDomain>>,
) {
    for (entity, params) in &query {
        if let Some(inst) = engine.get_mut(entity) {
            inst.estimate_entropy(params.subsystem_size);
        }
    }
}

pub fn casimir_system(
    mut engine: ResMut<QuantumEngine>,
    query: Query<(Entity, &CasimirPlate, &CasimirParams), With<QuantumDomain>>,
) {
    for (entity, plate, params) in &query {
        if let Some(inst) = engine.get_mut(entity) {
            let config = CasimirWorldlineConfig {
                n_loop_points: params.n_loop_points,
                n_loops: params.n_loops,
                t_min: params.t_min,
                t_max: params.t_max,
                n_t_points: params.n_t_points,
                seed: params.seed,
            };
            inst.compute_casimir(plate.geometry, plate.position, &config);
        }
    }
}

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

        let PlateGeometry::ParallelPlates { .. } = plate.geometry else {
            continue;
        };

        if let Some(inst) = engine.get_mut(entity) {
            let config = CasimirWorldlineConfig {
                n_loop_points: params.n_loop_points,
                n_loops: params.n_loops,
                t_min: params.t_min,
                t_max: params.t_max,
                n_t_points: params.n_t_points,
                seed: params.seed,
            };
            inst.compute_casimir_field(
                plate.geometry,
                &CasimirFieldRequest {
                    resolution: field_cfg.resolution,
                    bounds: field_cfg.bounds,
                },
                &config,
            );
            field_cfg.dirty = false;
        }
    }
}

pub fn diagnostics_system(
    engine: Res<QuantumEngine>,
    mut query: Query<(Entity, &mut QuantumDiagnostics), With<QuantumDomain>>,
) {
    for (entity, mut diag) in &mut query {
        if let Some(inst) = engine.get(entity) {
            let kernel_diag = inst.kernel.diagnostics();
            diag.entanglement_entropy = kernel_diag.entanglement_entropy;
            diag.mera_layers = kernel_diag.mera_layers;
            diag.casimir_energy = kernel_diag.casimir_energy;
            diag.casimir_error = kernel_diag.casimir_error;
        }
    }
}

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
    use gororoba_kernel_api::quantum::CasimirGeometry;

    #[test]
    fn quantum_init_creates_instance() {
        let mut engine = QuantumEngine::default();
        let entity = Entity::from_bits(1);
        let config = QuantumConfig {
            lattice: SpinLatticeConfig {
                n_sites: 16,
                local_dim: 2,
                seed: 42,
            },
            subsystem_size: 4,
        };
        engine.create_instance(entity, &config);

        let inst = engine.get(entity).unwrap();
        assert_eq!(inst.kernel.config.lattice.n_sites, 16);
        assert!(inst.layer_count() > 0);
    }

    #[test]
    fn casimir_field_snapshot_is_cached_for_gameplay() {
        let mut engine = QuantumEngine::default();
        let entity = Entity::from_bits(1);
        let config = QuantumConfig {
            lattice: SpinLatticeConfig {
                n_sites: 8,
                local_dim: 2,
                seed: 42,
            },
            subsystem_size: 4,
        };
        engine.create_instance(entity, &config);

        let inst = engine.get_mut(entity).unwrap();
        inst.compute_casimir_field(
            CasimirGeometry::ParallelPlates { separation: 1.0 },
            &CasimirFieldRequest {
                resolution: (2, 2, 2),
                bounds: (-1.0, 1.0, 0.0, 1.0, -1.0, 1.0),
            },
            &CasimirWorldlineConfig {
                n_loop_points: 16,
                n_loops: 50,
                t_min: 0.01,
                t_max: 3.0,
                n_t_points: 4,
                seed: 42,
            },
        );

        let field = inst.casimir_field_3d.as_ref().unwrap();
        assert_eq!(field.data.len(), 8);
    }
}
