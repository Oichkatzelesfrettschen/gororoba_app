// Strategy builder: place Player moves, check constraints, undo.

use bevy::prelude::*;
use bevy_egui::input::EguiWantsInput;

use gororoba_bevy_game_semantics::{GameSemanticsEngine, LevelPhase, Polarity};

use crate::states::SimState;

pub struct BuilderPlugin;

impl Plugin for BuilderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            builder_input_system.run_if(in_state(SimState::StrategyBuilder)),
        );
    }
}

fn builder_input_system(
    keys: Res<ButtonInput<KeyCode>>,
    egui_input: Res<EguiWantsInput>,
    mut engine: ResMut<GameSemanticsEngine>,
) {
    if egui_input.wants_any_keyboard_input() {
        return;
    }
    if engine.phase != LevelPhase::StrategyBuilder {
        return;
    }

    // Undo with Backspace.
    if keys.just_pressed(KeyCode::Backspace) {
        engine.undo_move();
        return;
    }

    // Number keys 1-9 select from available moves.
    let available = engine.available_moves();
    // Filter to Player moves only (the player places their own moves).
    let player_available: Vec<usize> = available
        .into_iter()
        .filter(|&m| {
            engine.strategy.as_ref().is_some_and(|s| {
                m < s.arena.events.len() && s.arena.events[m].polarity == Polarity::Player
            })
        })
        .collect();

    let key_map = [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
        KeyCode::Digit8,
        KeyCode::Digit9,
    ];

    for (i, &key) in key_map.iter().enumerate() {
        if keys.just_pressed(key) && i < player_available.len() {
            engine.add_player_move(player_available[i]);
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_plugin_constructs() {
        let _ = BuilderPlugin;
    }
}
