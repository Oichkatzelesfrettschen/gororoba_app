// Game UI: level select, arena visualization, constraints panel,
// payoff matrix, and results screen.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass};

use gororoba_bevy_core::{EguiReady, PedagogyMode, PedagogyState};
use gororoba_bevy_game_semantics::{GameSemanticsEngine, Polarity};

use crate::states::{InteractionArenaPhase, SimState};

pub struct InteractionUiPlugin;

impl Plugin for InteractionUiPlugin {
    fn build(&self, app: &mut App) {
        let egui_ready = resource_exists::<EguiReady>;
        app.add_systems(Startup, setup_pedagogy)
            .add_systems(
                EguiPrimaryContextPass,
                menu_ui_system
                    .run_if(in_state(InteractionArenaPhase::Menu))
                    .run_if(egui_ready),
            )
            .add_systems(
                EguiPrimaryContextPass,
                arena_view_ui_system
                    .run_if(in_state(SimState::ArenaView))
                    .run_if(egui_ready),
            )
            .add_systems(
                EguiPrimaryContextPass,
                builder_ui_system
                    .run_if(in_state(SimState::StrategyBuilder))
                    .run_if(egui_ready),
            )
            .add_systems(
                EguiPrimaryContextPass,
                execution_ui_system
                    .run_if(in_state(SimState::Execution))
                    .run_if(egui_ready),
            )
            .add_systems(
                EguiPrimaryContextPass,
                results_ui_system
                    .run_if(in_state(SimState::Results))
                    .run_if(egui_ready),
            );
    }
}

fn setup_pedagogy(mut pedagogy: ResMut<PedagogyState>) {
    pedagogy.add(
        PedagogyMode::Story,
        "Arenas & Moves",
        "Programs are two-player games. Player (+blue) is the program; \
         Opponent (-red) is the environment. Moves are questions and answers. \
         A function call is a question; its return value is an answer.",
    );
    pedagogy.add(
        PedagogyMode::Story,
        "Strategies",
        "A strategy is a consistent plan: for every Opponent question, \
         have a Player answer ready. Strategies must respect causality \
         (answer only what's been asked) and alternate turns.",
    );
    pedagogy.add(
        PedagogyMode::Explorer,
        "Well-Bracketing",
        "Well-bracketing = call/return discipline. Answer the most recent \
         unanswered question before starting a new one. Like a stack: LIFO. \
         Languages like PCF enforce this; IA with interference does not.",
    );
    pedagogy.add(
        PedagogyMode::Explorer,
        "Innocence",
        "An innocent strategy depends only on the P-view: the visible \
         history from Player's perspective. Hidden Opponent moves cannot \
         influence decisions. This models pure, side-effect-free programs.",
    );
    pedagogy.add(
        PedagogyMode::Explorer,
        "Composition",
        "Strategies compose like functions: given sigma: A->B and tau: B->C, \
         their composition sigma;tau: A->C hides the internal B moves. \
         The result behaves as if A directly connects to C.",
    );
    pedagogy.add(
        PedagogyMode::Research,
        "Payoffs & Equilibria",
        "Classical game theory scores strategies via payoff matrices. \
         Nash equilibrium: no player benefits from unilateral deviation. \
         Pareto optimal: no outcome improves both players simultaneously. \
         Minimax: guarantee the best worst-case payoff.",
    );
    pedagogy.add(
        PedagogyMode::Research,
        "The Semantic Cube",
        "Abramsky's semantic cube: toggle innocence, well-bracketing, \
         and sequentiality to capture different languages.\n\
         PCF = innocent + well-bracketed + sequential\n\
         IA = well-bracketed + sequential (drops innocence)\n\
         IA// = innocent + well-bracketed (drops sequentiality)\n\
         Each language sees a different set of valid strategies.",
    );
}

fn menu_ui_system(mut contexts: EguiContexts, engine: Res<GameSemanticsEngine>) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    if !ctx.content_rect().is_finite() {
        return;
    }

    bevy_egui::egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(80.0);
            ui.heading("Gororoba: Interaction Arena");
            ui.add_space(10.0);
            ui.label("Game Semantics meets Classical Game Theory");
            ui.add_space(30.0);

            // Level select list.
            ui.label("Levels:");
            ui.add_space(5.0);
            for (i, level) in engine.levels.iter().enumerate() {
                let completed = engine.levels_completed.get(i).copied().unwrap_or(false);
                let marker = if completed { "[x]" } else { "[ ]" };
                ui.label(format!(
                    "  {} {}. {} -- {}",
                    marker, level.number, level.name, level.concept
                ));
            }

            ui.add_space(30.0);
            ui.label("Press SPACE to start Level 1");
            ui.add_space(10.0);
            ui.label("Controls:");
            ui.label("  1-9: place move (strategy builder)");
            ui.label("  Backspace: undo move");
            ui.label("  Enter: advance phase");
            ui.label("  F1: toggle HUD | F2: toggle pedagogy");
        });
    });
}

