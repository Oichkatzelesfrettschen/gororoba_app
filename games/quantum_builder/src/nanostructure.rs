// Nanostructure building: Casimir plates and spin lattice placement.
//
// The player arranges parallel plates and spin lattice sites
// to construct quantum nanostructures. Casimir energy and
// entanglement entropy are computed by the quantum plugin.

use bevy::prelude::*;

use gororoba_bevy_quantum::{
    CasimirParams, CasimirPlate, PlateGeometry, QuantumDiagnostics, QuantumDomain, QuantumEngine,
    QuantumParams, SpinLattice,
};

use crate::states::QuantumSimState;

/// Configuration for the nanostructure scene.
#[derive(Resource)]
pub struct NanostructureConfig {
    /// Number of spin sites in the lattice.
    pub n_sites: usize,
    /// Local Hilbert space dimension (2 = spin-1/2).
    pub local_dim: usize,
    /// Default plate separation for Casimir effect.
    pub plate_separation: f64,
    /// Number of Casimir plate pairs to spawn.
    pub n_plate_pairs: usize,
}

impl Default for NanostructureConfig {
    fn default() -> Self {
        Self {
            n_sites: 16,
            local_dim: 2,
            plate_separation: 1.0,
            n_plate_pairs: 3,
        }
    }
}

/// Experiment results accumulated during gameplay.
#[derive(Resource, Default)]
pub struct ExperimentResults {
    pub peak_entropy: f64,
    pub final_casimir_energy: f64,
    pub final_casimir_error: f64,
    pub measurements_performed: usize,
    pub lattice_sites: usize,
    pub plate_pairs: usize,
}

/// Plugin for nanostructure management.
pub struct NanostructurePlugin;

impl Plugin for NanostructurePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NanostructureConfig>()
            .init_resource::<ExperimentResults>()
            .add_systems(OnEnter(QuantumSimState::Building), setup_nanostructure)
            .add_systems(OnExit(QuantumSimState::Results), teardown_nanostructure)
            .add_systems(
                Update,
                (
                    plate_editor_system,
                    plate_gizmo_system,
                    lattice_gizmo_system,
                    update_results_system,
                )
                    .run_if(in_state(QuantumSimState::Building)),
            );
    }
}

/// Spawn the quantum domain with spin lattice and Casimir plates.
fn setup_nanostructure(mut commands: Commands, config: Res<NanostructureConfig>) {
    // Spawn quantum domain with spin lattice.
    commands.spawn((
        QuantumDomain,
        SpinLattice {
            n_sites: config.n_sites,
            local_dim: config.local_dim,
            seed: 42,
        },
        QuantumParams::default(),
        QuantumDiagnostics::default(),
        CasimirPlate {
            geometry: PlateGeometry::ParallelPlates {
                separation: config.plate_separation,
            },
            position: [0.0, config.plate_separation / 2.0, 0.0],
        },
        CasimirParams::default(),
    ));

    // Spawn additional plate pairs at different positions.
    for i in 1..config.n_plate_pairs {
        let x_offset = i as f64 * 5.0;
        let separation = config.plate_separation + i as f64 * 0.5;
        commands.spawn((
            QuantumDomain,
            SpinLattice {
                n_sites: config.n_sites,
                local_dim: config.local_dim,
                seed: 42 + i as u64,
            },
            QuantumParams::default(),
            QuantumDiagnostics::default(),
            CasimirPlate {
                geometry: PlateGeometry::ParallelPlates { separation },
                position: [x_offset, separation / 2.0, 0.0],
            },
            CasimirParams::default(),
        ));
    }
}

/// Remove nanostructure entities when leaving.
fn teardown_nanostructure(mut commands: Commands, query: Query<Entity, With<QuantumDomain>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

/// Allow the player to adjust plate separation with Q/E keys.
fn plate_editor_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut plates: Query<&mut CasimirPlate, With<QuantumDomain>>,
) {
    let delta = if keys.pressed(KeyCode::KeyQ) {
        -0.01
    } else if keys.pressed(KeyCode::KeyE) {
        0.01
    } else {
        return;
    };

    for mut plate in &mut plates {
        if let PlateGeometry::ParallelPlates { ref mut separation } = plate.geometry {
            *separation = (*separation + delta).max(0.1);
            plate.position[1] = *separation / 2.0;
        }
    }
}

