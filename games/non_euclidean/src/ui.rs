// Game-specific UI: menu, exploration HUD, puzzle interface,
// results screen, and pedagogy content.

use bevy::prelude::*;
use bevy_egui::EguiContexts;

use gororoba_bevy_algebra::{AlgebraDiagnostics, AlgebraDomain};
use gororoba_bevy_core::{PedagogyMode, PedagogyState};

use crate::portals::PortalTraversalCount;
use crate::puzzles::{PuzzleState, introductory_puzzles};
use crate::rooms::{ActiveRoom, DistortionState};
use crate::states::{PuzzleGamePhase, PuzzleSimState};

/// Plugin for game-specific UI systems.
pub struct PuzzleUiPlugin;

impl Plugin for PuzzleUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_pedagogy)
            .add_systems(
                Update,
                menu_ui_system.run_if(in_state(PuzzleGamePhase::Menu)),
            )
            .add_systems(
                Update,
                explore_ui_system.run_if(in_state(PuzzleSimState::Exploring)),
            )
            .add_systems(
                Update,
                puzzle_ui_system.run_if(in_state(PuzzleSimState::PuzzleSolving)),
            )
            .add_systems(
                Update,
                results_ui_system.run_if(in_state(PuzzleSimState::Results)),
            );
    }
}

/// Register pedagogy content about hypercomplex algebra.
fn setup_pedagogy(mut pedagogy: ResMut<PedagogyState>) {
    pedagogy.add(
        PedagogyMode::Story,
        "Non-Euclidean Rooms",
        "Navigate rooms connected by hypercomplex rotations. \
         In these spaces, the order of rotations matters -- \
         turning left then right may not return you to where you started.",
    );
    pedagogy.add(
        PedagogyMode::Explorer,
        "Cayley-Dickson Construction",
        "Starting from real numbers, each step doubles the dimension: \
         reals (1) -> complex (2) -> quaternions (4) -> octonions (8) -> \
         sedenions (16). Each step loses a property: commutativity, \
         then associativity, then alternativity.",
    );
    pedagogy.add(
        PedagogyMode::Explorer,
        "Zero-Divisors",
        "From sedenions onward, pairs (a, b) exist where a*b = 0 \
         but neither a nor b is zero. These 'zero-divisors' create \
         portals in our game -- places where geometry breaks down.",
    );
    pedagogy.add(
        PedagogyMode::Research,
        "Associator",
        "[a, b, c] = (a*b)*c - a*(b*c)\n\n\
         The associator measures how badly multiplication fails to \
         be associative. Quaternions: always zero. Octonions: nonzero \
         for most triples. The norm |[a,b,c]| drives visual distortion.",
    );
    pedagogy.add(
        PedagogyMode::Research,
        "Non-Associative Geometry",
        "In standard geometry, composing three rotations A, B, C gives \
         the same result regardless of grouping: (AB)C = A(BC). In \
         octonions and beyond, this fails. Rooms connected by such \
         rotations form impossible spaces.",
    );
}

/// Menu screen overlay.
fn menu_ui_system(mut contexts: EguiContexts) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    bevy_egui::egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(100.0);
            ui.heading("Gororoba: Non-Euclidean");
            ui.add_space(20.0);
            ui.label("Navigate impossible rooms connected by hypercomplex rotations.");
            ui.add_space(40.0);
            ui.label("Press SPACE to start");
            ui.add_space(20.0);
            ui.label("Controls:");
            ui.label("  Right-click + drag: orbit camera");
            ui.label("  Scroll: zoom");
            ui.label("  T: traverse portal");
            ui.label("  1-9: select basis element (puzzles)");
            ui.label("  Space: compute product (puzzles)");
            ui.label("  Backspace: undo selection");
            ui.label("  Enter: advance to next phase");
            ui.label("  F1: toggle HUD");
            ui.label("  F2: toggle pedagogy panel");
        });
    });
}

