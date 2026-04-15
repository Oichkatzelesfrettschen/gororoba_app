// Aerodynamic diagnostics: drag, lift, Reynolds number, MLUPS.
//
// Reads simulation diagnostics from the LBM engine and pushes
// formatted values to the shared HUD.

use bevy::prelude::*;

use gororoba_bevy_core::HudState;
use gororoba_bevy_lbm::{LbmCpuEngine, SimulationDiagnostics, VoxelGrid};

use crate::scenarios::{WindTunnelConfig, WindTunnelDomain};
use crate::states::FluidSimState;

/// Cached aerodynamic results for the Results screen.
#[derive(Resource, Default)]
pub struct AerodynamicResults {
    pub drag: f64,
    pub lift: f64,
    pub drag_coefficient: f64,
    pub lift_coefficient: f64,
    pub reynolds_number: f64,
    pub timestep: usize,
    pub mlups: f64,
}

/// Plugin for aerodynamic diagnostics.
pub struct AerodynamicsPlugin;

impl Plugin for AerodynamicsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AerodynamicResults>()
            .init_resource::<MlupsTimer>()
            .add_systems(
                Update,
                (hud_diagnostics_system, compute_aero_system)
                    .run_if(in_state(FluidSimState::WindTunnel)),
            );
    }
}

/// Timer for MLUPS (million lattice updates per second) measurement.
#[derive(Resource)]
pub struct MlupsTimer {
    pub last_timestep: usize,
    pub timer: Timer,
}

impl Default for MlupsTimer {
    fn default() -> Self {
        Self {
            last_timestep: 0,
            timer: Timer::from_seconds(1.0, TimerMode::Repeating),
        }
    }
}

/// Compute drag and lift from the LBM velocity field.
fn compute_aero_system(
    engine: Res<LbmCpuEngine>,
    config: Res<WindTunnelConfig>,
    domain_query: Query<(Entity, &VoxelGrid), With<WindTunnelDomain>>,
    diag_query: Query<&SimulationDiagnostics, With<WindTunnelDomain>>,
    time: Res<Time>,
    mut results: ResMut<AerodynamicResults>,
    mut mlups_timer: ResMut<MlupsTimer>,
) {
    for (entity, voxels) in &domain_query {
        if let Some(inst) = engine.get(entity) {
            let snapshot = inst.aerodynamic_snapshot(voxels);
            results.drag = snapshot.drag;
            results.lift = snapshot.lift;
            results.drag_coefficient = snapshot.drag_coefficient;
            results.lift_coefficient = snapshot.lift_coefficient;
            results.reynolds_number = snapshot.reynolds_number;
        }

        // MLUPS computation.
        if let Ok(diag) = diag_query.get(entity) {
            results.timestep = diag.timestep;
            mlups_timer.timer.tick(time.delta());
            if mlups_timer.timer.just_finished() {
                let steps = diag.timestep.saturating_sub(mlups_timer.last_timestep);
                let n = config.nx * config.ny * config.nz;
                results.mlups = (steps * n) as f64 / 1e6;
                mlups_timer.last_timestep = diag.timestep;
            }
        }
    }
}

/// Push simulation diagnostics to the HUD.
fn hud_diagnostics_system(
    diag_query: Query<&SimulationDiagnostics, With<WindTunnelDomain>>,
    results: Res<AerodynamicResults>,
    mut hud: ResMut<HudState>,
) {
    for diag in &diag_query {
        hud.set("Timestep", format!("{}", diag.timestep));
        hud.set("Max Velocity", format!("{:.4}", diag.max_velocity));
        hud.set("Mean Velocity", format!("{:.4}", diag.mean_velocity));
        hud.set("Total Mass", format!("{:.2}", diag.total_mass));
        hud.set(
            "Stable",
            if diag.stable {
                "Yes".to_string()
            } else {
                "UNSTABLE".to_string()
            },
        );
        hud.set("Drag (Cd)", format!("{:.4}", results.drag_coefficient));
        hud.set("Lift (Cl)", format!("{:.4}", results.lift_coefficient));
        hud.set("Re", format!("{:.0}", results.reynolds_number));
        hud.set("MLUPS", format!("{:.1}", results.mlups));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_results_zeroed() {
        let results = AerodynamicResults::default();
        assert!((results.drag).abs() < 1e-15);
        assert!((results.lift).abs() < 1e-15);
        assert_eq!(results.timestep, 0);
    }

    #[test]
    fn mlups_timer_default() {
        let timer = MlupsTimer::default();
        assert_eq!(timer.last_timestep, 0);
    }
}
