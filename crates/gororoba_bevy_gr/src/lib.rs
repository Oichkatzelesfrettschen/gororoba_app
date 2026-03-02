// General relativity as a Bevy plugin.
//
// Wraps open_gororoba's gr_core and cosmology_core for geodesic
// integration, gravitational lensing, and time dilation.

use bevy::prelude::*;

pub mod components;
pub mod resources;
pub mod systems;

pub struct GrPlugin;

impl Plugin for GrPlugin {
    fn build(&self, _app: &mut App) {
        // Phase 6: register GR resources, events, and systems.
        // Geodesic integration runs in FixedUpdate.
    }
}
