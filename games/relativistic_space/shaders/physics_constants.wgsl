// physics_constants.wgsl
// Physical constants, Schwarzschild metric functions, and verified physics kernels.
//
// Translated from Blackhole GLSL:
//   - shader/include/physics_constants.glsl
//   - shader/include/schwarzschild.glsl
//   - shader/include/verified/physics.glsl
//
// All functions use normalized units where r_s = 1.0 (pass actual r_s as parameter).

// ============================================================================
// Mathematical Constants
// ============================================================================

const PI: f32 = 3.14159265358979323846;
const TWO_PI: f32 = 6.28318530717958647692;
const HALF_PI: f32 = 1.57079632679489661923;
const INV_PI: f32 = 0.31830988618379067154;

// ============================================================================
// Physical Ratios (dimensionless, exact)
// ============================================================================

// Photon sphere radius in Schwarzschild radii: r_ph = 1.5 r_s
const PHOTON_SPHERE_RATIO: f32 = 1.5;

// ISCO radius in Schwarzschild radii: r_ISCO = 3 r_s
const ISCO_RATIO: f32 = 3.0;

// Critical impact parameter: b_crit = (3*sqrt(3)/2) r_s
const CRITICAL_IMPACT_RATIO: f32 = 2.598076211353316;

// ============================================================================
// Rendering Tolerances
// ============================================================================

// Minimum radius for ray termination (avoid singularity)
const MIN_RADIUS_FACTOR: f32 = 1.001;

// Maximum ray length in Schwarzschild radii
const MAX_RAY_LENGTH: f32 = 100.0;

// Step size factor for ray marching
const RAY_STEP_FACTOR: f32 = 0.1;

// Convergence tolerance for iterative calculations
const CONVERGENCE_TOL: f32 = 1e-6;

// ============================================================================
// Core Schwarzschild Metric Functions
// ============================================================================

// Metric factor f(r) = 1 - r_s/r
fn metric_factor(r: f32, r_s: f32) -> f32 {
    return 1.0 - r_s / r;
}

// Photon sphere radius: r_ph = 1.5 r_s
fn sch_photon_sphere_radius(r_s: f32) -> f32 {
    return 1.5 * r_s;
}

// ISCO radius: r_ISCO = 3 r_s
fn sch_isco_radius(r_s: f32) -> f32 {
    return 3.0 * r_s;
}

// Critical impact parameter: b_crit = (3*sqrt(3)/2) r_s
fn critical_impact_parameter(r_s: f32) -> f32 {
    return 2.598076211353316 * r_s;
}

// ============================================================================
// Gravitational Effects
// ============================================================================

// Gravitational redshift factor: z = 1/sqrt(1 - r_s/r) - 1
fn gravitational_redshift_sch(r: f32, r_s: f32) -> f32 {
    if r <= r_s {
        return 1e10;
    }
    let f = 1.0 - r_s / r;
    return 1.0 / sqrt(f) - 1.0;
}

// Time dilation factor: dtau/dt = sqrt(1 - r_s/r)
fn time_dilation_factor(r: f32, r_s: f32) -> f32 {
    if r <= r_s {
        return 0.0;
    }
    return sqrt(1.0 - r_s / r);
}

// Escape velocity ratio: v_esc/c = sqrt(r_s/r)
fn escape_velocity_ratio(r: f32, r_s: f32) -> f32 {
    if r <= r_s {
        return 1.0;
    }
    return sqrt(r_s / r);
}

// ============================================================================
// Light Deflection
// ============================================================================

// Gravitational deflection angle (weak field): dphi = 2*r_s/b
fn gravitational_deflection(b: f32, r_s: f32) -> f32 {
    return 2.0 * r_s / b;
}

// Check if photon will be captured (b < b_crit)
fn is_photon_captured(b: f32, r_s: f32) -> bool {
    return b < critical_impact_parameter(r_s);
}

// ============================================================================
// Ray Tracing Helpers
// ============================================================================

// Effective potential for null geodesics: V_eff(r) = (1 - r_s/r) / r^2
fn null_effective_potential(r: f32, r_s: f32) -> f32 {
    return (1.0 - r_s / r) / (r * r);
}

// Squared radial derivative for null geodesics
fn null_radial_deriv_sq(r: f32, r_s: f32, b: f32) -> f32 {
    let r2 = r * r;
    let r4 = r2 * r2;
    let b2 = b * b;
    return r4 / b2 - r2 * (1.0 - r_s / r);
}

// Bending function for ray direction update
fn bending_angle(r: f32, r_s: f32, step_size: f32) -> f32 {
    let r2 = r * r;
    return 1.5 * r_s * step_size / r2;
}

// ============================================================================
// Accretion Disk Helpers
// ============================================================================

// Keplerian angular velocity: Omega = sqrt(r_s/(2*r^3))
fn keplerian_angular_velocity(r: f32, r_s: f32) -> f32 {
    if r <= sch_isco_radius(r_s) {
        return 0.0;
    }
    return sqrt(r_s / (2.0 * r * r * r));
}

// Orbital velocity: v/c = sqrt(r_s/(2*r))
fn orbital_velocity_ratio(r: f32, r_s: f32) -> f32 {
    if r <= r_s {
        return 1.0;
    }
    return sqrt(r_s / (2.0 * r));
}

// Doppler factor for orbiting matter
fn total_doppler_factor(r: f32, r_s: f32, view_angle: f32) -> f32 {
    let grav_factor = time_dilation_factor(r, r_s);
    let v = orbital_velocity_ratio(r, r_s);
    let doppler = sqrt((1.0 - v * cos(view_angle)) / (1.0 + v * cos(view_angle)));
    return grav_factor * doppler;
}

