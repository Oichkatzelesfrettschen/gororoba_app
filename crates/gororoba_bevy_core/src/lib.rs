// Shared Bevy plugin infrastructure for Gororoba games.
//
// Sub-plugins: camera, HUD, pedagogy panels, input abstractions.

use bevy::prelude::*;

pub mod camera;
pub mod hud;
pub mod input;
pub mod pedagogy;

/// Top-level plugin bundle that registers all shared infrastructure.
pub struct GororobaCorePlugin;

impl Plugin for GororobaCorePlugin {
    fn build(&self, _app: &mut App) {
        // Phase 1: register camera, HUD, pedagogy, and input sub-plugins.
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
