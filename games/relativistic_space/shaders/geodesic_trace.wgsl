// geodesic_trace.wgsl
// Ray tracing engine: ray generation, RK4 geodesic integration,
// disk intersection, background sampling, and hit shading.
//
// Translated from Blackhole GLSL:
//   - shader/include/interop_raygen.glsl
//   - shader/include/interop_trace.glsl
//
// WGSL translation notes:
// - GLSL `inout Ray` -> ptr<function, Ray>
// - GLSL `out vec3 hitPoint` -> returns struct with hit flag + point
// - GLSL `isnan()`/`isinf()` -> manual range checks (WGSL has no isnan/isinf)
// - GLSL `any(isnan(v))` -> component-wise range check
// - Extern uniforms referenced from blackhole_main bind groups

// ============================================================================
// Constants
// ============================================================================

const BH_EPSILON: f32 = 1e-6;
const BH_DEBUG_MAX_RADIUS_MULT: f32 = 4.0;
const BH_DEBUG_FLAG_NAN: i32 = 1;
const BH_DEBUG_FLAG_RANGE: i32 = 2;
const BH_BACKGROUND_LAYERS: i32 = 3;

const GEO_PI: f32 = 3.14159265358979323846;
const GEO_TWO_PI: f32 = 6.28318530717958647692;
const GEO_INV_PI: f32 = 0.31830988618379067154;

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

struct DiskIntersection {
    hit: bool,
    point: vec3<f32>,
};

// ============================================================================
// Uniform Buffer Declarations
// ============================================================================

struct TraceUniforms {
    kerr_spin: f32,
    adisk_enabled: f32,
    isco_radius: f32,
    schwarzschild_radius: f32,
    enable_redshift: f32,
    use_luts: f32,
    use_spectral_lut: f32,
    use_grb_modulation: f32,
    lut_radius_min: f32,
    lut_radius_max: f32,
    redshift_radius_min: f32,
    redshift_radius_max: f32,
    spectral_radius_min: f32,
    spectral_radius_max: f32,
    grb_time: f32,
    grb_time_min: f32,
    grb_time_max: f32,
    background_enabled: f32,
    background_intensity: f32,
    time: f32,
    debug_flags: f32,
    _padding: f32,
};

// Bind group 1: trace uniforms and LUT textures
// (Bind group 0 is reserved for Bevy's view/mesh bindings)
@group(1) @binding(0) var<uniform> trace: TraceUniforms;
@group(1) @binding(1) var emissivity_lut: texture_2d<f32>;
@group(1) @binding(2) var emissivity_sampler: sampler;
@group(1) @binding(3) var redshift_lut: texture_2d<f32>;
@group(1) @binding(4) var redshift_sampler: sampler;
@group(1) @binding(5) var spectral_lut: texture_2d<f32>;
@group(1) @binding(6) var spectral_sampler: sampler;
@group(1) @binding(7) var grb_modulation_lut: texture_2d<f32>;
@group(1) @binding(8) var grb_modulation_sampler: sampler;
@group(1) @binding(9) var galaxy_texture: texture_cube<f32>;
@group(1) @binding(10) var galaxy_sampler: sampler;
@group(1) @binding(11) var background_layer_0: texture_2d<f32>;
@group(1) @binding(12) var background_layer_1: texture_2d<f32>;
@group(1) @binding(13) var background_layer_2: texture_2d<f32>;
@group(1) @binding(14) var background_sampler: sampler;

struct BackgroundLayerParams {
    params: array<vec4<f32>, 3>,
    lod_bias: array<f32, 3>,
    _padding: f32,
};

@group(1) @binding(15) var<uniform> bg_layers: BackgroundLayerParams;

// ============================================================================
// Ray Generation
// ============================================================================

fn bh_pixel_uv(pixel_coord: vec2<f32>, resolution: vec2<f32>) -> vec2<f32> {
    var uv = pixel_coord / resolution - vec2<f32>(0.5, 0.5);
    uv.x *= resolution.x / max(resolution.y, 1.0);
    return uv;
}

