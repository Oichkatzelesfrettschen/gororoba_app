// LBM physics as a Bevy plugin.
//
// Wraps local fluid kernels as Bevy resources, components, and systems.
//
// Physics steps run in FixedUpdate (deterministic, framerate-independent).
// Diagnostics and rendering readback run in Update.

use bevy::prelude::*;

pub mod components;
pub mod compute_bridge;
pub mod resources;
pub mod systems;

pub use components::{
    BoundaryConditions, BoundaryType, FluidDomain, SimulationDiagnostics, SimulationParams,
    VoxelGrid,
};
pub use compute_bridge::{FluidFrameBridgeConfig, FluidFrameTarget};
pub use gororoba_kernel_fluid::{LbmSolverScalar, LbmSolverSoA, LocalCpuFluidKernel, VoxelMask3};
pub use resources::{FluidSimulationEngine, LbmCpuEngine, SolverConfig, SolverInstance};

pub struct LbmPlugin;

impl Plugin for LbmPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FluidSimulationEngine>()
            .init_resource::<FluidFrameBridgeConfig>()
            .init_resource::<FluidFrameTarget>()
            .add_systems(
                FixedUpdate,
                (
                    systems::solver_init_system,
                    systems::simulation_step_system,
                    systems::boundary_update_system,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                (
                    compute_bridge::frame_bridge_init_system,
                    systems::diagnostics_system,
                    compute_bridge::frame_readback_system,
                )
                    .chain(),
            )
            .add_systems(PostUpdate, systems::solver_cleanup_system);
    }
}
