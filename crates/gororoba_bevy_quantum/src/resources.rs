// Tensor-network and Casimir resources backed by the local quantum kernel.

use bevy::prelude::*;
use gororoba_kernel_api::quantum::{
    CasimirFieldSnapshot, MeraLayerSnapshot, QuantumDomainConfig, QuantumKernel,
};
use gororoba_kernel_quantum::QuantumCpuKernel;

#[derive(Resource, Default)]
pub struct QuantumEngine {
    pub instances: Vec<(Entity, QuantumInstance)>,
}

pub struct QuantumInstance {
    pub kernel: QuantumCpuKernel,
    pub mera_layers: Vec<MeraLayerSnapshot>,
    pub entropy: f64,
    pub casimir_energy: Option<f64>,
    pub casimir_error: Option<f64>,
    pub casimir_field_3d: Option<CasimirFieldSnapshot>,
}

pub type QuantumConfig = QuantumDomainConfig;

impl QuantumEngine {
    pub fn create_instance(&mut self, entity: Entity, config: &QuantumConfig) {
        self.instances.retain(|(current, _)| *current != entity);
        self.instances.push((entity, QuantumInstance::new(config)));
    }

    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut QuantumInstance> {
        self.instances
            .iter_mut()
            .find(|(current, _)| *current == entity)
            .map(|(_, instance)| instance)
    }

    pub fn get(&self, entity: Entity) -> Option<&QuantumInstance> {
        self.instances
            .iter()
            .find(|(current, _)| *current == entity)
            .map(|(_, instance)| instance)
    }

    pub fn remove(&mut self, entity: Entity) {
        self.instances.retain(|(current, _)| *current != entity);
    }
}

impl QuantumInstance {
    pub fn new(config: &QuantumConfig) -> Self {
        let kernel = QuantumCpuKernel::new(*config);
        Self {
            mera_layers: kernel.mera_layers.clone(),
            entropy: kernel.entropy,
            casimir_energy: None,
            casimir_error: None,
            casimir_field_3d: None,
            kernel,
        }
    }

    pub fn estimate_entropy(&mut self, subsystem_size: usize) {
        self.entropy = self.kernel.estimate_entropy(subsystem_size);
    }

    pub fn compute_casimir(
        &mut self,
        geometry: gororoba_kernel_api::quantum::CasimirGeometry,
        position: [f64; 3],
        config: &gororoba_kernel_api::quantum::CasimirWorldlineConfig,
    ) {
        let sample = self.kernel.casimir_at_point(geometry, position, config);
        self.casimir_energy = Some(sample.energy);
        self.casimir_error = Some(sample.error);
    }

    pub fn compute_casimir_field(
        &mut self,
        geometry: gororoba_kernel_api::quantum::CasimirGeometry,
        request: &gororoba_kernel_api::quantum::CasimirFieldRequest,
        config: &gororoba_kernel_api::quantum::CasimirWorldlineConfig,
    ) {
        self.casimir_field_3d = self.kernel.casimir_field(geometry, request, config);
    }

    pub fn layer_count(&self) -> usize {
        self.mera_layers.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gororoba_kernel_api::quantum::SpinLatticeConfig;

    fn test_config() -> QuantumConfig {
        QuantumConfig {
            lattice: SpinLatticeConfig {
                n_sites: 16,
                local_dim: 2,
                seed: 42,
            },
            subsystem_size: 4,
        }
    }

    #[test]
    fn engine_create_and_lookup() {
        let mut engine = QuantumEngine::default();
        let entity = Entity::from_bits(1);
        engine.create_instance(entity, &test_config());
        assert!(engine.get(entity).is_some());
        assert!(engine.get(Entity::from_bits(99)).is_none());
    }

    #[test]
    fn instance_exposes_local_kernel_state_for_games() {
        let mut engine = QuantumEngine::default();
        let entity = Entity::from_bits(1);
        engine.create_instance(entity, &test_config());

        let inst = engine.get(entity).unwrap();
        assert_eq!(inst.layer_count(), inst.mera_layers.len());
    }
}
