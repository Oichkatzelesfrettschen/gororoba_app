// kerr.wgsl
// Kerr metric utilities for rotating black hole ray tracing.
//
// Translated from Blackhole GLSL: shader/include/kerr.glsl
//
// WGSL translation notes:
// - GLSL `inout` parameters replaced with ptr<function, T> arguments
// - GLSL struct constructors replaced with explicit field initialization

const KERR_EPSILON: f32 = 1e-6;

struct KerrConsts {
    e: f32,   // Energy
    lz: f32,  // z-component of angular momentum
    q: f32,   // Carter constant
};

struct KerrRay {
    r: f32,
    theta: f32,
    phi: f32,
    t: f32,
    sign_r: f32,
    sign_theta: f32,
};

// Kerr Sigma function: Sigma = r^2 + a^2 cos^2(theta)
fn kerr_sigma(r: f32, a: f32, cos_theta: f32) -> f32 {
    return r * r + a * a * cos_theta * cos_theta;
}

// Kerr Delta function: Delta = r^2 - r_s*r + a^2
fn kerr_delta(r: f32, a: f32, r_s: f32) -> f32 {
    return r * r - r_s * r + a * a;
}

// Kerr outer horizon radius
fn kerr_outer_horizon(r_s: f32, a: f32) -> f32 {
    let m = 0.5 * r_s;
    let disc = m * m - a * a;
    if disc < 0.0 {
        return r_s;
    }
    return m + sqrt(disc);
}

// Convert Boyer-Lindquist to Cartesian coordinates
fn kerr_to_cartesian(r: f32, theta: f32, phi: f32) -> vec3<f32> {
    let sin_theta = sin(theta);
    return vec3<f32>(
        r * sin_theta * cos(phi),
        r * sin_theta * sin(phi),
        r * cos(theta),
    );
}

// Approximate Carter constants from flat-space angular momentum
fn kerr_init_consts(pos: vec3<f32>, dir: vec3<f32>) -> KerrConsts {
    let l = cross(pos, dir);
    let l2 = dot(l, l);
    return KerrConsts(
        1.0,        // E
        l.z,        // Lz
        max(0.0, l2 - l.z * l.z),  // Q
    );
}

// Initialize a Kerr ray from Cartesian position and direction
fn kerr_init_ray(pos: vec3<f32>, dir: vec3<f32>) -> KerrRay {
    let r = length(pos);
    let inv_r = select(0.0, 1.0 / r, r > KERR_EPSILON);
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

    return KerrRay(
        r,
        theta,
        phi,
        0.0,
        select(-1.0, 1.0, dr >= 0.0),
        select(-1.0, 1.0, dtheta >= 0.0),
    );
}

// Step the Kerr ray forward by dlam in affine parameter.
// Uses ptr<function, KerrRay> since WGSL does not support inout parameters.
fn kerr_step(ray: ptr<function, KerrRay>, r_s: f32, a: f32, c: KerrConsts, dlam: f32) {
    let r = (*ray).r;
    let theta = (*ray).theta;
    let sin_theta = sin(theta);
    let cos_theta = cos(theta);
    let sin2 = max(sin_theta * sin_theta, 1e-6);

    let delta = kerr_delta(r, a, r_s);
    let big_a = (r * r + a * a) * c.e - a * c.lz;
    let lz_minus_a_e = c.lz - a * c.e;

    var big_r = big_a * big_a - delta * (c.q + lz_minus_a_e * lz_minus_a_e);
    var big_theta = c.q + (a * a * c.e * c.e * cos_theta * cos_theta) -
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

    // PI imported via physics_constants scope
    (*ray).theta = clamp((*ray).theta, 1e-6, 3.14159265358979323846 - 1e-6);
}
