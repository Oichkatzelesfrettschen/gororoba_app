// Wind tunnel scenario: domain setup and simulation lifecycle.
//
// Spawns the LBM fluid domain with appropriate boundary conditions
// and manages the simulation lifecycle (setup, run, teardown).

use bevy::prelude::*;

use gororoba_bevy_lbm::{FluidDomain, SimulationDiagnostics, SimulationParams, VoxelGrid};

use crate::states::FluidSimState;
use crate::vehicle::{Vehicle, VehiclePreset, preset_voxels};

/// Wind tunnel configuration resource.
#[derive(Resource)]
pub struct WindTunnelConfig {
    /// Domain size in lattice units.
    pub nx: usize,
    pub ny: usize,
    pub nz: usize,
    /// Freestream velocity (lattice units, typically 0.01-0.1).
    pub freestream_velocity: [f64; 3],
    /// Relaxation time (controls viscosity).
    pub tau: f64,
    /// Substeps per FixedUpdate tick.
    pub substeps: usize,
    /// Currently selected preset (for the menu).
    pub selected_preset: VehiclePreset,
}

impl Default for WindTunnelConfig {
    fn default() -> Self {
        Self {
            nx: 64,
            ny: 32,
            nz: 32,
            freestream_velocity: [0.05, 0.0, 0.0],
            tau: 0.8,
            substeps: 3,
            selected_preset: VehiclePreset::Sphere,
        }
    }
}

/// Marker for the fluid domain entity in the wind tunnel.
#[derive(Component)]
pub struct WindTunnelDomain;

/// Plugin for wind tunnel scenario management.
pub struct ScenariosPlugin;

impl Plugin for ScenariosPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WindTunnelConfig>()
            .add_systems(OnEnter(FluidSimState::WindTunnel), setup_wind_tunnel)
            .add_systems(OnExit(FluidSimState::WindTunnel), teardown_wind_tunnel)
            .add_systems(OnEnter(FluidSimState::VehicleDesign), setup_design_mode);
    }
}

/// Spawn the fluid domain entity with the vehicle placed inside.
fn setup_wind_tunnel(
    mut commands: Commands,
    config: Res<WindTunnelConfig>,
    vehicles: Query<&VoxelGrid, With<Vehicle>>,
) {
    // Use the vehicle's voxel grid if available, otherwise generate from preset.
    let voxels = if let Some(grid) = vehicles.iter().next() {
        VoxelGrid::new(grid.nx, grid.ny, grid.nz)
    } else {
        preset_voxels(config.selected_preset, config.nx, config.ny, config.nz)
    };

    // Copy vehicle voxels into a fresh domain-sized grid.
    let mut domain_grid = VoxelGrid::new(config.nx, config.ny, config.nz);
    if let Some(vehicle_grid) = vehicles.iter().next() {
        // Center the vehicle in the domain.
        let ox = (config.nx.saturating_sub(vehicle_grid.nx)) / 2;
        let oy = (config.ny.saturating_sub(vehicle_grid.ny)) / 2;
        let oz = (config.nz.saturating_sub(vehicle_grid.nz)) / 2;
        for z in 0..vehicle_grid.nz.min(config.nz) {
            for y in 0..vehicle_grid.ny.min(config.ny) {
                for x in 0..vehicle_grid.nx.min(config.nx) {
                    if vehicle_grid.get(x, y, z) {
                        let dx = ox + x;
                        let dy = oy + y;
                        let dz = oz + z;
                        if dx < config.nx && dy < config.ny && dz < config.nz {
                            domain_grid.set(dx, dy, dz, true);
                        }
                    }
                }
            }
        }
    } else {
        domain_grid = voxels;
    }

    commands.spawn((
        WindTunnelDomain,
        FluidDomain,
        domain_grid,
        SimulationParams {
            tau: config.tau,
            rho_init: 1.0,
            u_init: config.freestream_velocity,
            force: [0.0, 0.0, 0.0],
            substeps: config.substeps,
        },
        SimulationDiagnostics::default(),
    ));
}

/// Remove the wind tunnel domain when leaving the WindTunnel state.
fn teardown_wind_tunnel(mut commands: Commands, query: Query<Entity, With<WindTunnelDomain>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

/// Set up the design mode with a fresh vehicle if none exists.
fn setup_design_mode(
    mut commands: Commands,
    config: Res<WindTunnelConfig>,
    existing: Query<Entity, With<Vehicle>>,
) {
    if existing.is_empty() {
        let voxels = preset_voxels(config.selected_preset, config.nx, config.ny, config.nz);
        commands.spawn((
            Vehicle {
                preset: Some(config.selected_preset),
            },
            voxels,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_valid() {
        let config = WindTunnelConfig::default();
        assert!(config.tau > 0.5, "tau must be > 0.5 for stability");
        assert!(config.nx > 0 && config.ny > 0 && config.nz > 0);
        assert!(config.substeps > 0);
    }

    #[test]
    fn freestream_velocity_reasonable() {
        let config = WindTunnelConfig::default();
        let u_mag: f64 = config
            .freestream_velocity
            .iter()
            .map(|v| v * v)
            .sum::<f64>()
            .sqrt();
        // LBM stability: Mach number < ~0.3, so |u| < 0.1 in lattice units.
        assert!(u_mag < 0.2, "freestream velocity too high: {u_mag}");
    }
}
