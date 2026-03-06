// Interaction Arena: game semantics + classical game theory strategy game.
//
// Teaches arenas, strategies, conditions, composition, payoffs, Nash
// equilibria, and Abramsky's semantic cube across 10 progressive levels.
//
// Flow: Menu -> ArenaView -> StrategyBuilder -> Execution -> Results -> Menu.

use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy_egui::EguiGlobalSettings;
use gororoba_bevy_core::{GororobaCorePlugin, HudState, OrbitCamera};
use gororoba_bevy_game_semantics::GameSemanticsPlugin;

mod builder;
mod execution;
mod states;
mod ui;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Gororoba: Interaction Arena".into(),
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
        .add_plugins(GameSemanticsPlugin)
        .add_plugins(states::InteractionStatesPlugin)
        .add_plugins(builder::BuilderPlugin)
        .add_plugins(execution::ExecutionPlugin)
        .add_plugins(ui::InteractionUiPlugin)
        .add_systems(Startup, setup_scene)
        .run();
}

fn setup_scene(mut commands: Commands, mut hud: ResMut<HudState>) {
    commands.spawn((
        Camera3d::default(),
        OrbitCamera {
            radius: 50.0,
            pitch: -0.3,
            yaw: 0.4,
            target: Vec3::ZERO,
            ..default()
        },
        Transform::from_xyz(30.0, 20.0, 40.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 10000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(10.0, 20.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.insert_resource(GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 400.0,
        affects_lightmapped_meshes: true,
    });

    hud.visible = true;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_modules_exist() {
        let _ = states::InteractionArenaPhase::Menu;
        let _ = states::SimState::ArenaView;
    }
}
