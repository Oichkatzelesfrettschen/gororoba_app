// redshift.wgsl
// Gravitational and Doppler redshift functions for color correction.
//
// Translated from Blackhole GLSL: shader/include/redshift.glsl
//
// WGSL translation notes:
// - GLSL function overloading (relativisticBeaming) replaced with
//   distinct function names: relativistic_beaming and relativistic_beaming_thermal
// - vec3(r, g, b) * factor -> vec3<f32>(r, g, b) * factor

// ============================================================================
// Wavelength Constants (nanometers)
// ============================================================================

const WAVELENGTH_RED: f32 = 700.0;
const WAVELENGTH_GREEN: f32 = 546.0;
const WAVELENGTH_BLUE: f32 = 436.0;

const WAVELENGTH_MIN: f32 = 380.0;  // Violet
const WAVELENGTH_MAX: f32 = 780.0;  // Red

// ============================================================================
// Redshift Calculations
// ============================================================================

// Apply redshift to a wavelength: lambda_obs = lambda_emit * (1 + z)
fn apply_redshift(wavelength: f32, z: f32) -> f32 {
    return wavelength * (1.0 + z);
}

// Apply blueshift (negative redshift): lambda_obs = lambda_emit / (1 + |z|)
fn apply_blueshift(wavelength: f32, z: f32) -> f32 {
    return wavelength / (1.0 + abs(z));
}

// Gravitational redshift only (no Doppler): z_grav = 1/sqrt(1 - r_s/r) - 1
fn gravitational_redshift(r: f32, r_s: f32) -> f32 {
    let f = max(1.0 - r_s / r, 0.001);
    return 1.0 / sqrt(f) - 1.0;
}

// Combined gravitational + Doppler redshift
fn total_redshift(r: f32, r_s: f32, v_los: f32) -> f32 {
    let z_grav = gravitational_redshift(r, r_s);
    let v_clamped = clamp(v_los, -0.99, 0.99);
    let z_doppler = sqrt((1.0 + v_clamped) / (1.0 - v_clamped)) - 1.0;
    return (1.0 + z_grav) * (1.0 + z_doppler) - 1.0;
}

// ============================================================================
// Wavelength to Color Conversion
// ============================================================================

// Convert wavelength to RGB color using CIE color matching approximation
// (Dan Bruton's algorithm).
fn wavelength_to_rgb(wavelength: f32) -> vec3<f32> {
    var r: f32 = 0.0;
    var g: f32 = 0.0;
    var b: f32 = 0.0;

    if wavelength >= 380.0 && wavelength < 440.0 {
        r = -(wavelength - 440.0) / (440.0 - 380.0);
        g = 0.0;
        b = 1.0;
    } else if wavelength >= 440.0 && wavelength < 490.0 {
        r = 0.0;
        g = (wavelength - 440.0) / (490.0 - 440.0);
        b = 1.0;
    } else if wavelength >= 490.0 && wavelength < 510.0 {
        r = 0.0;
        g = 1.0;
        b = -(wavelength - 510.0) / (510.0 - 490.0);
    } else if wavelength >= 510.0 && wavelength < 580.0 {
        r = (wavelength - 510.0) / (580.0 - 510.0);
        g = 1.0;
        b = 0.0;
    } else if wavelength >= 580.0 && wavelength < 645.0 {
        r = 1.0;
        g = -(wavelength - 645.0) / (645.0 - 580.0);
        b = 0.0;
    } else if wavelength >= 645.0 && wavelength <= 780.0 {
        r = 1.0;
        g = 0.0;
        b = 0.0;
    }

    // Intensity falloff at edges of visible spectrum
    var factor: f32 = 1.0;
    if wavelength >= 380.0 && wavelength < 420.0 {
        factor = 0.3 + 0.7 * (wavelength - 380.0) / (420.0 - 380.0);
    } else if wavelength >= 700.0 && wavelength <= 780.0 {
        factor = 0.3 + 0.7 * (780.0 - wavelength) / (780.0 - 700.0);
    }

    // Handle out-of-range wavelengths
    if wavelength < 380.0 {
        // UV - show as faint violet
        r = 0.5;
        g = 0.0;
        b = 1.0;
        factor = max(0.0, 1.0 - (380.0 - wavelength) / 100.0);
    } else if wavelength > 780.0 {
        // IR - show as faint red
        r = 1.0;
        g = 0.0;
        b = 0.0;
        factor = max(0.0, 1.0 - (wavelength - 780.0) / 200.0);
    }

    return vec3<f32>(r, g, b) * factor;
}

// ============================================================================
// Color Shift Functions
// ============================================================================

// Apply gravitational redshift to RGB color.
// Converts to approximate wavelength, shifts, converts back.
fn apply_gravitational_redshift(color: vec3<f32>, z: f32) -> vec3<f32> {
    let luminance = dot(color, vec3<f32>(0.299, 0.587, 0.114));
    if luminance < 0.01 {
        return color;
    }

    let total = color.r + color.g + color.b + 0.001;
    let r_weight = color.r / total;
    let b_weight = color.b / total;

    var est_wavelength = 550.0 + 150.0 * (r_weight - b_weight);
    est_wavelength = clamp(est_wavelength, 400.0, 700.0);

    let shifted_wavelength = apply_redshift(est_wavelength, z);
    var shifted_color = wavelength_to_rgb(shifted_wavelength);

    let orig_intensity = length(color);
    let new_intensity = length(shifted_color);
    if new_intensity > 0.01 {
        shifted_color *= orig_intensity / new_intensity;
    }

    return shifted_color;
}

// Simple intensity-based redshift (faster, less accurate).
// Uses (1+z)^(-3) for specific intensity transformation (Liouville).
fn apply_simple_redshift(color: vec3<f32>, z: f32) -> vec3<f32> {
    let one_plus_z = 1.0 + z;
    let dimming = 1.0 / (one_plus_z * one_plus_z * one_plus_z);
    return color * dimming;
}

// Apply Doppler shift for moving source
fn apply_doppler_shift(color: vec3<f32>, v_los: f32) -> vec3<f32> {
    let v_clamped = clamp(v_los, -0.99, 0.99);
    let z = sqrt((1.0 + v_clamped) / (1.0 - v_clamped)) - 1.0;
    return apply_gravitational_redshift(color, z);
}

// ============================================================================
// Beaming Effect
// ============================================================================

// Relativistic beaming with thermal/non-thermal selection.
// is_thermal: true -> delta^4 (accretion disk), false -> delta^3 (jets)
fn relativistic_beaming(v_los: f32, is_thermal: bool) -> f32 {
    let v_clamped = clamp(v_los, -0.99, 0.99);
    let gamma = 1.0 / sqrt(1.0 - v_clamped * v_clamped);
    let delta = 1.0 / (gamma * (1.0 - v_clamped));

    if is_thermal {
        let d2 = delta * delta;
        return d2 * d2;
    } else {
        return delta * delta * delta;
    }
}

// Convenience wrapper: thermal beaming (delta^4) by default for disk emission
fn relativistic_beaming_thermal(v_los: f32) -> f32 {
    return relativistic_beaming(v_los, true);
}
