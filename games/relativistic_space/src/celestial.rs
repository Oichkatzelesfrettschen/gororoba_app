// Celestial objects: black holes, accretion disks, stars.
//
// Sets up the GR domain with black hole parameters and spawns
// visual representations for the observer.

use bevy::prelude::*;

use gororoba_bevy_gr::{
    AccretionDisk, BlackHole, GrDiagnostics, GrEngine, GrParams, MetricType, SpacetimeDomain,
};

use crate::states::SpaceSimState;

/// Configuration for the celestial scene.
#[derive(Resource)]
pub struct CelestialConfig {
    /// Black hole mass (geometric units).
    pub mass: f64,
    /// Black hole spin parameter (0 = Schwarzschild, up to ~0.998).
    pub spin: f64,
    /// Observer distance (units of M).
    pub observer_distance: f64,
    /// Number of shadow boundary points to compute.
    pub shadow_points: usize,
}

impl Default for CelestialConfig {
    fn default() -> Self {
        Self {
            mass: 1.0,
            spin: 0.0,
            observer_distance: 50.0,
            shadow_points: 128,
        }
    }
}

/// Mission results accumulated during gameplay.
#[derive(Resource, Default)]
pub struct MissionResults {
    pub proper_time_elapsed: f64,
    pub coordinate_time_elapsed: f64,
    pub min_approach_radius: f64,
    pub geodesics_traced: usize,
}

/// Plugin for celestial object management.
pub struct CelestialPlugin;

impl Plugin for CelestialPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CelestialConfig>()
            .init_resource::<MissionResults>()
            .add_systems(OnEnter(SpaceSimState::Observing), setup_celestial)
            .add_systems(OnExit(SpaceSimState::Results), teardown_celestial)
            .add_systems(
                Update,
                (shadow_gizmo_system, accretion_gizmo_system, update_results)
                    .run_if(in_state(SpaceSimState::Observing)),
            );
    }
}

/// Spawn the spacetime domain with a black hole and accretion disk.
fn setup_celestial(mut commands: Commands, config: Res<CelestialConfig>) {
    let metric = if config.spin.abs() < 1e-12 {
        MetricType::Schwarzschild
    } else {
        MetricType::Kerr { spin: config.spin }
    };

    commands.spawn((
        SpacetimeDomain,
        BlackHole {
            mass: config.mass,
            metric,
        },
        AccretionDisk::default(),
        GrParams {
            observer_distance: config.observer_distance,
            ..default()
        },
        GrDiagnostics::default(),
    ));
}

/// Remove celestial objects when leaving.
fn teardown_celestial(mut commands: Commands, query: Query<Entity, With<SpacetimeDomain>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

/// Draw the black hole shadow boundary as a gizmo ring.
fn shadow_gizmo_system(
    engine: Res<GrEngine>,
    domain: Query<Entity, With<SpacetimeDomain>>,
    mut gizmos: Gizmos,
) {
    for entity in &domain {
        if let Some(inst) = engine.get(entity) {
            if inst.shadow_alpha.len() < 2 {
                continue;
            }

            // Draw shadow boundary as connected line segments.
            let scale = 2.0; // visual scaling
            for i in 0..inst.shadow_alpha.len() {
                let j = (i + 1) % inst.shadow_alpha.len();
                let a = Vec3::new(
                    inst.shadow_alpha[i] as f32 * scale,
                    inst.shadow_beta[i] as f32 * scale,
                    0.0,
                );
                let b = Vec3::new(
                    inst.shadow_alpha[j] as f32 * scale,
                    inst.shadow_beta[j] as f32 * scale,
                    0.0,
                );
                gizmos.line(a, b, Color::BLACK);
            }
        }
    }
}

/// Draw the accretion disk as concentric gizmo rings.
fn accretion_gizmo_system(
    query: Query<(&AccretionDisk, &BlackHole), With<SpacetimeDomain>>,
    mut gizmos: Gizmos,
) {
    for (disk, _bh) in &query {
        let n_rings = 8;
        let dr = (disk.r_outer - disk.r_inner) / n_rings as f64;

        for i in 0..n_rings {
            let r = disk.r_inner + dr * (i as f64 + 0.5);
            let t = i as f32 / n_rings as f32;
            // Color from hot (inner, yellow-white) to cool (outer, red).
            let color = Color::srgb(1.0, 1.0 - t * 0.7, (1.0 - t) * 0.5);

            gizmos.circle(
                Isometry3d::from_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                r as f32 * 2.0, // visual scaling
                color,
            );
        }
    }
}

/// Update mission results from GR diagnostics.
fn update_results(
    diag_query: Query<&GrDiagnostics, With<SpacetimeDomain>>,
    engine: Res<GrEngine>,
    domain: Query<Entity, With<SpacetimeDomain>>,
    mut results: ResMut<MissionResults>,
) {
    for entity in &domain {
        if let Ok(diag) = diag_query.get(entity) {
            results.proper_time_elapsed = diag.proper_time;
            results.coordinate_time_elapsed = diag.coordinate_time;
            results.geodesics_traced = diag.active_geodesics;
        }
        if let Some(inst) = engine.get(entity) {
            let horizon = inst.event_horizon();
            if results.min_approach_radius == 0.0 {
                results.min_approach_radius = horizon * 3.0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_reasonable() {
        let config = CelestialConfig::default();
        assert!(config.mass > 0.0);
        assert!(config.observer_distance > 0.0);
        assert!(config.shadow_points > 0);
    }

    #[test]
    fn default_results_zeroed() {
        let results = MissionResults::default();
        assert_eq!(results.geodesics_traced, 0);
        assert!((results.proper_time_elapsed).abs() < 1e-15);
    }

    #[test]
    fn schwarzschild_metric_for_zero_spin() {
        let config = CelestialConfig {
            spin: 0.0,
            ..default()
        };
        let metric = if config.spin.abs() < 1e-12 {
            MetricType::Schwarzschild
        } else {
            MetricType::Kerr { spin: config.spin }
        };
        assert_eq!(metric, MetricType::Schwarzschild);
    }

    #[test]
    fn kerr_metric_for_nonzero_spin() {
        let config = CelestialConfig {
            spin: 0.9,
            ..default()
        };
        let metric = if config.spin.abs() < 1e-12 {
            MetricType::Schwarzschild
        } else {
            MetricType::Kerr { spin: config.spin }
        };
        assert_eq!(metric, MetricType::Kerr { spin: 0.9 });
    }
}
