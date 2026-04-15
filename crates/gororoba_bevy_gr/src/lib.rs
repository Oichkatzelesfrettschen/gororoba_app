// General relativity as a Bevy plugin.
//
// Uses gororoba_app's local relativity kernel boundary for geodesic
// integration, gravitational lensing, and time dilation.
//
// Geodesic integration runs in FixedUpdate (deterministic physics).
// Shadow computation and diagnostics run in Update.

use bevy::prelude::*;

pub mod components;
pub mod resources;
pub mod systems;

pub use components::{
    AccretionDisk, BlackHole, Geodesic, GeodesicType, GrDiagnostics, GrParams, MetricType,
    SpacetimeDomain,
};
pub use gororoba_kernel_gr::GrConfig;
pub use resources::{GrEngine, GrInstance};

pub struct GrPlugin;

impl Plugin for GrPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GrEngine>()
            .add_systems(
                FixedUpdate,
                (systems::gr_init_system, systems::geodesic_step_system).chain(),
            )
            .add_systems(
                Update,
                (systems::shadow_system, systems::diagnostics_system),
            )
            .add_systems(PostUpdate, systems::gr_cleanup_system);
    }
}
