// Game-specific UI: menu, builder HUD, measurement display,
// results screen, and pedagogy content.

use bevy::prelude::*;
use bevy_egui::EguiContexts;

use gororoba_bevy_core::{PedagogyMode, PedagogyState};
use gororoba_bevy_quantum::{QuantumDiagnostics, QuantumDomain, QuantumEngine};

use crate::lattice_editor::LatticeSelection;
use crate::measurement::MeasurementLog;
use crate::nanostructure::{ExperimentResults, NanostructureConfig};
use crate::states::{QuantumGamePhase, QuantumSimState};

/// Plugin for game-specific UI systems.
pub struct QuantumUiPlugin;

impl Plugin for QuantumUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_pedagogy)
            .add_systems(
                Update,
                menu_ui_system.run_if(in_state(QuantumGamePhase::Menu)),
            )
            .add_systems(
                Update,
                builder_ui_system.run_if(in_state(QuantumSimState::Building)),
            )
            .add_systems(
                Update,
                measurement_ui_system.run_if(in_state(QuantumSimState::Measuring)),
            )
            .add_systems(
                Update,
                results_ui_system.run_if(in_state(QuantumSimState::Results)),
            );
    }
}

/// Register pedagogy content about quantum mechanics and Casimir effect.
fn setup_pedagogy(mut pedagogy: ResMut<PedagogyState>) {
    pedagogy.add(
        PedagogyMode::Story,
        "Quantum Builder",
        "Build nanoscale structures and explore quantum phenomena. \
         Arrange spin lattices to create entanglement, position \
         Casimir plates to harness vacuum fluctuations.",
    );
    pedagogy.add(
        PedagogyMode::Explorer,
        "Entanglement Entropy",
        "Entanglement entropy S measures quantum correlations between \
         subsystems. For a bipartite system, S = -Tr(rho_A log rho_A). \
         MERA tensor networks efficiently approximate ground-state entropy.",
    );
    pedagogy.add(
        PedagogyMode::Explorer,
        "Casimir Effect",
        "Two uncharged conducting plates attract due to vacuum \
         fluctuations of the electromagnetic field. The energy is \
         E = -pi^2 hbar c / (720 d^3) per unit area. Computed here \
         via worldline Monte Carlo.",
    );
    pedagogy.add(
        PedagogyMode::Research,
        "MERA Networks",
        "Multi-scale Entanglement Renormalization Ansatz: a tensor \
         network that captures entanglement at all length scales. \
         Layers of disentanglers and isometries coarse-grain the lattice. \
         Efficient for critical (conformal) systems.",
    );
    pedagogy.add(
        PedagogyMode::Research,
        "Worldline Monte Carlo",
        "Casimir energies are computed by summing over closed Brownian \
         loops (worldlines) in Euclidean spacetime. Each loop contributes \
         to the vacuum energy via its winding number and proper-time weight. \
         Statistical error decreases as 1/sqrt(N_loops).",
    );
}

fn menu_ui_system(mut contexts: EguiContexts) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    bevy_egui::egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(100.0);
            ui.heading("Gororoba: Quantum Builder");
            ui.add_space(20.0);
            ui.label("Build nanostructures and explore quantum phenomena.");
            ui.add_space(40.0);
            ui.label("Press SPACE to start");
            ui.add_space(20.0);
            ui.label("Controls:");
            ui.label("  Right-click + drag: orbit camera");
            ui.label("  Scroll: zoom");
            ui.label("  Q/E: decrease/increase plate separation");
            ui.label("  1-9: select lattice sites");
            ui.label("  0: clear selection");
            ui.label("  M: perform measurement");
            ui.label("  Enter: advance to next phase");
            ui.label("  F1: toggle HUD");
            ui.label("  F2: toggle pedagogy panel");
        });
    });
}

/// Builder mode HUD: lattice config, plate separation, diagnostics.
fn builder_ui_system(
    mut contexts: EguiContexts,
    config: Res<NanostructureConfig>,
    selection: Res<LatticeSelection>,
    engine: Res<QuantumEngine>,
    domain: Query<Entity, With<QuantumDomain>>,
    diag_query: Query<&QuantumDiagnostics, With<QuantumDomain>>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    bevy_egui::egui::Window::new("Quantum Builder")
        .anchor(
            bevy_egui::egui::Align2::LEFT_TOP,
            bevy_egui::egui::vec2(10.0, 10.0),
        )
        .resizable(false)
        .show(ctx, |ui| {
            ui.label(format!("Lattice Sites: {}", config.n_sites));
            ui.label(format!("Local Dimension: {}", config.local_dim));
            ui.label(format!("Plate Pairs: {}", config.n_plate_pairs));
            ui.label(format!("Plate Separation: {:.2}", config.plate_separation));

            ui.separator();
            ui.heading("Selection");
            match (selection.site_a, selection.site_b) {
                (Some(a), Some(b)) => ui.label(format!("Selected: {a} - {b}")),
                (Some(a), None) => ui.label(format!("Selected: {a} (pick second)")),
                _ => ui.label("No site selected"),
            };
            ui.label(format!("Pairs Created: {}", selection.pairs_created));

            if let Some(entity) = domain.iter().next() {
                if let Some(inst) = engine.get(entity) {
                    ui.separator();
                    ui.heading("Tensor Network");
                    ui.label(format!("MERA Layers: {}", inst.layer_count()));
                    ui.label(format!("Entropy: {:.4}", inst.entropy));
                }

                if let Ok(diag) = diag_query.get(entity) {
                    ui.separator();
                    ui.heading("Casimir Energy");
                    ui.label(format!("Energy: {:.6}", diag.casimir_energy));
                    ui.label(format!("Error: {:.6}", diag.casimir_error));
                }
            }

            ui.separator();
            ui.label("Q/E: adjust plate separation");
            ui.label("1-9: select sites, 0: clear");
            ui.label("Press ENTER to measure");
        });
}

