// Story / Explorer / Research pedagogy panels.
//
// Each game registers domain-specific content; this module provides the
// panel framework and progressive disclosure logic.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPlugin, EguiPrimaryContextPass};

pub struct PedagogyPlugin;

impl Plugin for PedagogyPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<EguiPlugin>() {
            app.add_plugins(EguiPlugin::default());
        }
        app.init_resource::<PedagogyState>().add_systems(
            EguiPrimaryContextPass,
            pedagogy_panel_system.run_if(resource_exists::<crate::EguiReady>),
        );
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PedagogyMode {
    #[default]
    Story,
    Explorer,
    Research,
}

/// Pedagogy panel state. Games register entries for each mode.
#[derive(Resource, Default)]
pub struct PedagogyState {
    pub visible: bool,
    pub mode: PedagogyMode,
    pub entries: Vec<PedagogyEntry>,
}

pub struct PedagogyEntry {
    pub mode: PedagogyMode,
    pub title: String,
    pub body: String,
}

impl PedagogyState {
    pub fn add(&mut self, mode: PedagogyMode, title: impl Into<String>, body: impl Into<String>) {
        self.entries.push(PedagogyEntry {
            mode,
            title: title.into(),
            body: body.into(),
        });
    }

    pub fn active_entries(&self) -> impl Iterator<Item = &PedagogyEntry> {
        self.entries.iter().filter(|e| e.mode == self.mode)
    }
}

fn pedagogy_panel_system(mut contexts: EguiContexts, mut state: ResMut<PedagogyState>) {
    if !state.visible {
        return;
    }

    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    if !ctx.content_rect().is_finite() {
        return;
    }

    bevy_egui::egui::SidePanel::left("pedagogy_panel")
        .default_width(300.0)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(state.mode == PedagogyMode::Story, "Story")
                    .clicked()
                {
                    state.mode = PedagogyMode::Story;
                }
                if ui
                    .selectable_label(state.mode == PedagogyMode::Explorer, "Explorer")
                    .clicked()
                {
                    state.mode = PedagogyMode::Explorer;
                }
                if ui
                    .selectable_label(state.mode == PedagogyMode::Research, "Research")
                    .clicked()
                {
                    state.mode = PedagogyMode::Research;
                }
            });

            ui.separator();

            let mode = state.mode;
            for entry in state.entries.iter().filter(|e| e.mode == mode) {
                ui.heading(&entry.title);
                ui.label(&entry.body);
                ui.add_space(8.0);
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pedagogy_add_and_filter() {
        let mut state = PedagogyState::default();
        state.add(PedagogyMode::Story, "Intro", "Welcome to the game.");
        state.add(PedagogyMode::Explorer, "Controls", "WASD to move.");
        state.add(PedagogyMode::Research, "Equations", "Navier-Stokes.");

        state.mode = PedagogyMode::Story;
        assert_eq!(state.active_entries().count(), 1);

        state.mode = PedagogyMode::Research;
        let entries: Vec<_> = state.active_entries().collect();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].title, "Equations");
    }
}
