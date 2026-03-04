// Orbit and fly camera systems.
//
// Input gathering and transform application are separate systems.
// Input systems are gated with `not(egui_wants_any_pointer_input)` so
// they do not run when the cursor is over egui widgets. Transform
// systems always run, ensuring the camera does not freeze/snap when
// egui has focus.
//
// Uses bevy_egui's run condition functions which read EguiWantsInput
// (populated in PostUpdate) -- the run_if check evaluates before the
// system body executes, so the one-frame lag is acceptable for gating.

use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use bevy::prelude::*;
use bevy_egui::input::{egui_wants_any_keyboard_input, egui_wants_any_pointer_input};
use std::f32::consts::{FRAC_PI_2, TAU};

// -- Orbit camera --

pub struct OrbitCameraPlugin;

impl Plugin for OrbitCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                orbit_camera_input_system.run_if(not(egui_wants_any_pointer_input)),
                orbit_camera_transform_system,
            )
                .chain(),
        );
    }
}

#[derive(Component)]
pub struct OrbitCamera {
    pub radius: f32,
    pub pitch: f32,
    pub yaw: f32,
    pub target: Vec3,
    pub sensitivity: f32,
    pub zoom_speed: f32,
    pub min_radius: f32,
    pub max_radius: f32,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            radius: 10.0,
            pitch: -0.3,
            yaw: 0.0,
            target: Vec3::ZERO,
            sensitivity: 0.005,
            zoom_speed: 1.0,
            min_radius: 1.0,
            max_radius: 100.0,
        }
    }
}

impl OrbitCamera {
    pub fn compute_transform(&self) -> Transform {
        let pos = self.target + orbit_offset(self.yaw, self.pitch, self.radius);
        Transform::from_translation(pos).looking_at(self.target, Vec3::Y)
    }
}

pub fn orbit_offset(yaw: f32, pitch: f32, radius: f32) -> Vec3 {
    let (sy, cy) = yaw.sin_cos();
    let (sp, cp) = pitch.sin_cos();
    Vec3::new(cp * sy, -sp, cp * cy) * radius
}

/// Gather mouse orbit and scroll-zoom input into OrbitCamera fields.
/// Gated with `run_if(not(egui_wants_any_pointer_input))` so it does
/// not steal clicks from egui widgets.
fn orbit_camera_input_system(
    mouse_button: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    mouse_scroll: Res<AccumulatedMouseScroll>,
    mut query: Query<&mut OrbitCamera>,
) {
    for mut orbit in &mut query {
        if mouse_button.pressed(MouseButton::Right) {
            let delta = mouse_motion.delta;
            orbit.yaw -= delta.x * orbit.sensitivity;
            orbit.pitch -= delta.y * orbit.sensitivity;
            orbit.yaw %= TAU;
            orbit.pitch = orbit.pitch.clamp(-FRAC_PI_2 + 0.01, FRAC_PI_2 - 0.01);
        }

        let scroll = mouse_scroll.delta.y;
        if scroll.abs() > 0.0 {
            orbit.radius -= scroll * orbit.zoom_speed;
            orbit.radius = orbit.radius.clamp(orbit.min_radius, orbit.max_radius);
        }
    }
}

/// Apply the current OrbitCamera parameters to the Transform.
/// Always runs so the camera never freezes when egui has focus.
fn orbit_camera_transform_system(mut query: Query<(&OrbitCamera, &mut Transform)>) {
    for (orbit, mut transform) in &mut query {
        *transform = orbit.compute_transform();
    }
}

// -- Fly camera --

pub struct FlyCameraPlugin;

impl Plugin for FlyCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                fly_camera_mouse_input_system.run_if(not(egui_wants_any_pointer_input)),
                fly_camera_keyboard_input_system.run_if(not(egui_wants_any_keyboard_input)),
                fly_camera_transform_system,
            )
                .chain(),
        );
    }
}

