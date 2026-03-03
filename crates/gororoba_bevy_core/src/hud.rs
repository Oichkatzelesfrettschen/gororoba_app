// egui-based HUD overlay for runtime diagnostics and game UI.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPlugin};

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<EguiPlugin>() {
            app.add_plugins(EguiPlugin::default());
        }
        app.init_resource::<HudState>()
            .add_systems(Update, hud_ui_system);
    }
}

/// Game-specific HUD entries. Each game pushes domain stats here.
#[derive(Resource, Default)]
pub struct HudState {
    pub visible: bool,
    pub entries: Vec<HudEntry>,
}

pub struct HudEntry {
    pub label: String,
    pub value: String,
}

impl HudState {
    pub fn set(&mut self, label: impl Into<String>, value: impl Into<String>) {
        let label = label.into();
        let value = value.into();
        if let Some(entry) = self.entries.iter_mut().find(|e| e.label == label) {
            entry.value = value;
        } else {
            self.entries.push(HudEntry { label, value });
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

fn hud_ui_system(mut contexts: EguiContexts, hud: Res<HudState>) {
    if !hud.visible {
        return;
    }

    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    bevy_egui::egui::Window::new("HUD")
        .anchor(
            bevy_egui::egui::Align2::RIGHT_TOP,
            bevy_egui::egui::vec2(-10.0, 10.0),
        )
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            bevy_egui::egui::Grid::new("hud_grid")
                .num_columns(2)
                .spacing([20.0, 4.0])
                .show(ui, |ui| {
                    for entry in &hud.entries {
                        ui.label(&entry.label);
                        ui.label(&entry.value);
                        ui.end_row();
                    }
                });
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hud_state_set_and_update() {
        let mut hud = HudState::default();
        hud.set("FPS", "60");
        assert_eq!(hud.entries.len(), 1);
        assert_eq!(hud.entries[0].value, "60");

        // Update existing entry.
        hud.set("FPS", "120");
        assert_eq!(hud.entries.len(), 1);
        assert_eq!(hud.entries[0].value, "120");

        // Add a different entry.
        hud.set("MLUPS", "42");
        assert_eq!(hud.entries.len(), 2);
    }

    #[test]
    fn hud_state_clear() {
        let mut hud = HudState::default();
        hud.set("A", "1");
        hud.set("B", "2");
        hud.clear();
        assert!(hud.entries.is_empty());
    }
}