/// Measurement mode HUD: measurement log, statistics.
fn measurement_ui_system(
    mut contexts: EguiContexts,
    log: Res<MeasurementLog>,
    diag_query: Query<&QuantumDiagnostics, With<QuantumDomain>>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    bevy_egui::egui::Window::new("Measurement")
        .anchor(
            bevy_egui::egui::Align2::LEFT_TOP,
            bevy_egui::egui::vec2(10.0, 10.0),
        )
        .resizable(false)
        .show(ctx, |ui| {
            ui.heading("Quantum Measurement");
            ui.label(format!("Measurements: {}", log.entropy_history.len()));

            if !log.entropy_history.is_empty() {
                let avg_entropy: f64 =
                    log.entropy_history.iter().sum::<f64>() / log.entropy_history.len() as f64;
                let max_entropy = log
                    .entropy_history
                    .iter()
                    .cloned()
                    .fold(f64::NEG_INFINITY, f64::max);
                let min_entropy = log
                    .entropy_history
                    .iter()
                    .cloned()
                    .fold(f64::INFINITY, f64::min);

                ui.separator();
                ui.heading("Entropy Statistics");
                ui.label(format!("Average: {avg_entropy:.4}"));
                ui.label(format!("Min: {min_entropy:.4}"));
                ui.label(format!("Max: {max_entropy:.4}"));
                ui.label(format!("Spread: {:.4}", max_entropy - min_entropy));
            }

            if !log.casimir_history.is_empty() {
                let avg_casimir: f64 =
                    log.casimir_history.iter().sum::<f64>() / log.casimir_history.len() as f64;

                ui.separator();
                ui.heading("Casimir Energy");
                ui.label(format!("Average: {avg_casimir:.6}"));
                ui.label(format!(
                    "Latest: {:.6}",
                    log.casimir_history.last().unwrap_or(&0.0)
                ));
            }

            for diag in &diag_query {
                ui.separator();
                ui.label(format!("Live Entropy: {:.4}", diag.entanglement_entropy));
                ui.label(format!("Live Casimir: {:.6}", diag.casimir_energy));
            }

            ui.separator();
            ui.label("M: perform measurement");
            ui.label("Press ENTER for results");
        });
}

/// Results screen.
fn results_ui_system(mut contexts: EguiContexts, results: Res<ExperimentResults>) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    bevy_egui::egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(50.0);
            ui.heading("Experiment Results");
            ui.add_space(20.0);

            bevy_egui::egui::Grid::new("results_grid")
                .num_columns(2)
                .spacing([40.0, 8.0])
                .show(ui, |ui| {
                    ui.label("Lattice Sites:");
                    ui.label(format!("{}", results.lattice_sites));
                    ui.end_row();

                    ui.label("Plate Pairs:");
                    ui.label(format!("{}", results.plate_pairs));
                    ui.end_row();

                    ui.label("Measurements Performed:");
                    ui.label(format!("{}", results.measurements_performed));
                    ui.end_row();

                    ui.label("Peak Entropy:");
                    ui.label(format!("{:.4}", results.peak_entropy));
                    ui.end_row();

                    ui.label("Final Casimir Energy:");
                    ui.label(format!("{:.6}", results.final_casimir_energy));
                    ui.end_row();

                    ui.label("Casimir Error:");
                    ui.label(format!("{:.6}", results.final_casimir_error));
                    ui.end_row();
                });

            ui.add_space(30.0);
            ui.label("Press ENTER to return to menu");
        });
    });
}

#[cfg(test)]
mod tests {
    use gororoba_bevy_core::PedagogyMode;

    #[test]
    fn pedagogy_modes_distinct() {
        // Verify all three pedagogy modes used in setup_pedagogy are distinct.
        let modes = [
            PedagogyMode::Story,
            PedagogyMode::Explorer,
            PedagogyMode::Research,
        ];
        assert_ne!(modes[0], modes[1]);
        assert_ne!(modes[1], modes[2]);
        assert_ne!(modes[0], modes[2]);
    }
}