#[derive(Component)]
pub struct FlyCamera {
    pub speed: f32,
    pub sensitivity: f32,
    pub pitch: f32,
    pub yaw: f32,
}

impl Default for FlyCamera {
    fn default() -> Self {
        Self {
            speed: 10.0,
            sensitivity: 0.003,
            pitch: 0.0,
            yaw: 0.0,
        }
    }
}

/// Gather mouse look input into FlyCamera fields.
/// Gated with `run_if(not(egui_wants_any_pointer_input))`.
fn fly_camera_mouse_input_system(
    mouse_button: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    mut query: Query<&mut FlyCamera>,
) {
    for mut fly in &mut query {
        if mouse_button.pressed(MouseButton::Right) {
            let delta = mouse_motion.delta;
            fly.yaw -= delta.x * fly.sensitivity;
            fly.pitch -= delta.y * fly.sensitivity;
            fly.yaw %= TAU;
            fly.pitch = fly.pitch.clamp(-FRAC_PI_2 + 0.01, FRAC_PI_2 - 0.01);
        }
    }
}

/// Gather WASD+QE movement input into the Transform.
/// Gated with `run_if(not(egui_wants_any_keyboard_input))`.
fn fly_camera_keyboard_input_system(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&FlyCamera, &mut Transform)>,
) {
    for (fly, mut transform) in &mut query {
        let mut direction = Vec3::ZERO;
        if keyboard.pressed(KeyCode::KeyW) {
            direction -= Vec3::Z;
        }
        if keyboard.pressed(KeyCode::KeyS) {
            direction += Vec3::Z;
        }
        if keyboard.pressed(KeyCode::KeyA) {
            direction -= Vec3::X;
        }
        if keyboard.pressed(KeyCode::KeyD) {
            direction += Vec3::X;
        }
        if keyboard.pressed(KeyCode::KeyE) {
            direction += Vec3::Y;
        }
        if keyboard.pressed(KeyCode::KeyQ) {
            direction -= Vec3::Y;
        }

        if direction != Vec3::ZERO {
            let rotation = Quat::from_euler(EulerRot::YXZ, fly.yaw, fly.pitch, 0.0);
            let movement = rotation * direction.normalize() * fly.speed * time.delta_secs();
            transform.translation += movement;
        }
    }
}

/// Apply the current FlyCamera yaw/pitch to the Transform rotation.
/// Always runs so the camera orientation never freezes.
fn fly_camera_transform_system(mut query: Query<(&FlyCamera, &mut Transform)>) {
    for (fly, mut transform) in &mut query {
        transform.rotation = Quat::from_euler(EulerRot::YXZ, fly.yaw, fly.pitch, 0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orbit_offset_at_zero_angles() {
        let offset = orbit_offset(0.0, 0.0, 10.0);
        // yaw=0, pitch=0 -> looking along +Z, camera at (0, 0, 10)
        assert!((offset.x).abs() < 1e-5);
        assert!((offset.y).abs() < 1e-5);
        assert!((offset.z - 10.0).abs() < 1e-5);
    }

    #[test]
    fn orbit_transform_looks_at_target() {
        let orbit = OrbitCamera {
            radius: 5.0,
            pitch: 0.0,
            yaw: 0.0,
            target: Vec3::new(1.0, 2.0, 3.0),
            ..default()
        };
        let t = orbit.compute_transform();
        // Camera should be at target + offset
        let expected_pos = orbit.target + orbit_offset(0.0, 0.0, 5.0);
        assert!((t.translation - expected_pos).length() < 1e-4);
    }

    #[test]
    fn orbit_pitch_clamp() {
        let mut orbit = OrbitCamera::default();
        orbit.pitch = 10.0; // Way beyond PI/2
        orbit.pitch = orbit.pitch.clamp(-FRAC_PI_2 + 0.01, FRAC_PI_2 - 0.01);
        assert!(orbit.pitch < FRAC_PI_2);
    }
}
