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
// Activation: The GPU bridge is inactive by default. Set GpuBridgeConfig.active
// to true before the FluidDomain entity is spawned. The init system will
// attempt to create a VulkanContext and GororobaEngine. If Vulkan is
// unavailable (headless, no GPU), it falls back to CPU mode silently.

use bevy::image::Image;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};

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

/// Resource wrapping the lbm_vulkan GPU compute engine and its Vulkan
/// command submission state.
///
/// Created lazily when GpuBridgeConfig.active is true and Vulkan
/// is available. Manages its own command pool, buffer, and fence
/// for the simulation loop.
#[derive(Resource)]
pub struct GpuVulkanEngine {
    pub context: lbm_vulkan::VulkanContext,
    pub engine: lbm_vulkan::compute::GororobaEngine,
    cmd_pool: ash::vk::CommandPool,
    cmd_buffer: ash::vk::CommandBuffer,
    fence: ash::vk::Fence,
    frame_counter: u32,
}

impl GpuVulkanEngine {
    /// Create the GPU engine with all Vulkan command submission resources.
    fn new(
        context: lbm_vulkan::VulkanContext,
        engine: lbm_vulkan::compute::GororobaEngine,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let cmd_pool = unsafe {
            context.device.create_command_pool(
                &ash::vk::CommandPoolCreateInfo {
                    queue_family_index: context.queue_family_index,
                    flags: ash::vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
                    ..Default::default()
                },
                None,
            )
        }?;

        let cmd_buffer = unsafe {
            context
                .device
                .allocate_command_buffers(&ash::vk::CommandBufferAllocateInfo {
                    command_pool: cmd_pool,
                    level: ash::vk::CommandBufferLevel::PRIMARY,
                    command_buffer_count: 1,
                    ..Default::default()
                })
        }?[0];

        let fence = unsafe {
            context.device.create_fence(
                &ash::vk::FenceCreateInfo {
                    flags: ash::vk::FenceCreateFlags::SIGNALED,
                    ..Default::default()
                },
                None,
            )
        }?;

        Ok(Self {
            context,
            engine,
            cmd_pool,
            cmd_buffer,
            fence,
            frame_counter: 0,
        })
    }

    /// Submit one simulation step and wait for completion.
    fn step_and_wait(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            self.context
                .device
                .wait_for_fences(&[self.fence], true, u64::MAX)?;
            self.context.device.reset_fences(&[self.fence])?;
            self.context
                .device
                .reset_command_buffer(self.cmd_buffer, ash::vk::CommandBufferResetFlags::empty())?;
            self.context.device.begin_command_buffer(
                self.cmd_buffer,
                &ash::vk::CommandBufferBeginInfo {
                    flags: ash::vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
                    ..Default::default()
                },
            )?;
            self.engine.step(self.cmd_buffer, self.frame_counter)?;
            self.context.device.end_command_buffer(self.cmd_buffer)?;
            self.context.device.queue_submit(
                self.context.queue,
                &[ash::vk::SubmitInfo {
                    command_buffer_count: 1,
                    p_command_buffers: &self.cmd_buffer,
                    ..Default::default()
                }],
                self.fence,
            )?;
            self.context
                .device
                .wait_for_fences(&[self.fence], true, u64::MAX)?;
        }
        self.frame_counter = self.frame_counter.wrapping_add(1);
        Ok(())
    }
}

impl Drop for GpuVulkanEngine {
    fn drop(&mut self) {
        unsafe {
            let _ = self.context.device.device_wait_idle();
            self.context
                .device
                .destroy_command_pool(self.cmd_pool, None);
            self.context.device.destroy_fence(self.fence, None);
        }
    }
}

/// Initialize the GPU bridge: create VulkanContext, GororobaEngine, and
/// the target Bevy Image. Runs once when activated.
pub fn gpu_bridge_init_system(
    mut commands: Commands,
    bridge: Res<GpuBridgeConfig>,
    existing_engine: Option<Res<GpuVulkanEngine>>,
    mut target: ResMut<GpuFrameTarget>,
    mut images: ResMut<Assets<Image>>,
) {
    if !bridge.active || existing_engine.is_some() || target.image.is_some() {
        return;
    }

    // Attempt Vulkan context creation (no validation layers in game mode).
    let context = match lbm_vulkan::VulkanContext::new(false) {
        Ok(ctx) => ctx,
        Err(e) => {
            warn!("GPU bridge: Vulkan unavailable, falling back to CPU: {e}");
            return;
        }
    };

    let params = context.get_scaling_parameters();
    let engine = match lbm_vulkan::compute::GororobaEngine::new(
        &context,
        params.grid_dim,
        (bridge.width, bridge.height),
        lbm_vulkan::Precision::FP32,
    ) {
        Ok(eng) => eng,
        Err(e) => {
            warn!("GPU bridge: GororobaEngine creation failed: {e}");
            return;
        }
    };

    let gpu = match GpuVulkanEngine::new(context, engine) {
        Ok(g) => g,
        Err(e) => {
            warn!("GPU bridge: command buffer setup failed: {e}");
            return;
        }
    };

    // Create the Bevy Image that will receive readback pixels.
    let mut image = Image::new_fill(
        Extent3d {
            width: bridge.width,
            height: bridge.height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::all(),
    );
    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::STORAGE_BINDING;

    let handle = images.add(image);
    target.image = Some(handle);

    commands.insert_resource(gpu);
    info!(
        "GPU bridge initialized: {}x{}, grid {:?}",
        bridge.width, bridge.height, params.grid_dim
    );
}

/// Step the GPU LBM simulation and read back the rendered frame.
///
/// Copies the RGBA pixel buffer from GororobaEngine into the Bevy Image.
pub fn gpu_readback_system(
    bridge: Res<GpuBridgeConfig>,
    mut gpu_engine: Option<ResMut<GpuVulkanEngine>>,
    target: Res<GpuFrameTarget>,
    mut images: ResMut<Assets<Image>>,
) {
    if !bridge.active {
        return;
    }

    let Some(ref mut gpu) = gpu_engine else {
        return;
    };
    let Some(ref handle) = target.image else {
        return;
    };
    let Some(image) = images.get_mut(handle) else {
        return;
    };

    // Step the GPU simulation and wait for completion.
    if let Err(e) = gpu.step_and_wait() {
        warn!("GPU LBM step failed: {e}");
        return;
    }

    // Read back rendered pixels from GPU to CPU.
    match gpu.engine.read_render_pixels() {
        Ok(pixels) => {
            let expected_len = (bridge.width * bridge.height * 4) as usize;
            if pixels.len() == expected_len
                && let Some(ref mut data) = image.data
            {
                data.clear();
                data.extend_from_slice(&pixels);
            }
        }
        Err(e) => {
            warn!("GPU readback failed: {e}");
        }
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

    #[test]
    fn gpu_frame_target_default_empty() {
        let target = GpuFrameTarget::default();
        assert!(target.image.is_none());
    }

    #[test]
    fn bridge_config_custom_dimensions() {
        let config = GpuBridgeConfig {
            width: 1920,
            height: 1080,
            active: true,
        };
        assert!(config.active);
        assert_eq!(config.width, 1920);
    }
}
