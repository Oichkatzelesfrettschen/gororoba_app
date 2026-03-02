// Non-Euclidean puzzle game.
//
// Uses Cayley-Dickson algebra via gororoba_bevy_algebra for non-associative
// geometry, zero-divisor portals, and hypercomplex rotations.

use bevy::prelude::*;
use gororoba_bevy_algebra::AlgebraPlugin;
use gororoba_bevy_core::GororobaCorePlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Gororoba: Non-Euclidean".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(GororobaCorePlugin)
        .add_plugins(AlgebraPlugin)
        .run();
}
