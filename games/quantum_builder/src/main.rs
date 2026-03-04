// Quantum Builder: nanoscale sandbox.
//
// Uses quantum mechanics and Casimir effect via gororoba_bevy_quantum
// for MERA tensor networks, spin lattices, and Casimir forces.
//
// Flow: Menu -> Building -> Measuring -> Results -> Menu.

use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy_egui::EguiGlobalSettings;
use gororoba_bevy_core::{GororobaCorePlugin, HudState, OrbitCamera};
use gororoba_bevy_quantum::QuantumPlugin;

mod lattice_editor;
mod measurement;
mod nanostructure;
mod states;
mod ui;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Gororoba: Quantum Builder".into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(LogPlugin {
                    filter: "info,wgpu_hal=error,bevy_render::view::window=error".into(),
                    ..default()
                }),
        )
        .insert_resource(EguiGlobalSettings {
            enable_absorb_bevy_input_system: true,
            ..default()
        })
        .add_plugins(GororobaCorePlugin)
        .add_plugins(QuantumPlugin)
        .add_plugins(states::QuantumStatesPlugin)
        .add_plugins(nanostructure::NanostructurePlugin)
        .add_plugins(lattice_editor::LatticeEditorPlugin)
        .add_plugins(measurement::MeasurementPlugin)
        .add_plugins(ui::QuantumUiPlugin)
        .add_systems(Startup, setup_scene)
        .run();
}

/// Set up the initial 3D scene: camera, lights.
fn setup_scene(mut commands: Commands, mut hud: ResMut<HudState>) {
    commands.spawn((
        Camera3d::default(),
        OrbitCamera {
            radius: 40.0,
            pitch: -0.4,
            yaw: 0.5,
            target: Vec3::ZERO,
            ..default()
        },
        Transform::from_xyz(30.0, 20.0, 30.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 5000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(10.0, 20.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.insert_resource(GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 300.0,
        affects_lightmapped_meshes: true,
    });

    hud.visible = true;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_modules_exist() {
        let _ = states::QuantumGamePhase::Menu;
        let _ = states::QuantumSimState::Building;
        let _ = nanostructure::NanostructureConfig::default();
        let _ = nanostructure::ExperimentResults::default();
        let _ = lattice_editor::LatticeSelection::default();
        let _ = measurement::MeasurementLog::default();
    }
}
