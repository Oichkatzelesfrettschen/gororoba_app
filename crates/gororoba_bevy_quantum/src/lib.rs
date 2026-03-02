// Quantum mechanics and Casimir effect as a Bevy plugin.
//
// Wraps open_gororoba's quantum_core, casimir_core, and
// spin_tomography_core for MERA tensor networks, Casimir energy,
// and quantum measurement.

use bevy::prelude::*;

pub mod components;
pub mod resources;
pub mod systems;

pub struct QuantumPlugin;

impl Plugin for QuantumPlugin {
    fn build(&self, _app: &mut App) {
        // Phase 7: register quantum resources, events, and systems.
        // Quantum state evolution runs in FixedUpdate.
    }
}
