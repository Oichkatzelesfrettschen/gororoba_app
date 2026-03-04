// Geodesic integration, shadow computation, and time dilation systems.
//
// gr_init_system initializes GR instances for new spacetime domains.
// geodesic_step_system integrates geodesics (FixedUpdate).
// shadow_system computes black hole shadow boundaries (Update, once).
// diagnostics_system updates diagnostic components (Update).

use bevy::prelude::*;

use gr_core::energy_conserving::{FullGeodesicState, energy_conserving_step};
use gr_core::metric::SpacetimeMetric;

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
/// Uses energy-conserving RK4 from gr_core with constraint-preserving
/// corrections. Each substep calls energy_conserving_step() which:
/// 1. Performs a 4th-order Runge-Kutta step using the geodesic equation
/// 2. Applies constraint correction to preserve the metric norm
///    (g_ab v^a v^b = 0 for null, -1 for timelike geodesics)
///
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

                let horizon = inst.event_horizon();
                let target_norm = match geodesic.geodesic_type {
                    GeodesicType::Null => 0.0,
                    GeodesicType::Timelike => -1.0,
                };

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

                    let state = FullGeodesicState {
                        x: geodesic.position,
                        v: geodesic.velocity,
                    };

                    let new_state = if let Some(ref kerr) = inst.kerr {
                        energy_conserving_step(
                            &state,
                            geodesic.step_size,
                            |x| kerr.metric_components(x),
                            |x| kerr.christoffel(x),
                            target_norm,
                        )
                    } else if let Some(ref schw) = inst.schwarzschild {
                        energy_conserving_step(
                            &state,
                            geodesic.step_size,
                            |x| schw.metric_components(x),
                            |x| schw.christoffel(x),
                            target_norm,
                        )
                    } else {
                        break;
                    };

                    geodesic.position = new_state.x;
                    geodesic.velocity = new_state.v;

                    // Accumulate proper time for massive particles.
                    if geodesic.geodesic_type == GeodesicType::Timelike {
                        geodesic.proper_time +=
                            geodesic.step_size * inst.time_dilation_factor(new_state.x[1]);
                    }
                }
            }
        }
    }
}

/// Update diagnostic components from GR engine state.
///
/// Populates coordinate time (max t among active geodesics), proper time
/// (max among timelike geodesics), time dilation at observer distance,
/// active geodesic count, and shadow point count.
pub fn diagnostics_system(
    engine: Res<GrEngine>,
    mut query: Query<(Entity, &GrParams, &mut GrDiagnostics), With<SpacetimeDomain>>,
    geodesics: Query<(&Geodesic, &ChildOf)>,
) {
    for (entity, params, mut diag) in &mut query {
        if let Some(inst) = engine.get(entity) {
            diag.shadow_points = inst.shadow_alpha.len();

            let mut active_count = 0usize;
            let mut max_coord_time = diag.coordinate_time;
            let mut max_proper_time = diag.proper_time;

            for (g, parent) in &geodesics {
                if parent.0 != entity {
                    continue;
                }
                if g.active {
                    active_count += 1;
                }
                // Coordinate time is Boyer-Lindquist t (position[0]).
                if g.position[0] > max_coord_time {
                    max_coord_time = g.position[0];
                }
                // Proper time from timelike geodesics.
                if g.geodesic_type == GeodesicType::Timelike && g.proper_time > max_proper_time {
                    max_proper_time = g.proper_time;
                }
            }

            diag.active_geodesics = active_count;
            diag.coordinate_time = max_coord_time;
            diag.proper_time = max_proper_time;
            diag.time_dilation = inst.time_dilation_factor(params.observer_distance);
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
    use gr_core::metric::SpacetimeMetric;

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

    #[test]
    fn energy_conserving_step_preserves_null_constraint() {
        // A null geodesic at r=10M in Schwarzschild should maintain
        // g_ab v^a v^b = 0 after integration.
        let schw = gr_core::Schwarzschild::new(1.0);

        // Start at r=10M, equatorial plane, radial infall.
        let x0 = [0.0, 10.0, std::f64::consts::FRAC_PI_2, 0.0];
        let g = schw.metric_components(&x0);

        // For a null radial geodesic: g_tt (v^t)^2 + g_rr (v^r)^2 = 0.
        // g_tt = -(1 - 2/10) = -0.8, g_rr = 1/(1 - 2/10) = 1.25.
        // So v^t = sqrt(g_rr / |g_tt|) * v^r = sqrt(1.25/0.8) * v^r.
        let v_r: f64 = -0.1;
        let v_t = (g[1][1] / (-g[0][0])).sqrt() * v_r.abs();
        let v0 = [v_t, v_r, 0.0, 0.0];

        // Verify initial null condition.
        let norm0: f64 = (0..4)
            .flat_map(|a| (0..4).map(move |b| (a, b)))
            .map(|(a, b)| g[a][b] * v0[a] * v0[b])
            .sum();
        assert!(norm0.abs() < 1e-10, "initial norm not null: {norm0}");

        let state = FullGeodesicState { x: x0, v: v0 };
        let new_state = energy_conserving_step(
            &state,
            0.1,
            |x| schw.metric_components(x),
            |x| schw.christoffel(x),
            0.0,
        );

        // Check constraint at new position.
        let g_new = schw.metric_components(&new_state.x);
        let norm_new: f64 = (0..4)
            .flat_map(|a| (0..4).map(move |b| (a, b)))
            .map(|(a, b)| g_new[a][b] * new_state.v[a] * new_state.v[b])
            .sum();
        assert!(
            norm_new.abs() < 1e-8,
            "null constraint violated after step: {norm_new}"
        );

        // Radius should decrease (infall).
        assert!(
            new_state.x[1] < 10.0,
            "r should decrease: {}",
            new_state.x[1]
        );
    }

    #[test]
    fn time_dilation_at_observer_distance() {
        let mut engine = GrEngine::default();
        let entity = Entity::from_bits(1);
        let config = GrConfig {
            mass: 1.0,
            metric: MetricType::Schwarzschild,
        };
        engine.create_instance(entity, &config);

        let inst = engine.get(entity).unwrap();
        // At r = 50M (default observer_distance), dilation should be close to 1.
        let td = inst.time_dilation_factor(50.0);
        let expected = (1.0 - 2.0 / 50.0_f64).sqrt();
        assert!(
            (td - expected).abs() < 1e-12,
            "time dilation at r=50: got {td}, expected {expected}"
        );
        // At event horizon, dilation should be 0.
        let td_horizon = inst.time_dilation_factor(2.0);
        assert!(
            td_horizon.abs() < 1e-12,
            "time dilation at horizon should be 0: {td_horizon}"
        );
    }

    #[test]
    fn kerr_geodesic_step_runs() {
        let kerr = gr_core::Kerr::new(1.0, 0.5);
        let x0 = [0.0, 10.0, std::f64::consts::FRAC_PI_2, 0.0];
        let g = kerr.metric_components(&x0);

        // Null radial infall.
        let v_r: f64 = -0.1;
        let v_t = (g[1][1] / (-g[0][0])).sqrt() * v_r.abs();
        let v0 = [v_t, v_r, 0.0, 0.0];

        let state = FullGeodesicState { x: x0, v: v0 };
        let new_state = energy_conserving_step(
            &state,
            0.1,
            |x| kerr.metric_components(x),
            |x| kerr.christoffel(x),
            0.0,
        );

        // Just check it ran and produced finite results.
        assert!(new_state.x[1].is_finite());
        assert!(new_state.v[0].is_finite());
        assert!(new_state.x[1] < 10.0, "r should decrease");
    }
}