fn bh_ray_dir_from_uv(uv: vec2<f32>, fov_scale: f32, camera_basis: mat3x3<f32>) -> vec3<f32> {
    let dir = normalize(vec3<f32>(-uv.x * fov_scale, uv.y * fov_scale, 1.0));
    return camera_basis * dir;
}

fn bh_ray_dir(
    pixel_coord: vec2<f32>,
    resolution: vec2<f32>,
    fov_scale: f32,
    camera_basis: mat3x3<f32>,
) -> vec3<f32> {
    return bh_ray_dir_from_uv(bh_pixel_uv(pixel_coord, resolution), fov_scale, camera_basis);
}

// ============================================================================
// NaN/Inf Detection (WGSL lacks isnan/isinf)
// ============================================================================

// Heuristic: a float is invalid if it exceeds a very large threshold
// or is not equal to itself (NaN property, though WGSL may not guarantee this)
fn bh_is_invalid_float(v: f32) -> bool {
    return abs(v) > 1e30 || v != v;
}

fn bh_is_invalid_vec3(v: vec3<f32>) -> bool {
    return bh_is_invalid_float(v.x) || bh_is_invalid_float(v.y) || bh_is_invalid_float(v.z);
}

fn bh_debug_mask() -> i32 {
    return i32(trace.debug_flags + 0.5);
}

fn bh_debug_evaluate(pos: vec3<f32>, vel: vec3<f32>, max_distance: f32) -> i32 {
    let mask = bh_debug_mask();
    if mask == 0 {
        return 0;
    }
    var flags: i32 = 0;
    if (mask & BH_DEBUG_FLAG_NAN) != 0 {
        if bh_is_invalid_vec3(pos) || bh_is_invalid_vec3(vel) {
            flags |= BH_DEBUG_FLAG_NAN;
        }
    }
    if (mask & BH_DEBUG_FLAG_RANGE) != 0 {
        let r = length(pos);
        if r > max_distance * BH_DEBUG_MAX_RADIUS_MULT {
            flags |= BH_DEBUG_FLAG_RANGE;
        }
    }
    return flags;
}

// ============================================================================
// Schwarzschild Geodesic Integration
// ============================================================================

// Schwarzschild acceleration: a = -1.5 * r_s * h^2 * pos / r^5
fn bh_schwarzschild_accel(pos: vec3<f32>, vel: vec3<f32>, r_s: f32) -> vec3<f32> {
    let r = length(pos);
    if r < BH_EPSILON {
        return vec3<f32>(0.0, 0.0, 0.0);
    }

    let h = cross(pos, vel);
    let h2 = dot(h, h);

    let r5 = r * r * r * r * r;
    return -1.5 * r_s * h2 * pos / r5;
}

// RK4 integrator step for Schwarzschild geodesic
fn bh_step_rk4(ray: ptr<function, Ray>, r_s: f32, dt: f32) {
    let x0 = (*ray).position;
    let v0 = (*ray).velocity;

    var accel = bh_schwarzschild_accel(x0, v0, r_s);
    let k1_x = v0;
    let k1_v = accel;

    let x1 = x0 + 0.5 * dt * k1_x;
    let v1 = v0 + 0.5 * dt * k1_v;
    accel = bh_schwarzschild_accel(x1, v1, r_s);
    let k2_x = v1;
    let k2_v = accel;

    let x2 = x0 + 0.5 * dt * k2_x;
    let v2 = v0 + 0.5 * dt * k2_v;
    accel = bh_schwarzschild_accel(x2, v2, r_s);
    let k3_x = v2;
    let k3_v = accel;

    let x3 = x0 + dt * k3_x;
    let v3 = v0 + dt * k3_v;
    accel = bh_schwarzschild_accel(x3, v3, r_s);
    let k4_x = v3;
    let k4_v = accel;

    (*ray).position = x0 + (dt / 6.0) * (k1_x + 2.0 * k2_x + 2.0 * k3_x + k4_x);
    (*ray).velocity = v0 + (dt / 6.0) * (k1_v + 2.0 * k2_v + 2.0 * k3_v + k4_v);
    (*ray).affine_parameter += dt;
}

