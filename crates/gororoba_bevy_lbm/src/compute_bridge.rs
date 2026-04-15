use bevy::image::Image;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};

use crate::components::FluidDomain;
use crate::resources::FluidSimulationEngine;

#[derive(Resource, Default)]
pub struct FluidFrameBridgeConfig {
    pub active: bool,
}

#[derive(Resource, Default)]
pub struct FluidFrameTarget {
    pub image: Option<Handle<Image>>,
}

pub fn frame_bridge_init_system(
    bridge: Res<FluidFrameBridgeConfig>,
    engine: Res<FluidSimulationEngine>,
    query: Query<Entity, With<FluidDomain>>,
    mut target: ResMut<FluidFrameTarget>,
    mut images: ResMut<Assets<Image>>,
) {
    if !bridge.active || target.image.is_some() {
        return;
    }

    for entity in &query {
        let Some(instance) = engine.get(entity) else {
            continue;
        };
        let Some(render_target) = instance.domain_config().execution.render_target else {
            continue;
        };

        let mut image = Image::new_fill(
            Extent3d {
                width: render_target.width,
                height: render_target.height,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[0, 0, 0, 255],
            TextureFormat::Rgba8UnormSrgb,
            bevy::asset::RenderAssetUsages::all(),
        );
        image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING
            | TextureUsages::COPY_DST
            | TextureUsages::STORAGE_BINDING;
        target.image = Some(images.add(image));
        return;
    }
}

pub fn frame_readback_system(
    bridge: Res<FluidFrameBridgeConfig>,
    mut engine: ResMut<FluidSimulationEngine>,
    target: Res<FluidFrameTarget>,
    query: Query<Entity, With<FluidDomain>>,
    mut images: ResMut<Assets<Image>>,
) {
    if !bridge.active {
        return;
    }

    let Some(handle) = &target.image else {
        return;
    };
    let Some(image) = images.get_mut(handle) else {
        return;
    };

    for entity in &query {
        let Some(instance) = engine.get_mut(entity) else {
            continue;
        };
        match instance.kernel.render_frame() {
            Ok(Some(frame)) => {
                if frame.rgba8.len() == frame.width as usize * frame.height as usize * 4 {
                    image.data = Some(frame.rgba8);
                }
                return;
            }
            Ok(None) => {}
            Err(error) => warn!("Fluid frame readback failed for {entity:?}: {error}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bridge_default_inactive() {
        let config = FluidFrameBridgeConfig::default();
        assert!(!config.active);
    }

    #[test]
    fn frame_target_default_empty() {
        let target = FluidFrameTarget::default();
        assert!(target.image.is_none());
    }
}
