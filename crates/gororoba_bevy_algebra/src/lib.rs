// Cayley-Dickson algebra as a Bevy plugin.
//
// Wraps open_gororoba's cd_kernel and algebra_core for non-associative
// geometry, zero-divisor portals, and hypercomplex rotations.

use bevy::prelude::*;

pub mod components;
pub mod resources;
pub mod systems;

pub struct AlgebraPlugin;

impl Plugin for AlgebraPlugin {
    fn build(&self, _app: &mut App) {
        // Phase 4: register algebra resources, events, and systems.
        // Algebra steps run in FixedUpdate.
    }
}