// ============================================================================
// Disk Intersection
// ============================================================================

// Check if ray crosses the equatorial plane (z=0) between old and new positions
fn bh_check_disk_intersection(
    old_pos: vec3<f32>,
    new_pos: vec3<f32>,
    r_in: f32,
    r_out: f32,
) -> DiskIntersection {
    if old_pos.z * new_pos.z > 0.0 {
        return DiskIntersection(false, vec3<f32>(0.0, 0.0, 0.0));
    }

    let t = -old_pos.z / (new_pos.z - old_pos.z);
    let hit_point = mix(old_pos, new_pos, t);
    let r = length(hit_point.xy);

    if r >= r_in && r <= r_out {
        return DiskIntersection(true, hit_point);
    }
    return DiskIntersection(false, vec3<f32>(0.0, 0.0, 0.0));
}

// Redshift factor: sqrt(1 - r_s/r)
fn bh_compute_redshift_factor(r: f32, r_s: f32) -> f32 {
    if r <= r_s {
        return 0.0;
    }
    let factor = 1.0 - r_s / r;
    if factor <= 0.0 {
        return 0.0;
    }
    return sqrt(factor);
}

// ============================================================================
// Kerr Geodesic Support (references kerr.wgsl types)
// ============================================================================
// NOTE: In a full Bevy shader pipeline, Kerr functions would be imported.
// Here we provide inline versions to keep the shader self-contained.

struct KerrConstsLocal {
    e: f32,
    lz: f32,
    q: f32,
};

struct KerrRayLocal {
    r: f32,
    theta: f32,
    phi: f32,
    t: f32,
    sign_r: f32,
    sign_theta: f32,
};

fn kerr_delta_local(r: f32, a: f32, r_s: f32) -> f32 {
    return r * r - r_s * r + a * a;
}

fn kerr_outer_horizon_local(r_s: f32, a: f32) -> f32 {
    let m = 0.5 * r_s;
    let disc = m * m - a * a;
    if disc < 0.0 {
        return r_s;
    }
    return m + sqrt(disc);
}

fn kerr_to_cartesian_local(r: f32, theta: f32, phi: f32) -> vec3<f32> {
    let sin_theta = sin(theta);
    return vec3<f32>(
        r * sin_theta * cos(phi),
        r * sin_theta * sin(phi),
        r * cos(theta),
    );
}

fn kerr_init_consts_local(pos: vec3<f32>, dir: vec3<f32>) -> KerrConstsLocal {
    let l = cross(pos, dir);
    let l2 = dot(l, l);
    return KerrConstsLocal(1.0, l.z, max(0.0, l2 - l.z * l.z));
}

fn kerr_init_ray_local(pos: vec3<f32>, dir: vec3<f32>) -> KerrRayLocal {
    let r = length(pos);
    let inv_r = select(0.0, 1.0 / r, r > BH_EPSILON);
    let cos_theta = clamp(pos.z * inv_r, -1.0, 1.0);
    let theta = acos(cos_theta);
    let phi = atan2(pos.y, pos.x);
    let e_r = normalize(pos);
    let e_theta = normalize(vec3<f32>(
        cos(theta) * cos(phi),
        cos(theta) * sin(phi),
        -sin(theta),
    ));
    let dr = dot(dir, e_r);
    let dtheta = dot(dir, e_theta);
    return KerrRayLocal(
        r, theta, phi, 0.0,
        select(-1.0, 1.0, dr >= 0.0),
        select(-1.0, 1.0, dtheta >= 0.0),
    );
}

