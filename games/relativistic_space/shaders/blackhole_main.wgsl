// blackhole_main.wgsl
// Main fragment shader for black hole rendering.
//
// Translated from Blackhole GLSL: shader/blackhole_main.frag
//
// WGSL translation notes:
// - GLSL `out vec4 fragColor` -> @location(0) return value
// - GLSL `uniform` blocks -> struct with @group/@binding
// - GLSL `#define iscoRadius sch_iscoRadius(schwarzschildRadius)` -> computed inline
// - GLSL `inout vec3 color, inout float alpha` in adiskColor -> returns struct
// - GLSL `sampler2D` -> separate texture_2d<f32> + sampler
// - GLSL `samplerCube` -> texture_cube<f32>
// - GLSL `sampler3D` -> texture_3d<f32>
// - GLSL `gl_FragCoord` -> @builtin(position)
//
// Bind group layout:
//   Group 0: Bevy view bindings (provided by framework)
//   Group 1: Reserved for Bevy mesh bindings
//   Group 2: Black hole uniforms + textures

// ============================================================================
// Constants
// ============================================================================

const BH_MAIN_EPSILON: f32 = 0.0001;
const BH_MAIN_PI: f32 = 3.14159265358979323846;

// ============================================================================
// Structures
// ============================================================================

struct Ray {
    position: vec3<f32>,
    velocity: vec3<f32>,
    affine_parameter: f32,
};

struct HitResult {
    hit_disk: bool,
    hit_horizon: bool,
    escaped: bool,
    hit_point: vec3<f32>,
    phi: f32,
    redshift_factor: f32,
    min_radius: f32,
    debug_flags: i32,
};

struct AdiskResult {
    contributed: bool,
    color: vec3<f32>,
    alpha: f32,
};

struct TraceResult {
    color: vec3<f32>,
    depth_distance: f32,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

// ============================================================================
// Main Uniforms (Group 2)
// ============================================================================

struct BlackHoleUniforms {
    resolution: vec2<f32>,
    time: f32,
    fov_scale: f32,

    camera_pos: vec3<f32>,
    depth_far: f32,

    schwarzschild_radius: f32,
    gravitational_lensing: f32,
    render_black_hole: f32,
    interop_parity_mode: f32,

    interop_max_steps: f32,
    interop_step_size: f32,
    adisk_enabled: f32,
    adisk_particle: f32,

    adisk_height: f32,
    adisk_lit: f32,
    adisk_density_v: f32,
    adisk_density_h: f32,

    adisk_noise_scale: f32,
    adisk_noise_lod: f32,
    adisk_speed: f32,
    doppler_strength: f32,

    enable_redshift: f32,
    enable_photon_sphere: f32,
    hawking_glow_enabled: f32,
    hawking_temp_scale: f32,

    hawking_glow_intensity: f32,
    use_hawking_luts: f32,
    black_hole_mass: f32,
    kerr_spin: f32,

    use_luts: f32,
    use_noise_texture: f32,
    use_grmhd: f32,
    use_spectral_lut: f32,

    noise_texture_scale: f32,
    lut_radius_min: f32,
    lut_radius_max: f32,
    redshift_radius_min: f32,

    redshift_radius_max: f32,
    spectral_radius_min: f32,
    spectral_radius_max: f32,
    use_grb_modulation: f32,

    grb_time: f32,
    grb_time_min: f32,
    grb_time_max: f32,
    background_enabled: f32,