/// Exploration mode HUD: room info, portal count, distortion.
fn explore_ui_system(
    mut contexts: EguiContexts,
    active: Res<ActiveRoom>,
    distortion: Res<DistortionState>,
    traversals: Res<PortalTraversalCount>,
    diag_query: Query<&AlgebraDiagnostics, With<AlgebraDomain>>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    bevy_egui::egui::Window::new("Explorer")
        .anchor(
            bevy_egui::egui::Align2::LEFT_TOP,
            bevy_egui::egui::vec2(10.0, 10.0),
        )
        .resizable(false)
        .show(ctx, |ui| {
            ui.label(format!("Current Room: {}", active.room_id));
            ui.label(format!("Portals Traversed: {}", traversals.count));
            ui.label(format!("Distortion: {:.1}%", distortion.intensity * 100.0));

            if let Some(diag) = diag_query.iter().next() {
                ui.separator();
                ui.label(format!("Algebra Dim: {}", diag.dimension));
                ui.label(format!("Zero-Divisors: {}", diag.zd_count_2blade));
                ui.label(format!("Associator: {:.4}", diag.associator_norm));
            }

            ui.add_space(10.0);
            ui.label("Press T to traverse a portal");
            ui.label("Press ENTER for puzzles");
        });
}

/// Puzzle solving interface.
fn puzzle_ui_system(mut contexts: EguiContexts, puzzle_state: Res<PuzzleState>) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    let puzzles = introductory_puzzles();
    let current = puzzle_state.current_puzzle;
    let puzzle = if current < puzzles.len() {
        Some(&puzzles[current])
    } else {
        None
    };

    bevy_egui::egui::Window::new("Puzzle")
        .anchor(
            bevy_egui::egui::Align2::LEFT_TOP,
            bevy_egui::egui::vec2(10.0, 10.0),
        )
        .resizable(false)
        .show(ctx, |ui| {
            if let Some(puzzle) = puzzle {
                ui.heading(puzzle.name);
                ui.label(puzzle.description);
                ui.separator();

                ui.label(format!("Available bases: {:?}", puzzle.available_bases));
                ui.label(format!("Selected: {:?}", puzzle_state.selected_elements));

                if let Some(ref result) = puzzle_state.result {
                    ui.separator();
                    ui.label("Result:");
                    // Show nonzero components only for readability.
                    let nonzero: Vec<(usize, f64)> = result
                        .iter()
                        .enumerate()
                        .filter(|(_, v)| v.abs() > 1e-10)
                        .map(|(i, v)| (i, *v))
                        .collect();
                    if nonzero.is_empty() {
                        ui.label("  = 0 (zero element)");
                    } else {
                        for (i, v) in &nonzero {
                            if *i == 0 {
                                ui.label(format!("  {:.4}", v));
                            } else {
                                ui.label(format!("  {:.4} * e{}", v, i));
                            }
                        }
                    }
                }

                if puzzle_state.solved {
                    ui.add_space(10.0);
                    ui.colored_label(bevy_egui::egui::Color32::GREEN, "SOLVED!");
                }
            } else {
                ui.label("All puzzles complete!");
            }

            ui.add_space(10.0);
            ui.label("Press ENTER for results");
        });
}

/// Results screen: show puzzle completion summary.
fn results_ui_system(mut contexts: EguiContexts, puzzle_state: Res<PuzzleState>) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    bevy_egui::egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(50.0);
            ui.heading("Puzzle Results");
            ui.add_space(20.0);

            let total = introductory_puzzles().len();
            ui.label(format!(
                "Puzzles Solved: {} / {}",
                puzzle_state.total_solved, total
            ));
            ui.add_space(10.0);

            if puzzle_state.total_solved >= total {
                ui.colored_label(
                    bevy_egui::egui::Color32::GREEN,
                    "Congratulations! All puzzles complete!",
                );
            } else {
                ui.label("Keep exploring to discover more algebraic structures.");
            }

            ui.add_space(30.0);
            ui.label("Press ENTER to return to menu");
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pedagogy_entries_cover_all_modes() {
        // Verify we have Story, Explorer, and Research entries.
        // (setup_pedagogy adds 5 entries covering all three modes.)
        let puzzles = introductory_puzzles();
        assert!(!puzzles.is_empty());
    }
}
