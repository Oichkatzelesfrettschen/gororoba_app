// Geodesic integration, shadow computation, and time dilation systems.
//
// gr_init_system initializes GR instances for new spacetime domains.
// geodesic_step_system delegates integration to the local relativity kernel.
// shadow_system computes black hole shadow boundaries once per domain init.
// diagnostics_system updates diagnostic components from kernel outputs.

use bevy::prelude::*;
use gororoba_kernel_api::relativity::{GeodesicKind, GeodesicSnapshot, RelativityKernel};

use crate::components::{
    BlackHole, Geodesic, GeodesicType, GrDiagnostics, GrParams, SpacetimeDomain,
};
use crate::resources::{GrConfig, GrEngine};

#[allow(clippy::type_complexity)]
pub fn gr_init_system(
    mut engine: ResMut<GrEngine>,
    query: Query<(Entity, &BlackHole, &GrParams), (With<SpacetimeDomain>, Added<SpacetimeDomain>)>,
) {
    for (entity, bh, params) in &query {
        let config = GrConfig {
            mass: bh.mass,
            metric: bh.metric,
            observer_inclination: params.observer_inclination,
            observer_distance: params.observer_distance,
            shadow_points: 128,
        };
        engine.create_instance(entity, &config);
    }
}

#[allow(clippy::type_complexity)]
pub fn shadow_system(mut engine: ResMut<GrEngine>, query: Query<Entity, Added<SpacetimeDomain>>) {
    for entity in &query {
        if let Some(inst) = engine.get_mut(entity) {
            inst.compute_shadow();
        }
    }
}

pub fn geodesic_step_system(
    engine: Res<GrEngine>,
    mut geodesics: Query<(Entity, &mut Geodesic, &ChildOf)>,
    domains: Query<(Entity, &GrParams), With<SpacetimeDomain>>,
) {
    for (entity, params) in &domains {
        let Some(inst) = engine.get(entity) else {
            continue;
        };

        let mut targets = Vec::new();
        let mut snapshots = Vec::new();
        for (geodesic_entity, geodesic, parent) in &mut geodesics {
            if parent.0 != entity {
                continue;
            }
            targets.push(geodesic_entity);
            snapshots.push(GeodesicSnapshot {
                position: geodesic.position,
                velocity: geodesic.velocity,
                kind: match geodesic.geodesic_type {
                    GeodesicType::Null => GeodesicKind::Null,
                    GeodesicType::Timelike => GeodesicKind::Timelike,
                },
                step_size: geodesic.step_size,
                max_steps: geodesic.max_steps,
                active: geodesic.active,
                proper_time: geodesic.proper_time,
            });
        }

        let _ = inst.kernel.step_geodesics(&mut snapshots, params.substeps);

        for (geodesic_entity, snapshot) in targets.into_iter().zip(snapshots) {
            if let Ok((_, mut geodesic, _)) = geodesics.get_mut(geodesic_entity) {
                geodesic.position = snapshot.position;
                geodesic.velocity = snapshot.velocity;
                geodesic.geodesic_type = match snapshot.kind {
                    GeodesicKind::Null => GeodesicType::Null,
                    GeodesicKind::Timelike => GeodesicType::Timelike,
                };
                geodesic.step_size = snapshot.step_size;
                geodesic.max_steps = snapshot.max_steps;
                geodesic.active = snapshot.active;
                geodesic.proper_time = snapshot.proper_time;
            }
        }
    }
}

pub fn diagnostics_system(
    engine: Res<GrEngine>,
    mut query: Query<(Entity, &GrParams, &mut GrDiagnostics), With<SpacetimeDomain>>,
    geodesics: Query<(&Geodesic, &ChildOf)>,
) {
    for (entity, params, mut diag) in &mut query {
        let Some(inst) = engine.get(entity) else {
            continue;
        };

        let mut snapshots = Vec::new();
        for (geodesic, parent) in &geodesics {
            if parent.0 != entity {
                continue;
            }
            snapshots.push(GeodesicSnapshot {
                position: geodesic.position,
                velocity: geodesic.velocity,
                kind: match geodesic.geodesic_type {
                    GeodesicType::Null => GeodesicKind::Null,
                    GeodesicType::Timelike => GeodesicKind::Timelike,
                },
                step_size: geodesic.step_size,
                max_steps: geodesic.max_steps,
                active: geodesic.active,
                proper_time: geodesic.proper_time,
            });
        }

        let kernel_diag = inst.kernel.step_geodesics(&mut snapshots, 0);
        diag.coordinate_time = kernel_diag.coordinate_time;
        diag.proper_time = kernel_diag.proper_time;
        diag.time_dilation = inst.time_dilation_factor(params.observer_distance);
        diag.active_geodesics = kernel_diag.active_geodesics;
        diag.shadow_points = inst.shadow_alpha.len();
    }
}

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

    #[test]
    fn gr_init_creates_instance() {
        let mut engine = GrEngine::default();
        let entity = Entity::from_bits(1);
        let config = GrConfig {
            mass: 1.0,
            metric: MetricType::Schwarzschild,
            observer_inclination: std::f64::consts::FRAC_PI_2,
            observer_distance: 50.0,
            shadow_points: 64,
        };
        engine.create_instance(entity, &config);

        let inst = engine.get(entity).unwrap();
        assert!((inst.kernel.mass() - 1.0).abs() < 1e-15);
    }

    #[test]
    fn shadow_computed_has_points() {
        let mut engine = GrEngine::default();
        let entity = Entity::from_bits(1);
        let config = GrConfig {
            mass: 1.0,
            metric: MetricType::Schwarzschild,
            observer_inclination: std::f64::consts::FRAC_PI_2,
            observer_distance: 50.0,
            shadow_points: 64,
        };
        engine.create_instance(entity, &config);

        let inst = engine.get_mut(entity).unwrap();
        inst.compute_shadow();
        assert!(!inst.shadow_alpha.is_empty());
    }
}