fn arena_view_ui_system(mut contexts: EguiContexts, engine: Res<GameSemanticsEngine>) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    if !ctx.content_rect().is_finite() {
        return;
    }

    let Some(level) = engine.current_level_def() else {
        return;
    };

    bevy_egui::egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(30.0);
            ui.heading(format!("Level {}: {}", level.number, level.name));
            ui.add_space(10.0);
            ui.label(level.description);
            ui.add_space(20.0);

            // Show arena structure.
            ui.heading("Arena");
            ui.add_space(5.0);
            for event in &level.arena.events {
                let pol = match event.polarity {
                    Polarity::Player => "+P",
                    Polarity::Opponent => "-O",
                };
                let kind = event.kind.symbol();
                let justifier = event
                    .justifier
                    .map(|j| format!(" (justified by {})", j))
                    .unwrap_or_default();
                ui.label(format!(
                    "  [{}] {} {}: {}{}",
                    event.id, pol, kind, event.label, justifier
                ));
            }

            if !level.arena.enabling.is_empty() {
                ui.add_space(10.0);
                ui.label("Enabling edges:");
                for (src, tgt) in &level.arena.enabling {
                    ui.label(format!("  {} --> {}", src, tgt));
                }
            }

            ui.add_space(10.0);
            ui.label(format!("Concept: {}", level.concept));
            ui.label(format!("Game Theory: {}", level.gt_element));

            // Active conditions.
            let conds = &level.conditions;
            if conds.well_bracketing
                || conds.innocence
                || conds.parallel_innocence
                || conds.sequentiality
            {
                ui.add_space(10.0);
                ui.label("Active constraints:");
                if conds.well_bracketing {
                    ui.label("  - Well-Bracketing");
                }
                if conds.innocence {
                    ui.label("  - Innocence");
                }
                if conds.parallel_innocence {
                    ui.label("  - Parallel Innocence");
                }
                if conds.sequentiality {
                    ui.label("  - Sequentiality");
                }
            }

            ui.add_space(20.0);
            ui.label("Press ENTER to start building your strategy");
        });
    });
}

fn builder_ui_system(mut contexts: EguiContexts, engine: Res<GameSemanticsEngine>) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    if !ctx.content_rect().is_finite() {
        return;
    }

    let Some(level) = engine.current_level_def() else {
        return;
    };

    // Strategy builder: left panel for moves, right panel for constraints.
    bevy_egui::egui::SidePanel::left("builder_panel")
        .min_width(300.0)
        .show(ctx, |ui| {
            ui.heading(format!("Level {}: Strategy Builder", level.number));
            ui.add_space(10.0);

            // Show current moves.
            if let Some(strategy) = &engine.strategy {
                ui.label("Current play:");
                for (i, &m) in strategy.moves.iter().enumerate() {
                    if m < strategy.arena.events.len() {
                        let e = &strategy.arena.events[m];
                        let pol = match e.polarity {
                            Polarity::Player => "+",
                            Polarity::Opponent => "-",
                        };
                        ui.label(format!(
                            "  {}. {} {} ({})",
                            i + 1,
                            pol,
                            e.label,
                            e.kind.symbol()
                        ));
                    }
                }

                ui.add_space(10.0);
                ui.label("Available Player moves:");
                let available = engine.available_moves();
                let mut idx = 1;
                for &m in &available {
                    if m < strategy.arena.events.len()
                        && strategy.arena.events[m].polarity == Polarity::Player
                    {
                        let e = &strategy.arena.events[m];
                        ui.label(format!(
                            "  Press {}: {} ({})",
                            idx,
                            e.label,
                            e.kind.symbol()
                        ));
                        idx += 1;
                    }
                }
                if idx == 1 {
                    ui.label("  (no moves available)");
                }
            }

            ui.add_space(10.0);
            ui.label("Backspace: undo | Enter: submit strategy");
        });

    // Constraints panel on the right.
    bevy_egui::egui::SidePanel::right("constraints_panel")
        .min_width(250.0)
        .show(ctx, |ui| {
            ui.heading("Constraints");
            ui.add_space(10.0);

            for result in &engine.condition_results {
                let color = if result.satisfied {
                    bevy_egui::egui::Color32::from_rgb(80, 200, 80)
                } else {
                    bevy_egui::egui::Color32::from_rgb(200, 80, 80)
                };
                let icon = if result.satisfied { "[OK]" } else { "[!!]" };
                ui.colored_label(color, format!("{} {}", icon, result.name));
                ui.label(format!("  {}", result.detail));
                ui.add_space(5.0);
            }

            if engine.condition_results.is_empty() {
                ui.label("No constraints active for this level.");
            }
        });
}