    background_intensity: f32,
    _padding0: f32,
    _padding1: f32,
    _padding2: f32,
};

@group(2) @binding(0) var<uniform> bh: BlackHoleUniforms;
@group(2) @binding(1) var<uniform> camera_basis: mat3x3<f32>;

// Textures
@group(2) @binding(2) var color_map_texture: texture_2d<f32>;
@group(2) @binding(3) var color_map_sampler: sampler;
@group(2) @binding(4) var noise_texture: texture_3d<f32>;
@group(2) @binding(5) var noise_sampler: sampler;
@group(2) @binding(6) var grmhd_texture: texture_3d<f32>;
@group(2) @binding(7) var grmhd_sampler: sampler;
@group(2) @binding(8) var photon_glow_lut: texture_2d<f32>;
@group(2) @binding(9) var photon_glow_sampler: sampler;
@group(2) @binding(10) var disk_density_lut: texture_2d<f32>;
@group(2) @binding(11) var disk_density_sampler: sampler;
@group(2) @binding(12) var hawking_temp_lut: texture_2d<f32>;
@group(2) @binding(13) var hawking_temp_sampler: sampler;
@group(2) @binding(14) var hawking_spectrum_lut: texture_2d<f32>;
@group(2) @binding(15) var hawking_spectrum_sampler: sampler;
@group(2) @binding(16) var main_emissivity_lut: texture_2d<f32>;
@group(2) @binding(17) var main_emissivity_sampler: sampler;
@group(2) @binding(18) var main_redshift_lut: texture_2d<f32>;
@group(2) @binding(19) var main_redshift_sampler: sampler;
@group(2) @binding(20) var main_spectral_lut: texture_2d<f32>;
@group(2) @binding(21) var main_spectral_sampler: sampler;

// ============================================================================
// Ray Generation (from interop_raygen)
// ============================================================================

fn bh_pixel_uv(pixel_coord: vec2<f32>, resolution: vec2<f32>) -> vec2<f32> {
    var uv = pixel_coord / resolution - vec2<f32>(0.5, 0.5);
    uv.x *= resolution.x / max(resolution.y, 1.0);
    return uv;
}

fn bh_ray_dir_from_uv(uv: vec2<f32>, fov_scale: f32, basis: mat3x3<f32>) -> vec3<f32> {
    let dir = normalize(vec3<f32>(-uv.x * fov_scale, uv.y * fov_scale, 1.0));
    return basis * dir;
}

fn bh_ray_dir(
    pixel_coord: vec2<f32>,
    resolution: vec2<f32>,
    fov_scale: f32,
    basis: mat3x3<f32>,
) -> vec3<f32> {
    return bh_ray_dir_from_uv(bh_pixel_uv(pixel_coord, resolution), fov_scale, basis);
}

// ============================================================================
// Derived Physics Quantities (replaces GLSL #define macros)
// ============================================================================

fn isco_radius() -> f32 {
    return 3.0 * bh.schwarzschild_radius;
}

fn photon_sphere_radius() -> f32 {
    return 1.5 * bh.schwarzschild_radius;
}

// ============================================================================
// Redshift Helpers (inlined for self-containment)
// ============================================================================

fn gravitational_redshift_main(r: f32, r_s: f32) -> f32 {
    let f = max(1.0 - r_s / r, 0.001);
    return 1.0 / sqrt(f) - 1.0;
}

fn apply_simple_redshift_main(color: vec3<f32>, z: f32) -> vec3<f32> {
    let one_plus_z = 1.0 + z;
    let dimming = 1.0 / (one_plus_z * one_plus_z * one_plus_z);
    return color * dimming;
}

fn apply_gravitational_redshift_main(color: vec3<f32>, z: f32) -> vec3<f32> {
    let luminance = dot(color, vec3<f32>(0.299, 0.587, 0.114));
    if luminance < 0.01 {
        return color;
    }
    let total = color.r + color.g + color.b + 0.001;
    let r_weight = color.r / total;
    let b_weight = color.b / total;
    let est_wl = clamp(550.0 + 150.0 * (r_weight - b_weight), 400.0, 700.0);
    let shifted_wl = est_wl * (1.0 + z);

    var r: f32 = 0.0;
    var g: f32 = 0.0;
    var b: f32 = 0.0;
    if shifted_wl >= 380.0 && shifted_wl < 440.0 {
        r = -(shifted_wl - 440.0) / 60.0;
        b = 1.0;
    } else if shifted_wl >= 440.0 && shifted_wl < 490.0 {
        g = (shifted_wl - 440.0) / 50.0;
        b = 1.0;
    } else if shifted_wl >= 490.0 && shifted_wl < 510.0 {
        g = 1.0;
        b = -(shifted_wl - 510.0) / 20.0;
    } else if shifted_wl >= 510.0 && shifted_wl < 580.0 {
        r = (shifted_wl - 510.0) / 70.0;
        g = 1.0;
    } else if shifted_wl >= 580.0 && shifted_wl < 645.0 {
        r = 1.0;
        g = -(shifted_wl - 645.0) / 65.0;
    } else if shifted_wl >= 645.0 && shifted_wl <= 780.0 {
        r = 1.0;
    }

    var shifted_color = vec3<f32>(r, g, b);
    let orig_intensity = length(color);
    let new_intensity = length(shifted_color);
    if new_intensity > 0.01 {
        shifted_color *= orig_intensity / new_intensity;
    }
    return shifted_color;
}

// ============================================================================
// Quaternion Rotation
// ============================================================================

fn rotate_vector(position: vec3<f32>, axis: vec3<f32>, angle: f32) -> vec3<f32> {
    let half_angle = (angle * 0.5) * BH_MAIN_PI / 180.0;
    let s = sin(half_angle);
    let qr = vec4<f32>(axis.x * s, axis.y * s, axis.z * s, cos(half_angle));
    let qr_conj = vec4<f32>(-qr.x, -qr.y, -qr.z, qr.w);
    let q_pos = vec4<f32>(position.x, position.y, position.z, 0.0);

    let q_tmp = vec4<f32>(
        qr.w * q_pos.x + qr.x * q_pos.w + qr.y * q_pos.z - qr.z * q_pos.y,
        qr.w * q_pos.y - qr.x * q_pos.z + qr.y * q_pos.w + qr.z * q_pos.x,
        qr.w * q_pos.z + qr.x * q_pos.y - qr.y * q_pos.x + qr.z * q_pos.w,
        qr.w * q_pos.w - qr.x * q_pos.x - qr.y * q_pos.y - qr.z * q_pos.z,
    );

    let result = vec4<f32>(
        q_tmp.w * qr_conj.x + q_tmp.x * qr_conj.w + q_tmp.y * qr_conj.z - q_tmp.z * qr_conj.y,
        q_tmp.w * qr_conj.y - q_tmp.x * qr_conj.z + q_tmp.y * qr_conj.w + q_tmp.z * qr_conj.x,
        q_tmp.w * qr_conj.z + q_tmp.x * qr_conj.y - q_tmp.y * qr_conj.x + q_tmp.z * qr_conj.w,
        q_tmp.w * qr_conj.w - q_tmp.x * qr_conj.x - q_tmp.y * qr_conj.y - q_tmp.z * qr_conj.z,
    );

    return vec3<f32>(result.x, result.y, result.z);
}

// ============================================================================
// Spherical Coordinates
// ============================================================================

fn to_spherical(p: vec3<f32>) -> vec3<f32> {
    let rho = sqrt(p.x * p.x + p.y * p.y + p.z * p.z);
    let theta = atan2(p.z, p.x);
    let phi = asin(p.y / rho);
    return vec3<f32>(rho, theta, phi);
}

// ============================================================================
// Schwarzschild RK4 Acceleration
// ============================================================================

fn bh_schwarzschild_accel(pos: vec3<f32>, vel: vec3<f32>, r_s: f32) -> vec3<f32> {
    let r = length(pos);
    if r < 1e-6 {
        return vec3<f32>(0.0, 0.0, 0.0);
    }
    let h = cross(pos, vel);
    let h2 = dot(h, h);
    let r5 = r * r * r * r * r;
    return -1.5 * r_s * h2 * pos / r5;
}

// ============================================================================
// Geodesic Tracer (Schwarzschild, standalone RK4)
// ============================================================================

fn bh_trace_geodesic(
    ray_in: Ray,
    r_s: f32,
    max_distance: f32,
    max_steps: i32,
    step_size: f32,
) -> HitResult {
    var result: HitResult;
    result.hit_disk = false;
    result.hit_horizon = false;
    result.escaped = false;
    result.hit_point = vec3<f32>(0.0, 0.0, 0.0);
    result.phi = 0.0;
    result.redshift_factor = 1.0;
    result.min_radius = length(ray_in.position);
    result.debug_flags = 0;

    var pos = ray_in.position;
    var vel = ray_in.velocity;
    let dt = step_size;

    for (var step = 0; step < max_steps; step++) {
        let r0 = length(pos);
        if r0 < 1e-6 {
            break;
        }

        // RK4 step
        let a0 = bh_schwarzschild_accel(pos, vel, r_s);
        let k1_x = vel;
        let k1_v = a0;

        let x1 = pos + 0.5 * dt * k1_x;
        let v1 = vel + 0.5 * dt * k1_v;
        let a1 = bh_schwarzschild_accel(x1, v1, r_s);

        let x2 = pos + 0.5 * dt * v1;
        let v2 = vel + 0.5 * dt * a1;
        let a2 = bh_schwarzschild_accel(x2, v2, r_s);

        let x3 = pos + dt * v2;
        let v3 = vel + dt * a2;
        let a3 = bh_schwarzschild_accel(x3, v3, r_s);

        pos = pos + (dt / 6.0) * (k1_x + 2.0 * v1 + 2.0 * v2 + v3);
        vel = vel + (dt / 6.0) * (a0 + 2.0 * a1 + 2.0 * a2 + a3);

        let r = length(pos);
        result.min_radius = min(result.min_radius, r);

        if r <= r_s {
            result.hit_horizon = true;
            result.hit_point = pos;
            return result;
        }

        if r > max_distance {
            result.escaped = true;
            result.hit_point = pos;
            return result;
        }
    }

    result.escaped = true;
    result.hit_point = pos;
    return result;
}

// ============================================================================
// Hit Shading
// ============================================================================

fn bh_shade_hit(hit: HitResult, camera_pos: vec3<f32>, r_s: f32) -> vec4<f32> {
    if hit.hit_horizon {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
    let dir = normalize(hit.hit_point - camera_pos);
    let u = atan2(dir.z, dir.x) / (2.0 * BH_MAIN_PI) + 0.5;
    let v = asin(clamp(dir.y, -1.0, 1.0)) / BH_MAIN_PI + 0.5;
    let color = textureSample(color_map_texture, color_map_sampler, vec2<f32>(u, v)).rgb;
    return vec4<f32>(color, 1.0);
}

// ============================================================================
// Accretion Disk Color
// ============================================================================

fn adisk_color(
    pos: vec3<f32>,
    ray_dir: vec3<f32>,
    input_color: vec3<f32>,
    input_alpha: f32,
) -> AdiskResult {
    let inner_radius = isco_radius();
    let outer_radius = isco_radius() * 4.0;
    let r = length(pos);

    let density_test = 1.0 - length(pos.xyz / vec3<f32>(outer_radius, bh.adisk_height, outer_radius));
    var density = max(0.0, density_test);
    if density < 0.001 {
        return AdiskResult(false, input_color, input_alpha);
    }

    let normalized_v = abs(pos.y) / bh.adisk_height;
    let vertical_density = textureSample(
        disk_density_lut, disk_density_sampler,
        vec2<f32>(clamp(normalized_v, 0.0, 1.0), 0.5),
    ).r;
    density *= vertical_density;
    density *= smoothstep(inner_radius, inner_radius * 1.1, r);

    if density < 0.001 {
        return AdiskResult(false, input_color, input_alpha);
    }

    var spherical_coord = to_spherical(pos);
    spherical_coord.y *= 2.0;
    spherical_coord.z *= 4.0;

    density *= 1.0 / pow(spherical_coord.x, bh.adisk_density_h);
    density *= 16000.0;

    if bh.adisk_particle < 0.5 {
        let new_color = input_color + vec3<f32>(0.0, 1.0, 0.0) * density * 0.02;
        return AdiskResult(true, new_color, input_alpha);
    }

    // Noise sampling
    var noise: f32 = 1.0;
    let use_noise_tex = bh.use_noise_texture > 0.5;
    var sc = spherical_coord;
    for (var i = 0; i < i32(bh.adisk_noise_lod); i++) {
        var noise_sample: f32 = 1.0;
        if use_noise_tex {
            let noise_coord = fract(sc * bh.noise_texture_scale * bh.adisk_noise_scale);
            noise_sample = textureSample(noise_texture, noise_sampler, noise_coord).r;
        }
        noise *= noise_sample;
        if i % 2 == 0 {
            sc.y += bh.time * bh.adisk_speed;
        } else {
            sc.y -= bh.time * bh.adisk_speed;
        }
    }

    var dust_color = textureSample(
        color_map_texture, color_map_sampler,
        vec2<f32>(spherical_coord.x / outer_radius, 0.5),
    ).rgb;

    // Relativistic Doppler beaming
    let vel_dir = normalize(vec3<f32>(-pos.z, 0.0, pos.x));
    let doppler = 1.0 + dot(vel_dir, normalize(ray_dir)) * 0.5 * bh.doppler_strength;
    dust_color *= max(0.2, doppler);

    // Emissivity LUT
    let r_norm = r / max(bh.schwarzschild_radius, BH_MAIN_EPSILON);
    if bh.use_luts > 0.5 {
        let denom = max(bh.lut_radius_max - bh.lut_radius_min, 0.0001);
        let u = clamp((r_norm - bh.lut_radius_min) / denom, 0.0, 1.0);
        let lut_emissivity = max(0.0, textureSample(
            main_emissivity_lut, main_emissivity_sampler,
            vec2<f32>(u, 0.5),
        ).r);
        density *= lut_emissivity;
    }

    // Spectral LUT
    if bh.use_spectral_lut > 0.5 {
        let denom = max(bh.spectral_radius_max - bh.spectral_radius_min, 0.0001);
        let u = clamp((r_norm - bh.spectral_radius_min) / denom, 0.0, 1.0);
        let spectral_value = max(0.0, textureSample(
            main_spectral_lut, main_spectral_sampler,
            vec2<f32>(u, 0.5),
        ).r);
        density *= spectral_value;
    }

    // Gravitational redshift
    if bh.enable_redshift > 0.5 {
        var z = gravitational_redshift_main(r, bh.schwarzschild_radius);
        if bh.use_luts > 0.5 {
            let denom = max(bh.redshift_radius_max - bh.redshift_radius_min, 0.0001);
            let u = clamp((r_norm - bh.redshift_radius_min) / denom, 0.0, 1.0);
            z = textureSample(main_redshift_lut, main_redshift_sampler, vec2<f32>(u, 0.5)).r;
        }
        dust_color = apply_gravitational_redshift_main(dust_color, z);
    }

    let new_color = input_color + density * bh.adisk_lit * dust_color * input_alpha * abs(noise);
    return AdiskResult(true, new_color, input_alpha);
}

// ============================================================================
// Legacy Ray Tracer (300-step volumetric)
// ============================================================================

fn trace_color(pos_in: vec3<f32>, dir_in: vec3<f32>) -> TraceResult {
    var color = vec3<f32>(0.0, 0.0, 0.0);
    var alpha: f32 = 1.0;
    let origin = pos_in;
    var depth_distance = bh.depth_far;

    let step_size: f32 = 0.1;
    var dir = dir_in * step_size;
    var pos = pos_in;

    let h = cross(pos, dir);
    let h2 = dot(h, h);

    var min_radius_reached = length(pos);
    let r_ph = photon_sphere_radius();

    for (var i = 0; i < 300; i++) {
        let r = length(pos);
        min_radius_reached = min(min_radius_reached, r);

        if bh.render_black_hole > 0.5 {
            if bh.gravitational_lensing > 0.5 {
                let r2 = dot(pos, pos);
                let r5 = r2 * r2 * r;
                let acc = -1.5 * bh.schwarzschild_radius * h2 * pos / r5;
                dir += acc;
            }

            if r < bh.schwarzschild_radius {
                depth_distance = min(depth_distance, length(pos - origin));

                if bh.hawking_glow_enabled > 0.5 {
                    let radius_ratio = r / max(bh.schwarzschild_radius, 1e-10);
                    let exp_factor = exp(-radius_ratio * 4.0);
                    let inv_sq = 1.0 / (radius_ratio * radius_ratio + 0.01);
                    let falloff = exp_factor * inv_sq;
                    let glow_color = vec3<f32>(1.0, 0.8, 0.6) * falloff * bh.hawking_glow_intensity;
                    color += glow_color;
                }

                return TraceResult(color, depth_distance);
            }

            if bh.enable_photon_sphere > 0.5 {
                let photon_dist = abs(r - r_ph);
                if photon_dist < 0.5 {
                    let u = photon_dist / 0.5;
                    let glow_intensity = textureSample(
                        photon_glow_lut, photon_glow_sampler,
                        vec2<f32>(u, 0.5),
                    ).r * 0.3;
                    color += vec3<f32>(1.0, 0.7, 0.3) * glow_intensity * alpha;
                    depth_distance = min(depth_distance, length(pos - origin));
                }
            }

            if bh.adisk_enabled > 0.5 {
                let adisk_result = adisk_color(pos, dir, color, alpha);
                if adisk_result.contributed {
                    color = adisk_result.color;
                    alpha = adisk_result.alpha;
                    depth_distance = min(depth_distance, length(pos - origin));
                }
            }
        }

        pos += dir;
    }

    // Background
    let rotated_dir = rotate_vector(dir, vec3<f32>(0.0, 1.0, 0.0), bh.time);
    var sky_color = vec3<f32>(0.0, 0.0, 0.0);
    let d = normalize(rotated_dir);
    let bg_u = atan2(d.z, d.x) / (2.0 * BH_MAIN_PI) + 0.5;
    let bg_v = asin(clamp(d.y, -1.0, 1.0)) / BH_MAIN_PI + 0.5;

    if bh.background_enabled > 0.5 {
        sky_color = textureSample(color_map_texture, color_map_sampler, vec2<f32>(bg_u, bg_v)).rgb;
        sky_color *= bh.background_intensity;
    } else {
        sky_color = textureSample(color_map_texture, color_map_sampler, vec2<f32>(bg_u, bg_v)).rgb;
    }

    if bh.enable_redshift > 0.5 && min_radius_reached < bh.schwarzschild_radius * 10.0 {
        var z = gravitational_redshift_main(min_radius_reached, bh.schwarzschild_radius);
        if bh.use_luts > 0.5 {
            let r_norm = min_radius_reached / max(bh.schwarzschild_radius, BH_MAIN_EPSILON);
            let denom = max(bh.redshift_radius_max - bh.redshift_radius_min, 0.0001);
            let u = clamp((r_norm - bh.redshift_radius_min) / denom, 0.0, 1.0);
            z = textureSample(main_redshift_lut, main_redshift_sampler, vec2<f32>(u, 0.5)).r;
        }
        sky_color = apply_simple_redshift_main(sky_color, z);
    }

    color += sky_color * alpha;
    return TraceResult(color, depth_distance);
}

// ============================================================================
// Fragment Entry Point
// ============================================================================

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let pixel_coord = in.position.xy;
    let dir = bh_ray_dir(pixel_coord, bh.resolution, bh.fov_scale, camera_basis);
    let pos = bh.camera_pos;

    // Parity mode: use RK4 geodesic tracer
    if bh.interop_parity_mode > 0.5 {
        var ray: Ray;
        ray.position = pos;
        ray.velocity = dir;
        ray.affine_parameter = 0.0;

        let steps = i32(max(1.0, bh.interop_max_steps + 0.5));
        let hit = bh_trace_geodesic(
            ray, bh.schwarzschild_radius, bh.depth_far, steps, bh.interop_step_size,
        );
        let shaded = bh_shade_hit(hit, bh.camera_pos, bh.schwarzschild_radius);
        let depth_normalized = clamp(
            length(hit.hit_point - bh.camera_pos) / max(bh.depth_far, 0.0001),
            0.0, 1.0,
        );
        return vec4<f32>(shaded.rgb, depth_normalized);
    }

    // Legacy mode: 300-step volumetric tracer
    let result = trace_color(pos, dir);
    let depth_normalized = clamp(result.depth_distance / bh.depth_far, 0.0, 1.0);
    return vec4<f32>(result.color, depth_normalized);
}
