// Shared Bevy plugin infrastructure for Gororoba games.
//
// Sub-plugins: camera, HUD, pedagogy panels, input abstractions.

use bevy::prelude::*;
use bevy_egui::EguiContexts;

pub mod camera;
pub mod hud;
pub mod input;
pub mod pedagogy;

pub use camera::{FlyCamera, FlyCameraPlugin, OrbitCamera, OrbitCameraPlugin};
pub use hud::{HudPlugin, HudState};
pub use input::{GameAction, GameInputPlugin, InputBindings};
pub use pedagogy::{PedagogyMode, PedagogyPlugin, PedagogyState};

/// Marker resource inserted once the egui context has completed its first
/// pass. UI systems should use `.run_if(resource_exists::<EguiReady>)` to
/// avoid calling egui before fonts and layout state are initialized.
#[derive(Resource)]
pub struct EguiReady;

/// Detects when the egui context has completed at least one pass and
/// inserts the [`EguiReady`] marker resource.
fn detect_egui_ready(
    mut commands: Commands,
    mut contexts: EguiContexts,
    ready: Option<Res<EguiReady>>,
) {
    if ready.is_some() {
        return;
    }
    // If we can get the context AND it has completed at least one pass,
    // fonts and layout are available.
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    if ctx.cumulative_pass_nr() > 0 {
        commands.insert_resource(EguiReady);
    }
}

/// Top-level plugin bundle that registers all shared infrastructure.
pub struct GororobaCorePlugin;

impl Plugin for GororobaCorePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(OrbitCameraPlugin)
            .add_plugins(FlyCameraPlugin)
            .add_plugins(HudPlugin)
            .add_plugins(PedagogyPlugin)
            .add_plugins(GameInputPlugin)
            .init_state::<GameState>()
            .add_systems(Update, detect_egui_ready)
            .add_systems(Update, handle_game_actions);
    }
}

/// Shared game state machine used by all four games.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum GameState {
    #[default]
    Menu,
    Loading,
    Playing,
    Paused,
}

fn handle_game_actions(
    mut actions: MessageReader<GameAction>,
    state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut hud: ResMut<HudState>,
    mut pedagogy: ResMut<PedagogyState>,
) {
    for action in actions.read() {
        match action {
            GameAction::Pause => match state.get() {
                GameState::Playing => next_state.set(GameState::Paused),
                GameState::Paused => next_state.set(GameState::Playing),
                _ => {}
            },
            GameAction::ToggleHud => {
                hud.visible = !hud.visible;
            }
            GameAction::TogglePedagogy => {
                pedagogy.visible = !pedagogy.visible;
            }
            GameAction::Interact => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_state_default_is_menu() {
        assert_eq!(GameState::default(), GameState::Menu);
    }
}