fn execution_ui_system(mut contexts: EguiContexts, engine: Res<GameSemanticsEngine>) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    if !ctx.content_rect().is_finite() {
        return;
    }

    let Some(level) = engine.current_level_def() else {
        return;
    };

    bevy_egui::egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(30.0);
            ui.heading(format!("Level {}: Execution", level.number));
            ui.add_space(10.0);

            // Show strategy execution step by step.
            if let Some(strategy) = &engine.strategy {
                let total = strategy.moves.len();
                let step = engine.execution_step;

                ui.label(format!("Step {} / {}", step, total));
                ui.add_space(10.0);

                for (i, &m) in strategy.moves.iter().enumerate() {
                    if m >= strategy.arena.events.len() {
                        continue;
                    }
                    let e = &strategy.arena.events[m];
                    let pol = match e.polarity {
                        Polarity::Player => "+",
                        Polarity::Opponent => "-",
                    };
                    let highlight = if i < step {
                        bevy_egui::egui::Color32::WHITE
                    } else if i == step {
                        bevy_egui::egui::Color32::YELLOW
                    } else {
                        bevy_egui::egui::Color32::DARK_GRAY
                    };
                    ui.colored_label(
                        highlight,
                        format!("  {}. {} {} ({})", i + 1, pol, e.label, e.kind.symbol()),
                    );
                }

                ui.add_space(20.0);
                ui.label(format!("Payoff: {:.1}", engine.accumulated_payoff));
            }

            ui.add_space(20.0);
            ui.label("Press ENTER to see results");
        });
    });
}

fn results_ui_system(mut contexts: EguiContexts, engine: Res<GameSemanticsEngine>) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    if !ctx.content_rect().is_finite() {
        return;
    }

    let Some(level) = engine.current_level_def() else {
        return;
    };

    bevy_egui::egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(30.0);
            ui.heading(format!("Level {}: Results", level.number));
            ui.add_space(20.0);

            // Score.
            ui.label(format!("Level Score: {:.1} / 100.0", engine.level_score));
            ui.label(format!("Total Score: {:.1}", engine.total_score));
            ui.add_space(15.0);

            // Constraint results.
            ui.heading("Constraint Report");
            if engine.condition_results.is_empty() {
                ui.label("No constraints were active.");
            } else {
                for result in &engine.condition_results {
                    let status = if result.satisfied {
                        "SATISFIED"
                    } else {
                        "VIOLATED"
                    };
                    ui.label(format!("  {} -- {}", result.name, status));
                }
            }
            ui.add_space(15.0);

            // Payoff matrix.
            ui.heading("Payoff Matrix");
            let pm = &level.payoff_matrix;

            // Header row.
            let mut header = String::from("         ");
            for col in &pm.cols {
                header.push_str(&format!("{:>10}", col));
            }
            ui.label(&header);

            // Data rows.
            for (i, row_label) in pm.rows.iter().enumerate() {
                let mut row_str = format!("{:<9}", row_label);
                for j in 0..pm.cols.len() {
                    row_str.push_str(&format!("{:>10.1}", pm.values[i][j]));
                }
                ui.label(&row_str);
            }
            ui.add_space(10.0);

            // Nash equilibria.
            ui.heading("Nash Equilibria");
            if engine.nash_results.is_empty() {
                ui.label("No equilibria found for this game.");
            } else {
                for (i, nash) in engine.nash_results.iter().enumerate() {
                    ui.label(format!(
                        "  Eq {}: value = {:.2}, P = {:?}, O = {:?}",
                        i + 1,
                        nash.value,
                        nash.player_strategy,
                        nash.opponent_strategy
                    ));
                }
            }
            ui.add_space(10.0);

            // Minimax.
            ui.label(format!("Minimax value: {:.1}", pm.minimax_value()));
            ui.label(format!("Maximin value: {:.1}", pm.maximin_value()));

            ui.add_space(20.0);
            ui.label("Press ENTER to return to menu");
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_plugin_constructs() {
        let _ = InteractionUiPlugin;
    }
}