fn kerr_step_local(ray: ptr<function, KerrRayLocal>, r_s: f32, a: f32, c: KerrConstsLocal, dlam: f32) {
    let r = (*ray).r;
    let theta = (*ray).theta;
    let sin_theta = sin(theta);
    let cos_theta = cos(theta);
    let sin2 = max(sin_theta * sin_theta, 1e-6);

    let delta = kerr_delta_local(r, a, r_s);
    let big_a = (r * r + a * a) * c.e - a * c.lz;
    let lz_minus_a_e = c.lz - a * c.e;

    let big_r = big_a * big_a - delta * (c.q + lz_minus_a_e * lz_minus_a_e);
    let big_theta = c.q + (a * a * c.e * c.e * cos_theta * cos_theta) -
                    (c.lz * c.lz / sin2);

    if big_r < 0.0 {
        (*ray).sign_r *= -1.0;
    }
    if big_theta < 0.0 {
        (*ray).sign_theta *= -1.0;
    }

    let sqrt_r = sqrt(max(big_r, 0.0));
    let sqrt_theta = sqrt(max(big_theta, 0.0));
    let delta_safe = max(delta, 1e-6);

    let dr_dlam = (*ray).sign_r * sqrt_r;
    let dtheta_dlam = (*ray).sign_theta * sqrt_theta;
    let dphi_dlam = (c.lz / sin2) - a * c.e + (a * big_a / delta_safe);
    let dt_dlam = ((r * r + a * a) * big_a / delta_safe) +
                  a * (c.lz - a * c.e * sin2);

    (*ray).r += dlam * dr_dlam;
    (*ray).theta += dlam * dtheta_dlam;
    (*ray).phi += dlam * dphi_dlam;
    (*ray).t += dlam * dt_dlam;
    (*ray).theta = clamp((*ray).theta, 1e-6, GEO_PI - 1e-6);
}

// ============================================================================
// Full Geodesic Tracer (dual Kerr/Schwarzschild)
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

    let a = 0.5 * trace.kerr_spin * r_s;

    // Kerr path (spinning black hole)
    if abs(a) > BH_EPSILON {
        let r_horizon = max(kerr_outer_horizon_local(r_s, a), BH_EPSILON);
        let r_disk_in = trace.isco_radius;
        let r_disk_out = 100.0 * r_s;

        let c = kerr_init_consts_local(ray_in.position, ray_in.velocity);
        var kerr_ray = kerr_init_ray_local(ray_in.position, ray_in.velocity);

        let dt = step_size;

        for (var step = 0; step < max_steps; step++) {
            let old_pos = kerr_to_cartesian_local(kerr_ray.r, kerr_ray.theta, kerr_ray.phi);
            result.min_radius = min(result.min_radius, kerr_ray.r);

            if kerr_ray.r <= r_horizon {
                result.hit_horizon = true;
                result.hit_point = old_pos;
                return result;
            }

            kerr_step_local(&kerr_ray, r_s, a, c, dt);
            let new_pos = kerr_to_cartesian_local(kerr_ray.r, kerr_ray.theta, kerr_ray.phi);
            result.debug_flags |= bh_debug_evaluate(new_pos, new_pos - old_pos, max_distance);

            if trace.adisk_enabled > 0.5 {
                let disk_hit = bh_check_disk_intersection(old_pos, new_pos, r_disk_in, r_disk_out);
                if disk_hit.hit {
                    result.hit_disk = true;
                    result.hit_point = disk_hit.point;
                    result.phi = atan2(disk_hit.point.y, disk_hit.point.x);
                    result.redshift_factor = bh_compute_redshift_factor(length(disk_hit.point), r_s);
                    return result;
                }
            }

            if kerr_ray.r > max_distance {
                result.escaped = true;
                result.hit_point = new_pos;
                return result;
            }
        }

        result.escaped = true;
        result.hit_point = kerr_to_cartesian_local(kerr_ray.r, kerr_ray.theta, kerr_ray.phi);
        return result;
    }

    // Schwarzschild path (non-spinning)
    let r_disk_in = trace.isco_radius;
    let r_disk_out = 100.0 * r_s;
    var ray = ray_in;
    let dt = step_size;

    for (var step = 0; step < max_steps; step++) {
        let old_pos = ray.position;
        bh_step_rk4(&ray, r_s, dt);
        result.debug_flags |= bh_debug_evaluate(ray.position, ray.velocity, max_distance);

        let r = length(ray.position);
        result.min_radius = min(result.min_radius, r);

        if r <= r_s {
            result.hit_horizon = true;
            result.hit_point = ray.position;
            return result;
        }

        if trace.adisk_enabled > 0.5 {
            let disk_hit = bh_check_disk_intersection(old_pos, ray.position, r_disk_in, r_disk_out);
            if disk_hit.hit {
                result.hit_disk = true;
                result.hit_point = disk_hit.point;
                result.phi = atan2(disk_hit.point.y, disk_hit.point.x);
                result.redshift_factor = bh_compute_redshift_factor(length(disk_hit.point), r_s);
                return result;
            }
        }

        if r > max_distance {
            result.escaped = true;
            result.hit_point = ray.position;
            return result;
        }
    }

    result.escaped = true;
    result.hit_point = ray.position;
    return result;
}

