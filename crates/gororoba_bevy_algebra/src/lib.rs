// Cayley-Dickson algebra as a Bevy plugin.
//
// Uses gororoba_app's local kernel crates for non-associative geometry,
// zero-divisor portals, and hypercomplex rotations.
//
// Algebra initialization and zero-divisor search run in FixedUpdate.
// Diagnostics and portal spawning run in Update.

use bevy::prelude::*;

pub mod components;
pub mod resources;
pub mod systems;

pub use components::{
    AlgebraDiagnostics, AlgebraDimension, AlgebraDomain, AlgebraParams, HypercomplexElement,
    ZeroDivisorPortal,
};
pub use gororoba_kernel_api::projection::{ProjectedPoint3, ProjectionSpec};
pub use resources::{AlgebraConfig, AlgebraInstance, CdAlgebraEngine};

pub struct AlgebraPlugin;

impl Plugin for AlgebraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CdAlgebraEngine>()
            .add_systems(
                FixedUpdate,
                (
                    systems::algebra_init_system,
                    systems::zd_search_system,
                    systems::associator_system,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                (systems::diagnostics_system, systems::portal_spawn_system),
            )
            .add_systems(PostUpdate, systems::algebra_cleanup_system);
    }
}