// ============================================================================
// Verified Physics Kernels (from Rocq formalization)
// ============================================================================

// Schwarzschild radius: r_s = 2M (geometric units)
fn schwarzschild_radius(m: f32) -> f32 {
    return 2.0 * m;
}

// ISCO radius: r_ISCO = 6M (geometric units)
fn schwarzschild_isco(m: f32) -> f32 {
    return 6.0 * m;
}

// Photon sphere radius: r_ph = 3M (geometric units)
fn photon_sphere_radius_m(m: f32) -> f32 {
    return 3.0 * m;
}

// Schwarzschild metric factor: f(r) = 1 - 2M/r
fn f_schwarzschild(r: f32, m: f32) -> f32 {
    return 1.0 - (2.0 * m) / r;
}

// g_tt component: -(1 - 2M/r)
fn schwarzschild_g_tt(r: f32, m: f32) -> f32 {
    return -f_schwarzschild(r, m);
}

// g_rr component: 1/(1 - 2M/r)
fn schwarzschild_g_rr(r: f32, m: f32) -> f32 {
    return 1.0 / f_schwarzschild(r, m);
}

// g_thth component: r^2
fn schwarzschild_g_thth(r: f32) -> f32 {
    return r * r;
}

// g_phph component: r^2 sin^2(theta)
fn schwarzschild_g_phph(r: f32, theta: f32) -> f32 {
    let sin_theta = sin(theta);
    return r * r * sin_theta * sin_theta;
}

// ============================================================================
// Christoffel Symbols (Schwarzschild)
// ============================================================================

fn christoffel_t_tr(r: f32, m: f32) -> f32 {
    return m / (r * (r - 2.0 * m));
}

fn christoffel_r_tt(r: f32, m: f32) -> f32 {
    return m * (r - 2.0 * m) / (r * r * r);
}

fn christoffel_r_rr(r: f32, m: f32) -> f32 {
    return -m / (r * (r - 2.0 * m));
}

fn christoffel_r_thth(r: f32, m: f32) -> f32 {
    return -(r - 2.0 * m);
}

fn christoffel_r_phph(r: f32, theta: f32, m: f32) -> f32 {
    let sin_theta = sin(theta);
    return -(r - 2.0 * m) * sin_theta * sin_theta;
}

fn christoffel_th_rth(r: f32) -> f32 {
    return 1.0 / r;
}

fn christoffel_th_phph(theta: f32) -> f32 {
    return -sin(theta) * cos(theta);
}

fn christoffel_ph_rph(r: f32) -> f32 {
    return 1.0 / r;
}

fn christoffel_ph_thph(theta: f32) -> f32 {
    return cos(theta) / sin(theta);
}

// ============================================================================
// Kerr ISCO (BPT formula, corrected Phase 6)
// ============================================================================

// BPT Z1 helper
fn bpt_z1(a: f32) -> f32 {
    let a2 = a * a;
    let one_minus_a2 = 1.0 - a2;
    return 1.0 + pow(one_minus_a2 / 2.0, 1.0 / 3.0) * (
        pow(1.0 + a, 1.0 / 3.0) + pow(1.0 - a, 1.0 / 3.0)
    );
}

// BPT Z2 (corrected formula): Z2 = sqrt(3*a^2 + Z1^2)
fn bpt_z2_corrected(a: f32, z1: f32) -> f32 {
    let a2 = a * a;
    let term = 3.0 * a2 + z1 * z1;
    return sqrt(max(term, 0.0));
}

// ISCO radius for prograde orbits (Kerr)
fn isco_radius_prograde(m: f32, a: f32) -> f32 {
    let z1 = bpt_z1(a);
    let z2 = bpt_z2_corrected(a, z1);
    let factor = (3.0 - z1) * (3.0 + z1 + 2.0 * z2);
    return m * (3.0 + z2 - sqrt(max(factor, 0.0)));
}

// ISCO radius for retrograde orbits (Kerr)
fn isco_radius_retrograde(m: f32, a: f32) -> f32 {
    let z1 = bpt_z1(a);
    let z2 = bpt_z2_corrected(a, z1);
    let factor = (3.0 - z1) * (3.0 + z1 + 2.0 * z2);
    return m * (3.0 + z2 + sqrt(max(factor, 0.0)));
}

// ============================================================================
// Curvature Invariants
// ============================================================================

// Ricci scalar (Schwarzschild): R = 0 (vacuum solution)
fn ricci_scalar_schwarzschild(r: f32, m: f32) -> f32 {
    // Suppress unused parameter warnings through use
    let _ = r;
    let _ = m;
    return 0.0;
}

// Kretschmann scalar (Schwarzschild): K = 48 M^2 / r^6
fn kretschmann_schwarzschild(r: f32, m: f32) -> f32 {
    let r6 = r * r * r * r * r * r;
    return 48.0 * m * m / r6;
}

// Validation helpers
fn outside_horizon_schwarzschild(r: f32, m: f32) -> bool {
    return r > schwarzschild_radius(m);
}

fn outside_photon_sphere(r: f32, m: f32) -> bool {
    return r > photon_sphere_radius_m(m);
}

fn outside_isco_schwarzschild(r: f32, m: f32) -> bool {
    return r > schwarzschild_isco(m);
}

// Redshift factor for ray tracing: sqrt(1 - r_s/r)
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
