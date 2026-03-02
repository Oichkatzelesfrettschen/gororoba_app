// Quantum Builder: nanoscale sandbox.
//
// Uses quantum mechanics and Casimir effect via gororoba_bevy_quantum
// for MERA tensor networks, spin lattices, and Casimir forces.

use bevy::prelude::*;
use gororoba_bevy_core::GororobaCorePlugin;
use gororoba_bevy_quantum::QuantumPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Gororoba: Quantum Builder".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(GororobaCorePlugin)
        .add_plugins(QuantumPlugin)
        .run();
}
