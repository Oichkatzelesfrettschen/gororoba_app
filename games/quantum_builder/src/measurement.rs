// Quantum measurement mechanics.
//
// The player triggers measurements that update the entanglement
// entropy and Casimir energy readings. Each measurement is a
// stochastic process (the quantum plugin uses Monte Carlo internally),
// so repeated measurements yield slightly different results,
// illustrating quantum uncertainty.

use bevy::prelude::*;

use gororoba_bevy_quantum::{QuantumDiagnostics, QuantumDomain, QuantumEngine};

use crate::nanostructure::ExperimentResults;
use crate::states::QuantumSimState;

/// Tracks the history of measurement outcomes.
#[derive(Resource, Default)]
pub struct MeasurementLog {
    /// Entropy values from successive measurements.
    pub entropy_history: Vec<f64>,
    /// Casimir energy values from successive measurements.
    pub casimir_history: Vec<f64>,
    /// Whether a measurement was just performed this frame.
    pub just_measured: bool,
}

/// Plugin for quantum measurement systems.
pub struct MeasurementPlugin;

impl Plugin for MeasurementPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MeasurementLog>()
            .add_systems(OnEnter(QuantumSimState::Measuring), reset_measurement_log)
            .add_systems(
                Update,
                (
                    measurement_trigger_system,
                    measurement_record_system,
                    measurement_gizmo_system,
                )
                    .chain()
                    .run_if(in_state(QuantumSimState::Measuring)),
            );
    }
}

/// Reset the measurement log when entering measurement mode.
fn reset_measurement_log(mut log: ResMut<MeasurementLog>) {
    log.entropy_history.clear();
    log.casimir_history.clear();
    log.just_measured = false;
}

/// Trigger a measurement when the player presses M.
///
/// Sets the measured_this_tick flag on diagnostics, which causes
/// the quantum plugin to record fresh values from the stochastic
/// computation. Each press of M represents one quantum measurement.
fn measurement_trigger_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut diag_query: Query<&mut QuantumDiagnostics, With<QuantumDomain>>,
    mut log: ResMut<MeasurementLog>,
) {
    log.just_measured = false;

    if keys.just_pressed(KeyCode::KeyM) {
        for mut diag in &mut diag_query {
            diag.measured_this_tick = true;
            log.just_measured = true;
        }
    }
}

/// Record measurement results into the log and experiment results.
fn measurement_record_system(
    diag_query: Query<&QuantumDiagnostics, With<QuantumDomain>>,
    mut log: ResMut<MeasurementLog>,
    mut results: ResMut<ExperimentResults>,
) {
    if !log.just_measured {
        return;
    }

    for diag in &diag_query {
        log.entropy_history.push(diag.entanglement_entropy);
        log.casimir_history.push(diag.casimir_energy);
        results.measurements_performed += 1;

        if diag.entanglement_entropy > results.peak_entropy {
            results.peak_entropy = diag.entanglement_entropy;
        }
        results.final_casimir_energy = diag.casimir_energy;
        results.final_casimir_error = diag.casimir_error;
    }
}

/// Visualize measurement outcomes as vertical bars (histogram-like).
fn measurement_gizmo_system(
    log: Res<MeasurementLog>,
    engine: Res<QuantumEngine>,
    domain: Query<Entity, With<QuantumDomain>>,
    mut gizmos: Gizmos,
) {
    let Some(entity) = domain.iter().next() else {
        return;
    };
    let has_instance = engine.get(entity).is_some();
    if !has_instance {
        return;
    }

    // Draw entropy history as vertical bars along the X axis.
    let bar_spacing = 0.5;
    let bar_width = 0.3;
    let base_y = -5.0_f32;
    let base_z = 8.0_f32;

    for (i, &entropy) in log.entropy_history.iter().enumerate() {
        let x = i as f32 * bar_spacing - log.entropy_history.len() as f32 * bar_spacing / 2.0;
        let height = entropy as f32 * 3.0;

        let bottom = Vec3::new(x, base_y, base_z);
        let top = Vec3::new(x, base_y + height, base_z);

        // Color bars by entropy value.
        let t = (entropy as f32).min(1.0);
        let color = Color::srgb(0.2 + t * 0.8, 0.8 - t * 0.5, 0.3);

        gizmos.line(bottom, top, color);

        // Draw horizontal cap.
        gizmos.line(
            top - Vec3::X * bar_width / 2.0,
            top + Vec3::X * bar_width / 2.0,
            color,
        );
    }

    // Draw Casimir energy history as bars along Z offset.
    let casimir_z = base_z + 3.0;
    for (i, &energy) in log.casimir_history.iter().enumerate() {
        let x = i as f32 * bar_spacing - log.casimir_history.len() as f32 * bar_spacing / 2.0;
        // Casimir energy is negative; show magnitude.
        let height = energy.abs() as f32 * 5.0;

        let bottom = Vec3::new(x, base_y, casimir_z);
        let top = Vec3::new(x, base_y + height, casimir_z);

        // Blue for Casimir energy.
        let color = Color::srgb(0.2, 0.4, 0.9);
        gizmos.line(bottom, top, color);
        gizmos.line(
            top - Vec3::X * bar_width / 2.0,
            top + Vec3::X * bar_width / 2.0,
            color,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_log_empty() {
        let log = MeasurementLog::default();
        assert!(log.entropy_history.is_empty());
        assert!(log.casimir_history.is_empty());
        assert!(!log.just_measured);
    }

    #[test]
    fn log_accumulates_measurements() {
        let mut log = MeasurementLog::default();
        log.entropy_history.push(0.5);
        log.casimir_history.push(-0.01);
        log.entropy_history.push(0.7);
        log.casimir_history.push(-0.02);

        assert_eq!(log.entropy_history.len(), 2);
        assert_eq!(log.casimir_history.len(), 2);
    }

    #[test]
    fn peak_entropy_tracking() {
        let mut results = ExperimentResults::default();
        let values = [0.3, 0.8, 0.5, 0.9, 0.1];
        for &v in &values {
            if v > results.peak_entropy {
                results.peak_entropy = v;
            }
        }
        assert!((results.peak_entropy - 0.9).abs() < 1e-15);
    }
}
