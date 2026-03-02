// Relativistic Space exploration game.
//
// Uses general relativity via gororoba_bevy_gr for geodesic integration,
// gravitational lensing, and time dilation around black holes.

use bevy::prelude::*;
use gororoba_bevy_core::GororobaCorePlugin;
use gororoba_bevy_gr::GrPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Gororoba: Relativistic Space".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(GororobaCorePlugin)
        .add_plugins(GrPlugin)
        .run();
}
