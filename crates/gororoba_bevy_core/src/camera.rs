// Orbit and fly camera systems.
//
// OrbitCameraPlugin: click-drag orbit around a focal point.
// FlyCameraPlugin: WASD + mouse free-fly navigation.

use bevy::prelude::*;

pub struct OrbitCameraPlugin;

impl Plugin for OrbitCameraPlugin {
    fn build(&self, _app: &mut App) {
        // Phase 1: implement orbit camera systems.
    }
}

pub struct FlyCameraPlugin;

impl Plugin for FlyCameraPlugin {
    fn build(&self, _app: &mut App) {
        // Phase 1: implement fly camera systems.
    }
}
