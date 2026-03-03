// Game-specific UI: menu, observation HUD, navigation instruments,
// results screen, and pedagogy content.

use bevy::prelude::*;
use bevy_egui::EguiContexts;

use gororoba_bevy_core::{PedagogyMode, PedagogyState};
use gororoba_bevy_gr::{GrDiagnostics, GrEngine, SpacetimeDomain};

use crate::celestial::{CelestialConfig, MissionResults};
use crate::spacecraft::TimeDilationDisplay;
use crate::states::{SpaceGamePhase, SpaceSimState};

/// Plugin for game-specific UI systems.
pub struct SpaceUiPlugin;

impl Plugin for SpaceUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_pedagogy)
            .add_systems(
                Update,
                menu_ui_system.run_if(in_state(SpaceGamePhase::Menu)),
            )
            .add_systems(
                Update,
                observe_ui_system.run_if(in_state(SpaceSimState::Observing)),
            )
            .add_systems(
                Update,
                navigate_ui_system.run_if(in_state(SpaceSimState::Navigating)),
            )
            .add_systems(
                Update,
                results_ui_system.run_if(in_state(SpaceSimState::Results)),
            );
    }
}

/// Register pedagogy content about general relativity.
fn setup_pedagogy(mut pedagogy: ResMut<PedagogyState>) {
    pedagogy.add(
        PedagogyMode::Story,
        "Black Hole Explorer",
        "Navigate near a black hole and experience how spacetime \
         curves around massive objects. Watch light bend, time slow \
         down, and shadows form.",
    );
    pedagogy.add(
        PedagogyMode::Explorer,
        "Schwarzschild Metric",
        "The simplest black hole: non-rotating, described by mass M only. \
         ds^2 = -(1-2M/r)dt^2 + (1-2M/r)^{-1}dr^2 + r^2 dOmega^2. \
         Event horizon at r = 2M. ISCO at r = 6M.",
    );
    pedagogy.add(
        PedagogyMode::Explorer,
        "Time Dilation",
        "Clocks run slower in stronger gravitational fields. \
         Near a black hole, dtau/dt = sqrt(1 - 2M/r). At the event \
         horizon, time stops from the perspective of a distant observer.",
    );
    pedagogy.add(
        PedagogyMode::Research,
        "Kerr Metric",
        "A rotating black hole, described by mass M and spin a = J/M. \
         Frame-dragging forces spacetime to rotate with the hole. \
         The shadow becomes asymmetric: flattened on the prograde side.",
    );
    pedagogy.add(
        PedagogyMode::Research,
        "Black Hole Shadow",
        "The shadow is the region of the sky from which no photons can \
         escape to a distant observer. Its boundary traces unstable \
         photon orbits. Shape depends on spin and observer inclination.",
    );
}

fn menu_ui_system(mut contexts: EguiContexts) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    bevy_egui::egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(100.0);
            ui.heading("Gororoba: Relativistic Space");
            ui.add_space(20.0);
            ui.label("Explore curved spacetime around a black hole.");
            ui.add_space(40.0);
            ui.label("Press SPACE to start");
            ui.add_space(20.0);
            ui.label("Controls:");
            ui.label("  Right-click + drag: orbit camera");
            ui.label("  Scroll: zoom");
            ui.label("  W/S: thrust radially (inward/outward)");
            ui.label("  A/D: thrust tangentially");
            ui.label("  Enter: advance to next phase");
            ui.label("  F1: toggle HUD");
            ui.label("  F2: toggle pedagogy panel");
        });
    });
}

/// Observation mode HUD: black hole info, shadow, diagnostics.
fn observe_ui_system(
    mut contexts: EguiContexts,
    config: Res<CelestialConfig>,
    engine: Res<GrEngine>,
    domain: Query<Entity, With<SpacetimeDomain>>,
    diag_query: Query<&GrDiagnostics, With<SpacetimeDomain>>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    bevy_egui::egui::Window::new("Black Hole Observer")
        .anchor(
            bevy_egui::egui::Align2::LEFT_TOP,
            bevy_egui::egui::vec2(10.0, 10.0),
        )
        .resizable(false)
        .show(ctx, |ui| {
            ui.label(format!("Mass: {:.2} M", config.mass));
            ui.label(format!("Spin: {:.3}", config.spin));
            ui.label(format!(
                "Observer Distance: {:.1} M",
                config.observer_distance
            ));
            ui.label(format!("Shadow Resolution: {} pts", config.shadow_points));

            if let Some(entity) = domain.iter().next() {
                if let Some(inst) = engine.get(entity) {
                    ui.separator();
                    ui.label(format!("Event Horizon: {:.3} M", inst.event_horizon()));
                    ui.label(format!("ISCO: {:.3} M", inst.isco_radius()));
                    ui.label(format!("Shadow Points: {}", inst.shadow_alpha.len()));
                }

                if let Ok(diag) = diag_query.get(entity) {
                    ui.separator();
                    ui.label(format!("Time Dilation: {:.4}", diag.time_dilation));
                    ui.label(format!("Active Geodesics: {}", diag.active_geodesics));
                }
            }

            ui.separator();
            ui.heading("Parameters");

            ui.add_space(10.0);
            ui.label("Press ENTER to navigate");
        });
}

/// Navigation mode HUD: spacecraft instruments.
fn navigate_ui_system(mut contexts: EguiContexts, dilation: Res<TimeDilationDisplay>) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    bevy_egui::egui::Window::new("Navigation")
        .anchor(
            bevy_egui::egui::Align2::LEFT_TOP,
            bevy_egui::egui::vec2(10.0, 10.0),
        )
        .resizable(false)
        .show(ctx, |ui| {
            ui.heading("Time Dilation");
            ui.label(format!("Factor: {:.4}", dilation.factor));
            ui.label(format!("Proper Time: {:.2}", dilation.proper_time));
            ui.label(format!("Coordinate Time: {:.2}", dilation.coordinate_time));

            let age_ratio = if dilation.coordinate_time > 0.0 {
                dilation.proper_time / dilation.coordinate_time
            } else {
                1.0
            };
            ui.label(format!(
                "You age at {:.1}% of distant observer",
                age_ratio * 100.0
            ));

            ui.add_space(10.0);
            ui.label("W/S: radial thrust");
            ui.label("A/D: angular thrust");
            ui.label("Press ENTER for results");
        });
}

/// Results screen.
fn results_ui_system(mut contexts: EguiContexts, results: Res<MissionResults>) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    bevy_egui::egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(50.0);
            ui.heading("Mission Results");
            ui.add_space(20.0);

            bevy_egui::egui::Grid::new("results_grid")
                .num_columns(2)
                .spacing([40.0, 8.0])
                .show(ui, |ui| {
                    ui.label("Proper Time Elapsed:");
                    ui.label(format!("{:.2}", results.proper_time_elapsed));
                    ui.end_row();

                    ui.label("Coordinate Time Elapsed:");
                    ui.label(format!("{:.2}", results.coordinate_time_elapsed));
                    ui.end_row();

                    ui.label("Closest Approach:");
                    ui.label(format!("{:.2} M", results.min_approach_radius));
                    ui.end_row();

                    ui.label("Geodesics Traced:");
                    ui.label(format!("{}", results.geodesics_traced));
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
