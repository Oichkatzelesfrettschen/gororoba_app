// Fluid Dynamics game: vehicle design and wind tunnel simulation.
//
// Uses LBM (Lattice Boltzmann Method) via gororoba_bevy_lbm for
// real-time computational fluid dynamics.

use bevy::prelude::*;
use gororoba_bevy_core::GororobaCorePlugin;
use gororoba_bevy_lbm::LbmPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Gororoba: Fluid Dynamics".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(GororobaCorePlugin)
        .add_plugins(LbmPlugin)
        .run();
}