// ============================================================================
// Shading
// ============================================================================

fn bh_horizon_color() -> vec4<f32> {
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
}

fn bh_disk_color_from_hit(hit: HitResult, r_s: f32) -> vec4<f32> {
    let r = length(hit.hit_point.xy);

    var flux: f32 = 0.0;
    if trace.use_luts > 0.5 {
        let r_norm = r / max(r_s, BH_EPSILON);
        let denom = max(trace.lut_radius_max - trace.lut_radius_min, 0.0001);
        let u = clamp((r_norm - trace.lut_radius_min) / denom, 0.0, 1.0);
        flux = max(0.0, textureSample(emissivity_lut, emissivity_sampler, vec2<f32>(u, 0.5)).r);
    } else {
        let r_in = trace.isco_radius;
        let x = r_in / r;
        flux = pow(x, 3.0) * (1.0 - sqrt(x));
        flux = max(0.0, flux);
    }

    let t_norm = pow(flux, 0.25);

    var color: vec3<f32>;
    if t_norm > 0.6 {
        color = vec3<f32>(1.0, 0.9, 0.8);
    } else if t_norm > 0.3 {
        color = vec3<f32>(1.0, 0.6, 0.2);
    } else {
        color = vec3<f32>(0.8, 0.2, 0.1);
    }

    var spectral: f32 = 1.0;
    if trace.use_spectral_lut > 0.5 {
        let r_norm = r / max(r_s, BH_EPSILON);
        let denom = max(trace.spectral_radius_max - trace.spectral_radius_min, 0.0001);
        let u = clamp((r_norm - trace.spectral_radius_min) / denom, 0.0, 1.0);
        spectral = max(0.0, textureSample(spectral_lut, spectral_sampler, vec2<f32>(u, 0.5)).r);
    }

    var intensity = flux * 2.0 * spectral;

    let v = sqrt(0.5 * r_s / r);
    let cos_phi = cos(hit.phi);
    let doppler = 1.0 + 0.3 * v * cos_phi;
    intensity *= doppler * doppler * doppler;

    if trace.use_grb_modulation > 0.5 {
        let denom = max(trace.grb_time_max - trace.grb_time_min, 0.0001);
        let u = clamp((trace.grb_time - trace.grb_time_min) / denom, 0.0, 1.0);
        let modulation = textureSample(grb_modulation_lut, grb_modulation_sampler, vec2<f32>(u, 0.5)).r;
        intensity *= max(modulation, 0.0);
    }

    if trace.enable_redshift > 0.5 {
        var z = 1.0 / max(hit.redshift_factor, BH_EPSILON) - 1.0;
        if trace.use_luts > 0.5 {
            let r_norm = r / max(r_s, BH_EPSILON);
            let denom = max(trace.redshift_radius_max - trace.redshift_radius_min, 0.0001);
            let u = clamp((r_norm - trace.redshift_radius_min) / denom, 0.0, 1.0);
            z = textureSample(redshift_lut, redshift_sampler, vec2<f32>(u, 0.5)).r;
        }
        // Inline simple redshift for disk color
        let one_plus_z = 1.0 + z;
        let dimming = 1.0 / (one_plus_z * one_plus_z * one_plus_z);
        color *= dimming;
    }

    return vec4<f32>(color * intensity, 1.0);
}

fn bh_rotate_y(v: vec3<f32>, angle_degrees: f32) -> vec3<f32> {
    let angle = radians(angle_degrees);
    let c = cos(angle);
    let s = sin(angle);
    return vec3<f32>(c * v.x + s * v.z, v.y, -s * v.x + c * v.z);
}

