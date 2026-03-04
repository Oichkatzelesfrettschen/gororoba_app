// Custom Bevy material for the black hole ray-tracing shader.
//
// Binds the BlackHoleUniforms struct and all LUT/noise textures to the
// blackhole_main.wgsl fragment shader via Bevy's AsBindGroup derive.
//
// The shader runs as a fullscreen fragment pass: every pixel shoots a ray
// from the camera through curved spacetime, accumulating accretion disk
// emission, gravitational lensing, Hawking glow, and background stars.

use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;

use crate::lut_loader::LutAssets;

/// Uniform block matching BlackHoleUniforms in blackhole_main.wgsl.
///
/// Fields are ordered to match the WGSL struct layout with 16-byte alignment.
/// Every vec3 is followed by a padding float to align to 16 bytes.
#[derive(Clone, ShaderType)]
pub struct BlackHoleUniformData {
    pub resolution: Vec2,
    pub time: f32,
    pub fov_scale: f32,

    pub camera_pos: Vec3,
    pub depth_far: f32,

    pub schwarzschild_radius: f32,
    pub gravitational_lensing: f32,
    pub render_black_hole: f32,
    pub interop_parity_mode: f32,

    pub interop_max_steps: f32,
    pub interop_step_size: f32,
    pub adisk_enabled: f32,
    pub adisk_particle: f32,

    pub adisk_height: f32,
    pub adisk_lit: f32,
    pub adisk_density_v: f32,
    pub adisk_density_h: f32,

    pub adisk_noise_scale: f32,
    pub adisk_noise_lod: f32,
    pub adisk_speed: f32,
    pub doppler_strength: f32,

    pub enable_redshift: f32,
    pub enable_photon_sphere: f32,
    pub hawking_glow_enabled: f32,
    pub hawking_temp_scale: f32,

    pub hawking_glow_intensity: f32,
    pub use_hawking_luts: f32,
    pub black_hole_mass: f32,
    pub kerr_spin: f32,

    pub use_luts: f32,
    pub use_noise_texture: f32,
    pub use_grmhd: f32,
    pub use_spectral_lut: f32,

    pub noise_texture_scale: f32,
    pub lut_radius_min: f32,
    pub lut_radius_max: f32,
    pub redshift_radius_min: f32,

    pub redshift_radius_max: f32,
    pub spectral_radius_min: f32,
    pub spectral_radius_max: f32,
    pub use_grb_modulation: f32,

    pub grb_time: f32,
    pub grb_time_min: f32,
    pub grb_time_max: f32,
    pub background_enabled: f32,

    pub background_intensity: f32,
    pub _padding0: f32,
    pub _padding1: f32,
    pub _padding2: f32,
}

impl Default for BlackHoleUniformData {
    fn default() -> Self {
        Self {
            resolution: Vec2::new(1280.0, 720.0),
            time: 0.0,
            fov_scale: 1.0,

            camera_pos: Vec3::new(0.0, 0.0, 50.0),
            depth_far: 1000.0,

            schwarzschild_radius: 2.0,
            gravitational_lensing: 1.0,
            render_black_hole: 1.0,
            interop_parity_mode: 0.0,

            interop_max_steps: 300.0,
            interop_step_size: 0.1,
            adisk_enabled: 1.0,
            adisk_particle: 0.0,

            adisk_height: 0.3,
            adisk_lit: 1.0,
            adisk_density_v: 1.0,
            adisk_density_h: 1.0,

            adisk_noise_scale: 4.0,
            adisk_noise_lod: 3.0,
            adisk_speed: 0.5,
            doppler_strength: 1.0,

            enable_redshift: 1.0,
            enable_photon_sphere: 0.0,
            hawking_glow_enabled: 0.0,
            hawking_temp_scale: 1.0,

            hawking_glow_intensity: 1.0,
            use_hawking_luts: 0.0,
            black_hole_mass: 1.0,
            kerr_spin: 0.0,

            use_luts: 0.0,
            use_noise_texture: 0.0,
            use_grmhd: 0.0,
            use_spectral_lut: 0.0,

            noise_texture_scale: 1.0,
            lut_radius_min: 3.0,
            lut_radius_max: 12.0,
            redshift_radius_min: 3.0,

            redshift_radius_max: 12.0,
            spectral_radius_min: 3.0,
            spectral_radius_max: 12.0,
            use_grb_modulation: 0.0,

            grb_time: 0.0,
            grb_time_min: 0.0,
            grb_time_max: 100.0,
            background_enabled: 1.0,

            background_intensity: 0.5,
            _padding0: 0.0,
            _padding1: 0.0,
            _padding2: 0.0,
        }
    }
}

