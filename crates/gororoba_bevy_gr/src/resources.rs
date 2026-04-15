// GR engine resource wrapping gororoba_app's local relativity kernel.
//
// Manages spacetime metric computations, geodesic integration,
// and cached shadow boundary data as a Bevy resource.

use bevy::prelude::*;
use gororoba_kernel_api::relativity::{RelativityDomainConfig, RelativityKernel};
use gororoba_kernel_gr::RelativityCpuKernel;

#[derive(Resource, Default)]
pub struct GrEngine {
    pub instances: Vec<(Entity, GrInstance)>,
}

pub struct GrInstance {
    pub kernel: RelativityCpuKernel,
    pub shadow_alpha: Vec<f64>,
    pub shadow_beta: Vec<f64>,
}

pub type GrConfig = RelativityDomainConfig;

impl GrEngine {
    pub fn create_instance(&mut self, entity: Entity, config: &GrConfig) {
        self.instances.retain(|(current, _)| *current != entity);
        self.instances.push((
            entity,
            GrInstance {
                kernel: RelativityCpuKernel::new(*config),
                shadow_alpha: Vec::new(),
                shadow_beta: Vec::new(),
            },
        ));
    }

    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut GrInstance> {
        self.instances
            .iter_mut()
            .find(|(current, _)| *current == entity)
            .map(|(_, instance)| instance)
    }

    pub fn get(&self, entity: Entity) -> Option<&GrInstance> {
        self.instances
            .iter()
            .find(|(current, _)| *current == entity)
            .map(|(_, instance)| instance)
    }

    pub fn remove(&mut self, entity: Entity) {
        self.instances.retain(|(current, _)| *current != entity);
    }
}

impl GrInstance {
    pub fn compute_shadow(&mut self) {
        let shadow = self.kernel.compute_shadow();
        self.shadow_alpha = shadow.alpha;
        self.shadow_beta = shadow.beta;
    }

    pub fn time_dilation_factor(&self, radius: f64) -> f64 {
        self.kernel.time_dilation_factor(radius)
    }

    pub fn event_horizon(&self) -> f64 {
        self.kernel.event_horizon()
    }

    pub fn isco_radius(&self) -> f64 {
        self.kernel.isco_radius()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::MetricType;

    fn schwarzschild_config() -> GrConfig {
        GrConfig {
            mass: 1.0,
            metric: MetricType::Schwarzschild,
            observer_inclination: std::f64::consts::FRAC_PI_2,
            observer_distance: 50.0,
            shadow_points: 64,
        }
    }

    #[test]
    fn engine_create_and_lookup() {
        let mut engine = GrEngine::default();
        let entity = Entity::from_bits(1);
        engine.create_instance(entity, &schwarzschild_config());

        assert!(engine.get(entity).is_some());
        assert!(engine.get(Entity::from_bits(99)).is_none());
    }

    #[test]
    fn invariants_delegate_to_local_kernel() {
        let mut engine = GrEngine::default();
        let entity = Entity::from_bits(1);
        engine.create_instance(entity, &schwarzschild_config());

        let inst = engine.get(entity).unwrap();
        assert!((inst.event_horizon() - 2.0).abs() < 1e-10);
        assert!((inst.isco_radius() - 6.0).abs() < 1e-10);
    }

    #[test]
    fn shadow_is_cached_for_bevy_rendering() {
        let mut engine = GrEngine::default();
        let entity = Entity::from_bits(1);
        let config = schwarzschild_config();
        engine.create_instance(entity, &config);

        let inst = engine.get_mut(entity).unwrap();
        inst.compute_shadow();
        assert_eq!(inst.shadow_alpha.len(), config.shadow_points * 2);
        assert_eq!(inst.shadow_alpha.len(), inst.shadow_beta.len());
    }
}
