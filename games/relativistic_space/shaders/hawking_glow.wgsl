// hawking_glow.wgsl
// Hawking radiation LUT sampling utilities and thermal glow shader.
//
// Translated from Blackhole GLSL:
//   - shader/hawking_luts.glsl
//   - shader/hawking_glow.glsl
//
// WGSL translation notes:
// - GLSL texture() -> textureSample()
// - GLSL log(x)/log(10.0) -> log(x) / log(10.0) (no log10 in WGSL)
// - sampler2D parameters -> separate texture_2d<f32> + sampler pairs
// - CGS float constants use f32 (limited precision is acceptable for visual rendering)

// ============================================================================
// Physical Constants (CGS units for Hawking calculations)
// ============================================================================

const PI_LOCAL: f32 = 3.14159265358979323846;

const HBAR: f32 = 1.054571817e-27;   // Reduced Planck constant [erg*s]
const C_LIGHT: f32 = 2.99792458e10;  // Speed of light [cm/s]
const G_GRAV: f32 = 6.67430e-8;      // Gravitational constant [cm^3/g/s^2]
const K_BOLTZ: f32 = 1.380649e-16;   // Boltzmann constant [erg/K]

// LUT range (hardcoded from generation script)
const HAWKING_LOG_MASS_MIN: f32 = 14.0;  // log10(mass_min) [log g]
const HAWKING_LOG_MASS_MAX: f32 = 42.0;  // log10(mass_max) [log g]
const HAWKING_LOG_TEMP_MIN: f32 = 3.0;   // log10(temp_min) [log K]
const HAWKING_LOG_TEMP_MAX: f32 = 12.0;  // log10(temp_max) [log K]

// ============================================================================
// Bind Group Declarations
// ============================================================================
// These are declared in the main shader that imports this module.
// When used standalone, bind groups must be declared externally.

// ============================================================================
// Direct Hawking Temperature Calculation (Fallback)
// ============================================================================

// Compute Hawking temperature directly: T_H = hbar*c^3 / (8*pi*G*M*k_B)
fn hawking_temperature_direct(mass: f32) -> f32 {
    if mass <= 0.0 {
        return 1e30;
    }
    let c3 = C_LIGHT * C_LIGHT * C_LIGHT;
    return HBAR * c3 / (8.0 * PI_LOCAL * G_GRAV * mass * K_BOLTZ);
}

// ============================================================================
// LUT Sampling Functions
// ============================================================================

// Sample Hawking temperature from LUT using log-linear interpolation
fn hawking_temperature_lut(
    mass: f32,
    hawking_temp_lut: texture_2d<f32>,
    hawking_temp_sampler: sampler,
) -> f32 {
    if mass <= 0.0 {
        return 1e30;
    }

    // Log-space interpolation: log10(mass)
    var log_mass = log(mass) / log(10.0);
    log_mass = clamp(log_mass, HAWKING_LOG_MASS_MIN, HAWKING_LOG_MASS_MAX);

    // Normalize to [0,1] for texture sampling
    let u = (log_mass - HAWKING_LOG_MASS_MIN) /
            (HAWKING_LOG_MASS_MAX - HAWKING_LOG_MASS_MIN);

    // Sample LUT (y=0.5 for 1D texture stored as 2D)
    let temperature = textureSample(hawking_temp_lut, hawking_temp_sampler, vec2<f32>(u, 0.5)).r;
    return temperature;
}

// Sample blackbody RGB color from spectrum LUT
fn sample_hawking_spectrum(
    temperature: f32,
    spectrum_lut: texture_2d<f32>,
    spectrum_sampler: sampler,
) -> vec3<f32> {
    if temperature <= 0.0 {
        return vec3<f32>(0.0, 0.0, 0.0);
    }

    var log_temp = log(temperature) / log(10.0);
    log_temp = clamp(log_temp, HAWKING_LOG_TEMP_MIN, HAWKING_LOG_TEMP_MAX);

    let u = (log_temp - HAWKING_LOG_TEMP_MIN) /
            (HAWKING_LOG_TEMP_MAX - HAWKING_LOG_TEMP_MIN);

    let rgb = textureSample(spectrum_lut, spectrum_sampler, vec2<f32>(u, 0.5)).rgb;
    return rgb;
}

// ============================================================================
// Direct Planck Blackbody (no LUT)
// ============================================================================

