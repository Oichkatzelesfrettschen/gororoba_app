// Non-Euclidean puzzle game: rooms connected by hypercomplex rotations.
//
// Uses Cayley-Dickson algebra via gororoba_bevy_algebra for non-associative
// geometry, zero-divisor portals, and hypercomplex puzzle mechanics.
//
// Flow: Menu -> Exploring -> PuzzleSolving -> Results -> Menu.

use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy_egui::EguiGlobalSettings;
use gororoba_bevy_algebra::AlgebraPlugin;
use gororoba_bevy_core::{GororobaCorePlugin, HudState, OrbitCamera};

mod portals;
mod puzzles;
mod rooms;
mod states;
mod ui;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Gororoba: Non-Euclidean".into(),
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
        .add_plugins(AlgebraPlugin)
        .add_plugins(states::PuzzleStatesPlugin)
        .add_plugins(rooms::RoomsPlugin)
        .add_plugins(portals::PortalsPlugin)
        .add_plugins(puzzles::PuzzlesPlugin)
        .add_plugins(ui::PuzzleUiPlugin)
        .add_systems(Startup, setup_scene)
        .run();
}

/// Set up the initial 3D scene: camera, lights.
fn setup_scene(mut commands: Commands, mut hud: ResMut<HudState>) {
    // Orbit camera looking at the center of the room layout.
    commands.spawn((
        Camera3d::default(),
        OrbitCamera {
            radius: 60.0,
            pitch: -0.4,
            yaw: 0.3,
            target: Vec3::ZERO,
            ..default()
        },
        Transform::from_xyz(40.0, 30.0, 50.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Directional light.
    commands.spawn((
        DirectionalLight {
            illuminance: 10000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(10.0, 20.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Global ambient light.
    commands.insert_resource(GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 400.0,
        affects_lightmapped_meshes: true,
    });

    // Enable HUD by default.
    hud.visible = true;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_modules_exist() {
        // Verify all modules are accessible.
        let _ = states::PuzzleGamePhase::Menu;
        let _ = states::PuzzleSimState::Exploring;
        let _ = rooms::RoomLayout::default();
        let _ = portals::PortalTraversalCount::default();
        let _ = puzzles::PuzzleState::default();
    }
}
