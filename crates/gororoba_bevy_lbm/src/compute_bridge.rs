// ash-to-Bevy texture bridge.
//
// Transfers rendered frames from lbm_vulkan's ash Vulkan compute pipeline
// to Bevy's wgpu renderer via CPU readback.
//
// Architecture:
// 1. GororobaEngine runs LBM + volume render on ash Vulkan compute pipeline.
// 2. After each step, read_render_pixels() copies RGBA from GPU to CPU buffer.
// 3. This system uploads the CPU buffer to a Bevy Image asset every frame.
// 4. A fullscreen quad or Bevy sprite displays the Image.
//
// The GPU bridge is not yet active because lbm_vulkan needs additive API
// changes (read_render_pixels returning Vec<u8>). For now, the CPU solver
// (lbm_3d) provides all simulation data and Bevy renders directly from
// density/velocity fields.

use bevy::prelude::*;

/// Configuration for the GPU compute bridge.
///
/// When active, the bridge transfers rendered frames from lbm_vulkan's
/// ash Vulkan pipeline to a Bevy Image for display.
#[derive(Resource)]
pub struct GpuBridgeConfig {
    /// Width of the render target in pixels.
    pub width: u32,
    /// Height of the render target in pixels.
    pub height: u32,
    /// Whether the GPU bridge is active. When false, CPU rendering is used.
    pub active: bool,
}

impl Default for GpuBridgeConfig {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 720,
            active: false,
        }
    }
}

/// Handle to the Bevy Image that receives GPU readback frames.
#[derive(Resource, Default)]
pub struct GpuFrameTarget {
    pub image: Option<Handle<Image>>,
}

/// System that uploads GPU readback pixels to a Bevy Image.
///
/// Currently a no-op because the GPU bridge is not yet active.
/// When lbm_vulkan exposes read_render_pixels() -> Vec<u8>, this
/// system will copy that buffer into the GpuFrameTarget image.
pub fn gpu_readback_system(
    bridge: Res<GpuBridgeConfig>,
    _target: ResMut<GpuFrameTarget>,
    _images: ResMut<Assets<Image>>,
) {
    if bridge.active {
        // Future implementation:
        // 1. let pixels = gpu_engine.read_render_pixels()?;
        // 2. let image = images.get_mut(target.image)?;
        // 3. image.data = pixels;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_bridge_default_inactive() {
        let config = GpuBridgeConfig::default();
        assert!(!config.active);
        assert_eq!(config.width, 1280);
        assert_eq!(config.height, 720);
    }
}
