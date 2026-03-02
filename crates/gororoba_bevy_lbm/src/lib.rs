// LBM physics as a Bevy plugin.
//
// Wraps open_gororoba's lbm_vulkan compute engine and exposes it as
// Bevy resources, components, and systems.

use bevy::prelude::*;

pub mod components;
pub mod compute_bridge;
pub mod resources;
pub mod systems;

pub struct LbmPlugin;

impl Plugin for LbmPlugin {
    fn build(&self, _app: &mut App) {
        // Phase 2: register LBM resources, events, and systems.
        // Physics steps run in FixedUpdate; readback and rendering in Update.
    }
}
