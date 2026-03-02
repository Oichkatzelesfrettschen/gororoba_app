// Game input abstractions.
//
// Maps raw Bevy keyboard/mouse/gamepad input to semantic game actions,
// decoupling game logic from specific key bindings.

use bevy::prelude::*;

pub struct GameInputPlugin;

impl Plugin for GameInputPlugin {
    fn build(&self, _app: &mut App) {
        // Phase 1: implement input abstraction layer.
    }
}
