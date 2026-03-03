// Relativistic Space exploration game: observe and navigate near black holes.
//
// Uses general relativity via gororoba_bevy_gr for geodesic integration,
// gravitational lensing, shadow computation, and time dilation.
//
// Flow: Menu -> Observing -> Navigating -> Results -> Menu.

use bevy::prelude::*;
use gororoba_bevy_core::{GororobaCorePlugin, HudState, OrbitCamera};
use gororoba_bevy_gr::GrPlugin;

mod celestial;
mod spacecraft;
mod states;
mod ui;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Gororoba: Relativistic Space".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(GororobaCorePlugin)
        .add_plugins(GrPlugin)
        .add_plugins(states::SpaceStatesPlugin)
        .add_plugins(celestial::CelestialPlugin)
        .add_plugins(spacecraft::SpacecraftPlugin)
        .add_plugins(ui::SpaceUiPlugin)
        .add_systems(Startup, setup_scene)
        .run();
}

/// Set up the initial 3D scene: camera, lights.
fn setup_scene(mut commands: Commands, mut hud: ResMut<HudState>) {
    commands.spawn((
        Camera3d::default(),
        OrbitCamera {
            radius: 80.0,
            pitch: -0.2,
            yaw: 0.3,
            target: Vec3::ZERO,
            ..default()
        },
        Transform::from_xyz(60.0, 30.0, 60.0).looking_at(Vec3::ZERO, Vec3::Y),
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
        brightness: 200.0,
        affects_lightmapped_meshes: true,
    });

    hud.visible = true;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_modules_exist() {
        let _ = states::SpaceGamePhase::Menu;
        let _ = states::SpaceSimState::Observing;
        let _ = celestial::CelestialConfig::default();
        let _ = celestial::MissionResults::default();
        let _ = spacecraft::Spacecraft::default();
        let _ = spacecraft::TimeDilationDisplay::default();
    }
}