fn bh_dir_to_uv(dir: vec3<f32>) -> vec2<f32> {
    let n = normalize(dir);
    let u = atan2(n.z, n.x) / GEO_TWO_PI + 0.5;
    let v = asin(clamp(n.y, -1.0, 1.0)) * GEO_INV_PI + 0.5;
    return vec2<f32>(u, v);
}

fn bh_background_color_from_dir(dir: vec3<f32>, min_radius: f32, r_s: f32) -> vec4<f32> {
    let n = normalize(dir);
    let sky_dir = bh_rotate_y(n, trace.time);
    var color = textureSample(galaxy_texture, galaxy_sampler, sky_dir).rgb;

    if trace.background_enabled > 0.5 {
        let base_uv = bh_dir_to_uv(sky_dir);
        var accum = vec3<f32>(0.0, 0.0, 0.0);
        var total_weight: f32 = 0.0;

        // Sample layer 0
        let params0 = bg_layers.params[0];
        if params0.w > 0.0 {
            let layer_uv = fract(base_uv * params0.z + params0.xy);
            let layer_color = textureSampleLevel(background_layer_0, background_sampler, layer_uv, bg_layers.lod_bias[0]).rgb;
            accum += layer_color * params0.w;
            total_weight += params0.w;
        }

        // Sample layer 1
        let params1 = bg_layers.params[1];
        if params1.w > 0.0 {
            let layer_uv = fract(base_uv * params1.z + params1.xy);
            let layer_color = textureSampleLevel(background_layer_1, background_sampler, layer_uv, bg_layers.lod_bias[1]).rgb;
            accum += layer_color * params1.w;
            total_weight += params1.w;
        }

        // Sample layer 2
        let params2 = bg_layers.params[2];
        if params2.w > 0.0 {
            let layer_uv = fract(base_uv * params2.z + params2.xy);
            let layer_color = textureSampleLevel(background_layer_2, background_sampler, layer_uv, bg_layers.lod_bias[2]).rgb;
            accum += layer_color * params2.w;
            total_weight += params2.w;
        }

        if total_weight > 0.0 {
            color = (accum / total_weight) * trace.background_intensity;
        }
    }

    if trace.enable_redshift > 0.5 && min_radius < r_s * 10.0 {
        var z = 1.0 / max(bh_compute_redshift_factor(min_radius, r_s), BH_EPSILON) - 1.0;
        if trace.use_luts > 0.5 {
            let r_norm = min_radius / max(r_s, BH_EPSILON);
            let denom = max(trace.redshift_radius_max - trace.redshift_radius_min, 0.0001);
            let u = clamp((r_norm - trace.redshift_radius_min) / denom, 0.0, 1.0);
            z = textureSample(redshift_lut, redshift_sampler, vec2<f32>(u, 0.5)).r;
        }
        // Inline simple redshift
        let one_plus_z = 1.0 + z;
        let dimming = 1.0 / (one_plus_z * one_plus_z * one_plus_z);
        color *= dimming;
    }

    return vec4<f32>(color, 1.0);
}

fn bh_shade_hit(hit: HitResult, camera_pos: vec3<f32>, r_s: f32) -> vec4<f32> {
    if bh_debug_mask() != 0 && hit.debug_flags != 0 {
        var debug_color = vec3<f32>(0.0, 0.0, 0.0);
        if (hit.debug_flags & BH_DEBUG_FLAG_NAN) != 0 {
            debug_color += vec3<f32>(1.0, 0.0, 1.0);
        }
        if (hit.debug_flags & BH_DEBUG_FLAG_RANGE) != 0 {
            debug_color += vec3<f32>(1.0, 1.0, 0.0);
        }
        return vec4<f32>(clamp(debug_color, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
    }
    if hit.hit_horizon {
        return bh_horizon_color();
    }
    if hit.hit_disk {
        return bh_disk_color_from_hit(hit, r_s);
    }
    return bh_background_color_from_dir(
        normalize(hit.hit_point - camera_pos),
        hit.min_radius,
        r_s,
    );
}
