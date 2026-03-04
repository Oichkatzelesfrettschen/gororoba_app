// Minimal bevy_egui interactivity test.
// If this works, the bug is in our game code.
// If this fails, the bug is in bevy_egui 0.39.1 / Bevy 0.18.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPlugin, EguiPrimaryContextPass};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "egui interactivity test".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EguiPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(EguiPrimaryContextPass, ui_system)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera3d::default());
}

fn ui_system(
    mut contexts: EguiContexts,
    mut counter: Local<u32>,
    mut slider_val: Local<f32>,
    mut frame_count: Local<u32>,
) {
    *frame_count += 1;
    if *frame_count < 3 {
        return;
    }
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    bevy_egui::egui::Window::new("Interactivity Test").show(ctx, |ui| {
        if ui.button("Click Me").clicked() {
            *counter += 1;
            info!("Button clicked! Count: {}", *counter);
        }
        ui.label(format!("Click count: {}", *counter));

        ui.separator();

        ui.add(bevy_egui::egui::Slider::new(&mut *slider_val, 0.0..=100.0).text("Slider"));
        ui.label(format!("Slider value: {:.1}", *slider_val));

        ui.separator();

        let mut checkbox = (*counter).is_multiple_of(2);
        if ui.checkbox(&mut checkbox, "Toggle me").changed() {
            info!("Checkbox toggled: {checkbox}");
        }
    });
}