/// Draw Casimir plates as gizmo rectangles.
fn plate_gizmo_system(plates: Query<&CasimirPlate, With<QuantumDomain>>, mut gizmos: Gizmos) {
    for plate in &plates {
        let pos = plate.position;
        let center = Vec3::new(pos[0] as f32, pos[1] as f32, pos[2] as f32);

        if let PlateGeometry::ParallelPlates { separation } = plate.geometry {
            let half_sep = separation as f32 / 2.0;
            let plate_size = 3.0;

            // Top plate.
            let top = center + Vec3::Y * half_sep;
            gizmos.rect(
                Isometry3d::from_translation(top),
                Vec2::splat(plate_size),
                Color::srgb(0.3, 0.7, 1.0),
            );

            // Bottom plate.
            let bottom = center - Vec3::Y * half_sep;
            gizmos.rect(
                Isometry3d::from_translation(bottom),
                Vec2::splat(plate_size),
                Color::srgb(0.3, 0.7, 1.0),
            );

            // Connecting lines at corners to show the gap.
            let offsets = [
                Vec3::new(-plate_size / 2.0, 0.0, -plate_size / 2.0),
                Vec3::new(plate_size / 2.0, 0.0, -plate_size / 2.0),
                Vec3::new(plate_size / 2.0, 0.0, plate_size / 2.0),
                Vec3::new(-plate_size / 2.0, 0.0, plate_size / 2.0),
            ];
            for offset in &offsets {
                gizmos.line(top + *offset, bottom + *offset, Color::srgb(0.2, 0.5, 0.8));
            }
        }
    }
}

/// Draw spin lattice sites as small spheres in a ring layout.
fn lattice_gizmo_system(
    lattices: Query<(&SpinLattice, &CasimirPlate), With<QuantumDomain>>,
    engine: Res<QuantumEngine>,
    domain: Query<Entity, With<QuantumDomain>>,
    mut gizmos: Gizmos,
) {
    for entity in &domain {
        if let Ok((lattice, plate)) = lattices.get(entity) {
            let center = Vec3::new(
                plate.position[0] as f32,
                plate.position[1] as f32,
                plate.position[2] as f32,
            );

            let has_entropy = engine
                .get(entity)
                .map(|inst| inst.entropy > 0.0)
                .unwrap_or(false);

            for i in 0..lattice.n_sites {
                let angle = std::f32::consts::TAU * i as f32 / lattice.n_sites as f32;
                let radius = 2.0;
                let site_pos = center + Vec3::new(radius * angle.cos(), 0.0, radius * angle.sin());

                // Color sites by entropy state: green if entangled, gray if not.
                let color = if has_entropy {
                    let t = i as f32 / lattice.n_sites as f32;
                    Color::srgb(0.2 + t * 0.6, 0.8 - t * 0.3, 0.4)
                } else {
                    Color::srgb(0.5, 0.5, 0.5)
                };

                gizmos.sphere(Isometry3d::from_translation(site_pos), 0.15, color);
            }

            // Draw bonds between adjacent sites.
            for i in 0..lattice.n_sites {
                let j = (i + 1) % lattice.n_sites;
                let angle_i = std::f32::consts::TAU * i as f32 / lattice.n_sites as f32;
                let angle_j = std::f32::consts::TAU * j as f32 / lattice.n_sites as f32;
                let radius = 2.0;
                let pos_i = center + Vec3::new(radius * angle_i.cos(), 0.0, radius * angle_i.sin());
                let pos_j = center + Vec3::new(radius * angle_j.cos(), 0.0, radius * angle_j.sin());
                gizmos.line(pos_i, pos_j, Color::srgb(0.4, 0.4, 0.6));
            }
        }
    }
}

/// Update experiment results from quantum diagnostics.
fn update_results_system(
    diag_query: Query<&QuantumDiagnostics, With<QuantumDomain>>,
    config: Res<NanostructureConfig>,
    mut results: ResMut<ExperimentResults>,
) {
    results.lattice_sites = config.n_sites;
    results.plate_pairs = config.n_plate_pairs;

    for diag in &diag_query {
        if diag.entanglement_entropy > results.peak_entropy {
            results.peak_entropy = diag.entanglement_entropy;
        }
        results.final_casimir_energy = diag.casimir_energy;
        results.final_casimir_error = diag.casimir_error;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_reasonable() {
        let config = NanostructureConfig::default();
        assert!(config.n_sites > 0);
        assert!(config.local_dim >= 2);
        assert!(config.plate_separation > 0.0);
        assert!(config.n_plate_pairs > 0);
    }

    #[test]
    fn default_results_zeroed() {
        let results = ExperimentResults::default();
        assert_eq!(results.measurements_performed, 0);
        assert!((results.peak_entropy).abs() < 1e-15);
    }

    #[test]
    fn plate_separation_clamp() {
        let mut sep = 0.05_f64;
        sep = sep.max(0.1);
        assert!((sep - 0.1).abs() < 1e-15);
    }
}