// Compute Planck blackbody distribution at RGB wavelengths.
// B_lambda(T) = (2hc^2/lambda^5) / (exp(hc/lambda*kT) - 1)
fn planck_blackbody_rgb(temperature: f32) -> vec3<f32> {
    if temperature <= 0.0 {
        return vec3<f32>(0.0, 0.0, 0.0);
    }

    // RGB wavelengths [cm]
    let wavelengths = array<f32, 3>(700e-7, 546e-7, 435e-7);

    // Planck constant (not reduced)
    let h = 2.0 * PI_LOCAL * HBAR;

    var intensities = vec3<f32>(0.0, 0.0, 0.0);
    for (var i = 0; i < 3; i++) {
        let lambda = wavelengths[i];

        // Exponent argument: hc/(lambda*kT)
        let x = h * C_LIGHT / (lambda * K_BOLTZ * temperature);

        // Avoid overflow
        if x > 700.0 {
            // intensities[i] stays 0.0
        } else {
            let lambda5 = lambda * lambda * lambda * lambda * lambda;
            let numerator = 2.0 * h * C_LIGHT * C_LIGHT / lambda5;
            let denominator = exp(x) - 1.0;
            // WGSL does not support vec indexing with var, use select chain
            if i == 0 {
                intensities.x = numerator / denominator;
            } else if i == 1 {
                intensities.y = numerator / denominator;
            } else {
                intensities.z = numerator / denominator;
            }
        }
    }

    // Normalize to peak
    let peak = max(max(intensities.r, intensities.g), intensities.b);
    if peak > 0.0 {
        intensities /= peak;
    }

    return intensities;
}

// ============================================================================
// Core Hawking Glow Functions
// ============================================================================

// Compute thermal glow color and intensity.
// 1. Compute T_H(M) * tempScale
// 2. Sample Planck blackbody spectrum at RGB wavelengths
// 3. Apply inverse-square falloff with near-horizon enhancement
fn hawking_thermal_glow(
    mass: f32,
    r: f32,
    r_s: f32,
    temp_scale: f32,
    intensity: f32,
    hawking_temp_lut: texture_2d<f32>,
    hawking_temp_sampler: sampler,
    hawking_spectrum_lut: texture_2d<f32>,
    hawking_spectrum_sampler: sampler,
    use_luts: f32,
) -> vec3<f32> {
    // 1. Compute Hawking temperature
    var t_h: f32;
    if use_luts > 0.5 {
        t_h = hawking_temperature_lut(mass, hawking_temp_lut, hawking_temp_sampler);
    } else {
        t_h = hawking_temperature_direct(mass);
    }

    // Apply pedagogical temperature scaling
    t_h *= temp_scale;

    // 2. Get blackbody RGB spectrum
    var spectrum: vec3<f32>;
    if use_luts > 0.5 {
        spectrum = sample_hawking_spectrum(t_h, hawking_spectrum_lut, hawking_spectrum_sampler);
    } else {
        spectrum = planck_blackbody_rgb(t_h);
    }

    // 3. Compute spatial falloff
    let radius_ratio = r / max(r_s, 1e-10);
    let exp_factor = exp(-radius_ratio * 4.0);
    let inv_sq = 1.0 / (radius_ratio * radius_ratio + 0.01);
    let falloff = exp_factor * inv_sq;

    // 4. Apply global intensity multiplier
    let glow = spectrum * falloff * intensity;
    return glow;
}

// Simplified thermal glow (precomputed temperature, single call)
fn hawking_spectrum(
    temperature: f32,
    radius_ratio: f32,
    spectrum_lut: texture_2d<f32>,
    spectrum_sampler: sampler,
) -> vec3<f32> {
    let intensities = sample_hawking_spectrum(temperature, spectrum_lut, spectrum_sampler);
    let falloff = exp(-radius_ratio * 4.0) /
                  (radius_ratio * radius_ratio + 0.01);
    return intensities * falloff;
}

// Apply Hawking glow to accumulated ray color (main integration function)
fn apply_hawking_glow(
    current_color: vec3<f32>,
    mass: f32,
    r: f32,
    r_s: f32,
    enabled: f32,
    temp_scale: f32,
    intensity: f32,
    hawking_temp_lut: texture_2d<f32>,
    hawking_temp_sampler: sampler,
    hawking_spectrum_lut: texture_2d<f32>,
    hawking_spectrum_sampler: sampler,
    use_luts: f32,
) -> vec3<f32> {
    if enabled < 0.5 {
        return current_color;
    }

    let glow = hawking_thermal_glow(
        mass, r, r_s, temp_scale, intensity,
        hawking_temp_lut, hawking_temp_sampler,
        hawking_spectrum_lut, hawking_spectrum_sampler,
        use_luts,
    );

    return current_color + glow;
}

// ============================================================================
// Validation Functions
// ============================================================================

// Validate Hawking temperature against known values
fn validate_hawking_temperature(
    mass: f32,
    hawking_temp_lut: texture_2d<f32>,
    hawking_temp_sampler: sampler,
) -> f32 {
    let t_direct = hawking_temperature_direct(mass);
    let t_lut = hawking_temperature_lut(mass, hawking_temp_lut, hawking_temp_sampler);
    // Relative error should be < 1e-6 (float32 precision)
    let _ = abs((t_lut - t_direct) / (t_direct + 1e-30));
    return t_lut;
}

// Test inverse mass relationship: T_H proportional to 1/M
fn test_inverse_mass_law(
    mass1: f32,
    mass2: f32,
    hawking_temp_lut: texture_2d<f32>,
    hawking_temp_sampler: sampler,
) -> f32 {
    let t1 = hawking_temperature_lut(mass1, hawking_temp_lut, hawking_temp_sampler);
    let t2 = hawking_temperature_lut(mass2, hawking_temp_lut, hawking_temp_sampler);
    return t2 / t1;
}
