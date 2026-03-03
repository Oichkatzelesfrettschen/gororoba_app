// Quantum mechanics and Casimir effect as a Bevy plugin.
//
// Wraps open_gororoba's quantum_core, casimir_core, and
// spin_tomography_core for MERA tensor networks, Casimir energy,
// and quantum measurement.
//
// MERA steps and Casimir computation run in FixedUpdate.
// Diagnostics run in Update.

use bevy::prelude::*;

pub mod components;
pub mod resources;
pub mod systems;

pub use components::{
    CasimirParams, CasimirPlate, EntangledPair, PlateGeometry, QuantumDiagnostics, QuantumDomain,
    QuantumParams, SpinLattice,
};
pub use resources::{QuantumConfig, QuantumEngine, QuantumInstance};

pub struct QuantumPlugin;

impl Plugin for QuantumPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<QuantumEngine>()
            .add_systems(
                FixedUpdate,
                (
                    systems::quantum_init_system,
                    systems::mera_step_system,
                    systems::casimir_system,
                )
                    .chain(),
            )
            .add_systems(Update, systems::diagnostics_system)
            .add_systems(PostUpdate, systems::quantum_cleanup_system);
    }
}
