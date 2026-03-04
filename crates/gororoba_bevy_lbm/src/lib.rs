// LBM physics as a Bevy plugin.
//
// Wraps open_gororoba's lbm_3d CPU solver (and future lbm_vulkan GPU solver)
// as Bevy resources, components, and systems.
//
// Physics steps run in FixedUpdate (deterministic, framerate-independent).
// Diagnostics and rendering readback run in Update.

use bevy::prelude::*;

pub mod components;
pub mod compute_bridge;
pub mod resources;
pub mod soa_solver;
pub mod systems;

pub use components::{
    BoundaryConditions, BoundaryType, FluidDomain, SimulationDiagnostics, SimulationParams,
    VoxelGrid,
};
pub use compute_bridge::{GpuBridgeConfig, GpuFrameTarget, GpuVulkanEngine};
pub use resources::{LbmCpuEngine, SolverConfig, SolverInstance};
pub use soa_solver::LbmSolverSoA;

pub struct LbmPlugin;

impl Plugin for LbmPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LbmCpuEngine>()
            .init_resource::<GpuBridgeConfig>()
            .init_resource::<GpuFrameTarget>()
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
                    compute_bridge::gpu_bridge_init_system,
                    systems::diagnostics_system,
                    compute_bridge::gpu_readback_system,
                )
                    .chain(),
            )
            .add_systems(PostUpdate, systems::solver_cleanup_system);
    }
}
