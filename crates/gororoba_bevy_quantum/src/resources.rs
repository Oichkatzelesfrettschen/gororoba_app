// TensorNetworkEngine and CasimirSolver resources.
//
// Manages MERA tensor network computations and Casimir energy
// calculations as Bevy resources.

use bevy::prelude::*;

use casimir_core::energy::{CasimirEnergyResult, WorldlineCasimirConfig, casimir_energy_at_point};
use casimir_core::geometry::ParallelPlates;
use quantum_core::mera::{MeraLayer, build_mera_structure, mera_entropy_estimate};

use crate::components::PlateGeometry;

/// CPU-based quantum simulation engine.
///
/// Wraps quantum_core for MERA tensor networks and casimir_core for
/// Casimir energy computations.
#[derive(Resource, Default)]
pub struct QuantumEngine {
    /// Active instances keyed by entity ID.
    pub instances: Vec<(Entity, QuantumInstance)>,
}

/// Per-entity quantum simulation state.
pub struct QuantumInstance {
    /// MERA network structure.
    pub mera_layers: Vec<MeraLayer>,
    /// Lattice size.
    pub n_sites: usize,
    /// Local Hilbert space dimension.
    pub local_dim: usize,
    /// Cached entanglement entropy.
    pub entropy: f64,
    /// Cached Casimir result.
    pub casimir_result: Option<CasimirEnergyResult>,
}

/// Configuration for creating a quantum instance.
pub struct QuantumConfig {
    pub n_sites: usize,
    pub local_dim: usize,
    pub seed: u64,
}

impl QuantumEngine {
    /// Create a new quantum instance for the given entity.
    pub fn create_instance(&mut self, entity: Entity, config: &QuantumConfig) {
        self.instances.retain(|(e, _)| *e != entity);

        let mera_layers = build_mera_structure(config.n_sites);

        self.instances.push((
            entity,
            QuantumInstance {
                mera_layers,
                n_sites: config.n_sites,
                local_dim: config.local_dim,
                entropy: 0.0,
                casimir_result: None,
            },
        ));
    }

    /// Get mutable reference to an instance by entity.
    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut QuantumInstance> {
        self.instances
            .iter_mut()
            .find(|(e, _)| *e == entity)
            .map(|(_, inst)| inst)
    }

    /// Get reference to an instance by entity.
    pub fn get(&self, entity: Entity) -> Option<&QuantumInstance> {
        self.instances
            .iter()
            .find(|(e, _)| *e == entity)
            .map(|(_, inst)| inst)
    }

    /// Remove instance for entity.
    pub fn remove(&mut self, entity: Entity) {
        self.instances.retain(|(e, _)| *e != entity);
    }
}

impl QuantumInstance {
    /// Estimate entanglement entropy using MERA.
    pub fn estimate_entropy(&mut self, subsystem_size: usize, seed: u64) {
        self.entropy = mera_entropy_estimate(subsystem_size, self.local_dim, seed);
    }

    /// Compute Casimir energy at a point for parallel plates.
    pub fn compute_casimir_parallel_plates(
        &mut self,
        separation: f64,
        point: [f64; 3],
        config: &WorldlineCasimirConfig,
    ) {
        let geometry = ParallelPlates { separation };
        self.casimir_result = Some(casimir_energy_at_point(&geometry, point, config));
    }

    /// Compute Casimir energy at a point for the given geometry.
    pub fn compute_casimir(
        &mut self,
        plate_geometry: &PlateGeometry,
        point: [f64; 3],
        config: &WorldlineCasimirConfig,
    ) {
        match plate_geometry {
            PlateGeometry::ParallelPlates { separation } => {
                let geometry = ParallelPlates {
                    separation: *separation,
                };
                self.casimir_result = Some(casimir_energy_at_point(&geometry, point, config));
            }
            PlateGeometry::SpherePlateSphere { .. } => {
                // SpherePlateSphere geometry requires casimir_core::SpherePlateSphere
                // which has a different constructor. Defer to future implementation.
            }
        }
    }

    /// Number of MERA layers.
    pub fn layer_count(&self) -> usize {
        self.mera_layers.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> QuantumConfig {
        QuantumConfig {
            n_sites: 16,
            local_dim: 2,
            seed: 42,
        }
    }

    #[test]
    fn engine_create_and_lookup() {
        let mut engine = QuantumEngine::default();
        let entity = Entity::from_bits(1);
        engine.create_instance(entity, &test_config());

        assert!(engine.get(entity).is_some());
        assert!(engine.get(Entity::from_bits(99)).is_none());

        let inst = engine.get(entity).unwrap();
        assert_eq!(inst.n_sites, 16);
    }

    #[test]
    fn mera_structure_built() {
        let mut engine = QuantumEngine::default();
        let entity = Entity::from_bits(1);
        engine.create_instance(entity, &test_config());

        let inst = engine.get(entity).unwrap();
        // 16 sites = 4 MERA layers (log2(16) = 4)
        assert!(inst.layer_count() > 0);
    }

    #[test]
    fn entropy_estimate() {
        let mut engine = QuantumEngine::default();
        let entity = Entity::from_bits(1);
        engine.create_instance(entity, &test_config());

        let inst = engine.get_mut(entity).unwrap();
        inst.estimate_entropy(4, 42);
        // Entropy should be non-negative
        assert!(inst.entropy >= 0.0);
    }

    #[test]
    fn engine_remove() {
        let mut engine = QuantumEngine::default();
        let entity = Entity::from_bits(1);
        engine.create_instance(entity, &test_config());
        engine.remove(entity);
        assert!(engine.get(entity).is_none());
    }

    #[test]
    fn casimir_parallel_plates() {
        let mut engine = QuantumEngine::default();
        let entity = Entity::from_bits(1);
        engine.create_instance(entity, &test_config());

        let inst = engine.get_mut(entity).unwrap();
        let config = WorldlineCasimirConfig::default();
        inst.compute_casimir_parallel_plates(1.0, [0.0, 0.5, 0.0], &config);

        assert!(inst.casimir_result.is_some());
        let result = inst.casimir_result.as_ref().unwrap();
        // Casimir energy should be negative between plates
        assert!(
            result.energy < 0.0,
            "Casimir energy should be negative, got {}",
            result.energy
        );
    }
}