/// Material for the black hole fragment shader.
///
/// Uses Bevy's `AsBindGroup` to bind uniforms and textures to the shader.
/// All bindings map to `@group(2)` in the WGSL (Bevy reserves groups 0-1).
#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct BlackHoleMaterial {
    #[uniform(0)]
    pub uniforms: BlackHoleUniformData,

    // Camera basis matrix (view-space to world-space rotation).
    // This maps to @group(2) @binding(1) in the WGSL.
    #[uniform(1)]
    pub camera_basis: Mat3,

    // Color map texture for accretion disk coloring.
    #[texture(2)]
    #[sampler(3)]
    pub color_map: Handle<Image>,

    // Emissivity LUT from Novikov-Thorne model.
    #[texture(16)]
    #[sampler(17)]
    pub emissivity_lut: Handle<Image>,

    // Redshift correction LUT.
    #[texture(18)]
    #[sampler(19)]
    pub redshift_lut: Handle<Image>,

    // Hawking temperature LUT.
    #[texture(12)]
    #[sampler(13)]
    pub hawking_temp_lut: Handle<Image>,

    // Hawking spectrum (blackbody RGB) LUT.
    #[texture(14)]
    #[sampler(15)]
    pub hawking_spectrum_lut: Handle<Image>,
}

impl Material for BlackHoleMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/blackhole_main.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Opaque
    }
}

/// Plugin that registers the black hole material with Bevy.
pub struct BlackHoleMaterialPlugin;

impl Plugin for BlackHoleMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<BlackHoleMaterial>::default())
            .add_systems(Update, (update_blackhole_time, update_blackhole_camera));
    }
}

/// Resource holding the active black hole material handle for live parameter updates.
#[derive(Resource)]
pub struct ActiveBlackHoleMaterial {
    pub handle: Handle<BlackHoleMaterial>,
}

/// Spawn a fullscreen quad with the black hole material.
pub fn spawn_blackhole_quad(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<BlackHoleMaterial>,
    luts: &LutAssets,
    color_map: Handle<Image>,
) -> Handle<BlackHoleMaterial> {
    let material = BlackHoleMaterial {
        uniforms: BlackHoleUniformData::default(),
        camera_basis: Mat3::IDENTITY,
        color_map,
        emissivity_lut: luts.emissivity.clone(),
        redshift_lut: luts.redshift.clone(),
        hawking_temp_lut: luts.hawking_temp.clone(),
        hawking_spectrum_lut: luts.hawking_spectrum.clone(),
    };

    let material_handle = materials.add(material);

    // Fullscreen quad: a plane scaled to fill the screen, positioned at the far plane.
    let quad = Mesh::from(Rectangle::new(2.0, 2.0));
    commands.spawn((
        Mesh3d(meshes.add(quad)),
        MeshMaterial3d(material_handle.clone()),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    material_handle
}

/// Update the time uniform each frame for animated effects.
fn update_blackhole_time(
    time: Res<Time>,
    active: Option<Res<ActiveBlackHoleMaterial>>,
    mut materials: ResMut<Assets<BlackHoleMaterial>>,
) {
    let Some(active) = active else { return };
    let Some(mat) = materials.get_mut(&active.handle) else {
        return;
    };
    mat.uniforms.time = time.elapsed_secs();
}

/// Sync the camera position and basis matrix to the shader uniforms.
fn update_blackhole_camera(
    active: Option<Res<ActiveBlackHoleMaterial>>,
    mut materials: ResMut<Assets<BlackHoleMaterial>>,
    camera_query: Query<&Transform, With<Camera3d>>,
) {
    let Some(active) = active else { return };
    let Some(mat) = materials.get_mut(&active.handle) else {
        return;
    };

    if let Ok(cam_transform) = camera_query.single() {
        mat.uniforms.camera_pos = cam_transform.translation;

        // Extract the 3x3 rotation matrix from the camera transform.
        let rot = cam_transform.rotation;
        let right = rot * Vec3::X;
        let up = rot * Vec3::Y;
        let forward = rot * Vec3::NEG_Z;
        mat.camera_basis = Mat3::from_cols(right, up, forward);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_uniforms_reasonable() {
        let u = BlackHoleUniformData::default();
        assert!(u.schwarzschild_radius > 0.0);
        assert!(u.fov_scale > 0.0);
        assert!(u.interop_max_steps > 0.0);
        assert!(u.depth_far > 0.0);
        assert!((u.render_black_hole - 1.0).abs() < 1e-6);
    }

    #[test]
    fn default_uniforms_alignment() {
        // Verify the struct has correct padding for GPU alignment.
        // BlackHoleUniforms has 48 f32 fields (192 bytes), which must be
        // a multiple of 16 for std140/std430 layout.
        let size = std::mem::size_of::<BlackHoleUniformData>();
        assert_eq!(size % 16, 0, "Uniform data must be 16-byte aligned");
    }

    #[test]
    fn camera_basis_identity() {
        let mat = Mat3::IDENTITY;
        assert!((mat.x_axis - Vec3::X).length() < 1e-6);
        assert!((mat.y_axis - Vec3::Y).length() < 1e-6);
        assert!((mat.z_axis - Vec3::Z).length() < 1e-6);
    }
}
