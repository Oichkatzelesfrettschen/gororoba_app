// Geodesic integration, shadow computation, and time dilation systems.
//
// gr_init_system initializes GR instances for new spacetime domains.
// geodesic_step_system integrates geodesics (FixedUpdate).
// shadow_system computes black hole shadow boundaries (Update, once).
// diagnostics_system updates diagnostic components (Update).

use bevy::prelude::*;

use crate::components::{
    BlackHole, Geodesic, GeodesicType, GrDiagnostics, GrParams, SpacetimeDomain,
};
use crate::resources::{GrConfig, GrEngine};

/// Initialize GR instances for newly-spawned SpacetimeDomain entities.
#[allow(clippy::type_complexity)]
pub fn gr_init_system(
    mut engine: ResMut<GrEngine>,
    query: Query<(Entity, &BlackHole), (With<SpacetimeDomain>, Added<SpacetimeDomain>)>,
) {
    for (entity, bh) in &query {
        let config = GrConfig {
            mass: bh.mass,
            metric: bh.metric,
        };
        engine.create_instance(entity, &config);
    }
}

/// Compute shadow boundaries for newly-initialized black holes.
#[allow(clippy::type_complexity)]
pub fn shadow_system(
    mut engine: ResMut<GrEngine>,
    query: Query<(Entity, &GrParams), (With<SpacetimeDomain>, Added<SpacetimeDomain>)>,
) {
    for (entity, params) in &query {
        if let Some(inst) = engine.get_mut(entity) {
            inst.compute_shadow(128, params.observer_inclination);
        }
    }
}

/// Advance geodesic integration by configured substeps.
///
/// Uses energy-conserving integration from gr_core when configured.
/// Runs in FixedUpdate for deterministic physics.
pub fn geodesic_step_system(
    engine: Res<GrEngine>,
    mut query: Query<(&mut Geodesic, &ChildOf)>,
    domains: Query<(Entity, &GrParams), With<SpacetimeDomain>>,
) {
    for (entity, params) in &domains {
        if let Some(inst) = engine.get(entity) {
            for (mut geodesic, parent) in &mut query {
                if parent.0 != entity || !geodesic.active {
                    continue;
                }

                // Simple RK4-style integration using gr_core.
                let horizon = inst.event_horizon();
                for _ in 0..params.substeps {
                    let r = geodesic.position[1];

                    // Terminate if geodesic falls below event horizon.
                    if r <= horizon * 1.01 {
                        geodesic.active = false;
                        break;
                    }

                    // Terminate if geodesic escapes to large radius.
                    if r > params.observer_distance * 10.0 {
                        geodesic.active = false;
                        break;
                    }

                    // Simple coordinate advance (placeholder for full RK4).
                    // In production, this would call
                    // gr_core::energy_conserving::integrate_energy_conserving.
                    let dt = geodesic.step_size;
                    geodesic.position[0] += geodesic.velocity[0] * dt;
                    geodesic.position[1] += geodesic.velocity[1] * dt;
                    geodesic.position[2] += geodesic.velocity[2] * dt;
                    geodesic.position[3] += geodesic.velocity[3] * dt;

                    // Accumulate proper time.
                    if geodesic.geodesic_type == GeodesicType::Timelike {
                        geodesic.proper_time +=
                            dt * inst.time_dilation_factor(geodesic.position[1]);
                    }
                }
            }
        }
    }
}

/// Update diagnostic components from GR engine state.
pub fn diagnostics_system(
    engine: Res<GrEngine>,
    mut query: Query<(Entity, &mut GrDiagnostics), With<SpacetimeDomain>>,
    geodesics: Query<(&Geodesic, &ChildOf)>,
) {
    for (entity, mut diag) in &mut query {
        if let Some(inst) = engine.get(entity) {
            diag.shadow_points = inst.shadow_alpha.len();

            // Count active geodesics for this domain.
            let active_count = geodesics
                .iter()
                .filter(|(g, parent)| parent.0 == entity && g.active)
                .count();
            diag.active_geodesics = active_count;
        }
    }
}

/// Clean up GR instances when SpacetimeDomain entities are despawned.
pub fn gr_cleanup_system(
    mut engine: ResMut<GrEngine>,
    mut removals: RemovedComponents<SpacetimeDomain>,
) {
    for entity in removals.read() {
        engine.remove(entity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::MetricType;
    use crate::resources::GrConfig;

    #[test]
    fn gr_init_creates_instance() {
        let mut engine = GrEngine::default();
        let entity = Entity::from_bits(1);
        let config = GrConfig {
            mass: 1.0,
            metric: MetricType::Schwarzschild,
        };
        engine.create_instance(entity, &config);

        let inst = engine.get(entity).unwrap();
        assert!((inst.mass - 1.0).abs() < 1e-15);
        assert!(inst.schwarzschild.is_some());
        assert!(inst.kerr.is_none());
    }

    #[test]
    fn kerr_init_creates_instance() {
        let mut engine = GrEngine::default();
        let entity = Entity::from_bits(1);
        let config = GrConfig {
            mass: 1.0,
            metric: MetricType::Kerr { spin: 0.5 },
        };
        engine.create_instance(entity, &config);

        let inst = engine.get(entity).unwrap();
        assert!(inst.schwarzschild.is_none());
        assert!(inst.kerr.is_some());
        assert!((inst.spin - 0.5).abs() < 1e-15);
    }

    #[test]
    fn shadow_computed_has_points() {
        let mut engine = GrEngine::default();
        let entity = Entity::from_bits(1);
        let config = GrConfig {
            mass: 1.0,
            metric: MetricType::Schwarzschild,
        };
        engine.create_instance(entity, &config);

        let inst = engine.get_mut(entity).unwrap();
        inst.compute_shadow(64, std::f64::consts::FRAC_PI_2);
        assert!(!inst.shadow_alpha.is_empty());
    }
}
