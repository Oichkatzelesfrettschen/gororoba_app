// Fluid Dynamics game: vehicle design and wind tunnel simulation.
//
// Uses LBM (Lattice Boltzmann Method) via gororoba_bevy_lbm for
// real-time computational fluid dynamics.
//
// Flow: Menu -> VehicleDesign -> WindTunnel -> Results -> Menu.

use bevy::prelude::*;
use gororoba_bevy_core::{GororobaCorePlugin, HudState, OrbitCamera};
use gororoba_bevy_lbm::LbmPlugin;

mod aerodynamics;
mod scenarios;
mod states;
mod ui;
mod vehicle;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Gororoba: Fluid Dynamics".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(GororobaCorePlugin)
        .add_plugins(LbmPlugin)
        .add_plugins(states::FluidStatesPlugin)
        .add_plugins(scenarios::ScenariosPlugin)
        .add_plugins(aerodynamics::AerodynamicsPlugin)
        .add_plugins(ui::FluidUiPlugin)
        .add_systems(Startup, setup_scene)
        .add_systems(
            Update,
            vehicle::vehicle_gizmo_system.run_if(in_state(states::FluidSimState::VehicleDesign)),
        )
        .run();
}

/// Set up the initial 3D scene: camera, lights, ground reference.
fn setup_scene(mut commands: Commands, mut hud: ResMut<HudState>) {
    // Orbit camera looking at the center of the domain.
    commands.spawn((
        Camera3d::default(),
        OrbitCamera {
            radius: 50.0,
            pitch: -0.3,
            yaw: 0.5,
            target: Vec3::ZERO,
            ..default()
        },
        Transform::from_xyz(30.0, 20.0, 40.0).looking_at(Vec3::ZERO, Vec3::Y),
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
        brightness: 300.0,
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
        let _ = states::FluidGamePhase::Menu;
        let _ = states::FluidSimState::VehicleDesign;
        let _ = scenarios::WindTunnelConfig::default();
        let _ = aerodynamics::AerodynamicResults::default();
        let _ = vehicle::VehiclePreset::Sphere;
    }
}
