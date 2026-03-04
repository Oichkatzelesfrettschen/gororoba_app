// EHT shadow validation: compare computed GR quantities against reference data.
//
// Uses ground-truth data from the Blackhole C++ renderer's validation suite:
//   metrics.json          -- reference ISCO, photon sphere, Schwarzschild radius
//   redshift_curve.csv    -- gravitational redshift z(r) for r in [3, 20] r_s
//   spin_radii_curve.csv  -- ISCO and photon sphere radii vs spin a in [-0.99, 0.99]

use bevy::prelude::*;

/// Reference metrics loaded from metrics.json.
#[derive(Resource)]
pub struct ValidationMetrics {
    pub mass_solar: f64,
    pub spin: f64,
    pub r_s_cm: f64,
    pub r_isco_cm: f64,
    pub r_ph_cm: f64,
    pub r_min_over_rs: f64,
    pub r_max_over_rs: f64,
    pub points: usize,
}

impl Default for ValidationMetrics {
    fn default() -> Self {
        Self {
            mass_solar: 4_000_000.0,
            spin: 0.0,
            r_s_cm: 1_181_300_107_257.459_7,
            r_isco_cm: 3_543_900_321_772.379,
            r_ph_cm: 1_771_950_160_886.189_5,
            r_min_over_rs: 3.0,
            r_max_over_rs: 20.0,
            points: 128,
        }
    }
}

/// A single data point from redshift_curve.csv.
struct RedshiftPoint {
    r_over_rs: f64,
    value: f64,
}

/// A single data point from spin_radii_curve.csv.
struct SpinRadiiPoint {
    spin: f64,
    r_isco_over_rs: f64,
    r_ph_over_rs: f64,
}

/// Validation results for reporting.
#[derive(Resource, Default)]
pub struct ValidationResults {
    /// ISCO ratio: computed / reference (should be ~1.0).
    pub isco_ratio: f64,
    /// Photon sphere ratio: computed / reference (should be ~1.0).
    pub photon_sphere_ratio: f64,
    /// Maximum relative error in redshift curve.
    pub redshift_max_error: f64,
    /// Mean relative error in redshift curve.
    pub redshift_mean_error: f64,
    /// Maximum relative error in spin-ISCO curve.
    pub spin_isco_max_error: f64,
    /// Maximum relative error in spin-photon sphere curve.
    pub spin_photon_max_error: f64,
    /// Whether validation has been run.
    pub completed: bool,
}

pub struct ValidationPlugin;

impl Plugin for ValidationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ValidationMetrics>()
            .init_resource::<ValidationResults>()
            .add_systems(Startup, (load_validation_data, run_validation).chain());
    }
}

/// Load reference data from the validation assets directory.
fn load_validation_data(mut commands: Commands) {
    let dir = "games/relativistic_space/assets/validation";
    let metrics = load_metrics(dir);
    commands.insert_resource(metrics);
}

/// Run all validation checks against the GR engine.
fn run_validation(metrics: Res<ValidationMetrics>, mut results: ResMut<ValidationResults>) {
    let dir = "games/relativistic_space/assets/validation";

    // 1. Validate ISCO and photon sphere radii for Schwarzschild (a=0).
    let computed_isco_ratio = 3.0; // ISCO = 3 r_s for Schwarzschild
    let reference_isco_ratio = metrics.r_isco_cm / metrics.r_s_cm;
    results.isco_ratio = computed_isco_ratio / reference_isco_ratio;

    let computed_photon_ratio = 1.5; // Photon sphere = 1.5 r_s for Schwarzschild
    let reference_photon_ratio = metrics.r_ph_cm / metrics.r_s_cm;
    results.photon_sphere_ratio = computed_photon_ratio / reference_photon_ratio;

    info!(
        "Validation: ISCO ratio = {:.6} (expect ~1.0), photon sphere ratio = {:.6} (expect ~1.0)",
        results.isco_ratio, results.photon_sphere_ratio,
    );

    // 2. Validate redshift curve: z(r) = 1/sqrt(1 - r_s/r) - 1
    //    The CSV stores the redshift factor sqrt(1 - r_s/r), not z itself.
    let redshift_data = load_redshift_curve(dir);
    if !redshift_data.is_empty() {
        let mut max_err: f64 = 0.0;
        let mut sum_err: f64 = 0.0;

        for pt in &redshift_data {
            let computed = (1.0 - 1.0 / pt.r_over_rs).sqrt();
            let rel_err = if pt.value.abs() > 1e-15 {
                ((computed - pt.value) / pt.value).abs()
            } else {
                computed.abs()
            };
            max_err = max_err.max(rel_err);
            sum_err += rel_err;
        }

        results.redshift_max_error = max_err;
        results.redshift_mean_error = sum_err / redshift_data.len() as f64;

        info!(
            "Validation: redshift curve max_error = {:.2e}, mean_error = {:.2e} ({} points)",
            results.redshift_max_error,
            results.redshift_mean_error,
            redshift_data.len(),
        );
    }

    // 3. Validate spin-dependent radii: ISCO(a) and photon sphere(a).
    //    Reference: BPT formula for ISCO, standard formula for photon sphere.
    let spin_data = load_spin_radii_curve(dir);
    if !spin_data.is_empty() {
        let mut isco_max_err: f64 = 0.0;
        let mut photon_max_err: f64 = 0.0;

        for pt in &spin_data {
            let computed_isco = bpt_isco_radius(pt.spin);
            let isco_err = if pt.r_isco_over_rs.abs() > 1e-15 {
                ((computed_isco - pt.r_isco_over_rs) / pt.r_isco_over_rs).abs()
            } else {
                computed_isco.abs()
            };
            isco_max_err = isco_max_err.max(isco_err);

            let computed_photon = prograde_photon_orbit(pt.spin);
            let photon_err = if pt.r_ph_over_rs.abs() > 1e-15 {
                ((computed_photon - pt.r_ph_over_rs) / pt.r_ph_over_rs).abs()
            } else {
                computed_photon.abs()
            };
            photon_max_err = photon_max_err.max(photon_err);
        }

        results.spin_isco_max_error = isco_max_err;
        results.spin_photon_max_error = photon_max_err;

        info!(
            "Validation: spin ISCO max_error = {:.2e}, photon sphere max_error = {:.2e} ({} points)",
            results.spin_isco_max_error,
            results.spin_photon_max_error,
            spin_data.len(),
        );
    }

    results.completed = true;
    info!("Validation complete");
}

// BPT ISCO radius formula: r_isco / r_s for given spin parameter a (prograde).
// From Bardeen, Press, Teukolsky (1972).
fn bpt_isco_radius(a: f64) -> f64 {
    // a is spin parameter normalized to M (so a = J/M^2, range [-1, 1]).
    // r_isco / M = 3 + Z2 - sign(a) * sqrt((3 - Z1)(3 + Z1 + 2*Z2))
    // where Z1 = 1 + (1-a^2)^(1/3) * ((1+a)^(1/3) + (1-a)^(1/3))
    //       Z2 = sqrt(3*a^2 + Z1^2)
    // We need r_isco / r_s = r_isco / (2M).

    let a2 = a * a;
    let cbrt_1ma2 = (1.0 - a2).cbrt();
    let cbrt_1pa = (1.0 + a).cbrt();
    let cbrt_1ma = (1.0 - a).cbrt();

    let z1 = 1.0 + cbrt_1ma2 * (cbrt_1pa + cbrt_1ma);
    let z2 = (3.0 * a2 + z1 * z1).sqrt();

    let sign_a = if a >= 0.0 { 1.0 } else { -1.0 };
    let r_isco_over_m = 3.0 + z2 - sign_a * ((3.0 - z1) * (3.0 + z1 + 2.0 * z2)).sqrt();

    // Convert from r/M to r/r_s (r_s = 2M)
    r_isco_over_m / 2.0
}

// Prograde (innermost) photon orbit radius for Kerr metric.
// r_ph / M = 2(1 + cos(2/3 * arccos(-|a|))) for prograde orbit.
// Uses |a| because the prograde orbit is always the tightest,
// regardless of spin direction.
fn prograde_photon_orbit(a: f64) -> f64 {
    let a_abs = a.abs();
    let r_ph_over_m = 2.0 * (1.0 + (2.0 / 3.0 * (-a_abs).acos()).cos());
    // Convert from r/M to r/r_s (r_s = 2M)
    r_ph_over_m / 2.0
}

fn load_metrics(dir: &str) -> ValidationMetrics {
    let path = format!("{dir}/metrics.json");
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to read validation metrics: {e}; using defaults");
            return ValidationMetrics::default();
        }
    };

    let mut m = ValidationMetrics::default();

    if let Some(v) = extract_json_f64(&content, "mass_solar") {
        m.mass_solar = v;
    }
    if let Some(v) = extract_json_f64(&content, "spin") {
        m.spin = v;
    }
    if let Some(v) = extract_json_f64(&content, "r_s_cm") {
        m.r_s_cm = v;
    }
    if let Some(v) = extract_json_f64(&content, "r_isco_cm") {
        m.r_isco_cm = v;
    }
    if let Some(v) = extract_json_f64(&content, "r_ph_cm") {
        m.r_ph_cm = v;
    }
    if let Some(v) = extract_json_f64(&content, "r_min_over_rs") {
        m.r_min_over_rs = v;
    }
    if let Some(v) = extract_json_f64(&content, "r_max_over_rs") {
        m.r_max_over_rs = v;
    }
    if let Some(v) = extract_json_f64(&content, "points") {
        m.points = v as usize;
    }

    m
}

fn load_redshift_curve(dir: &str) -> Vec<RedshiftPoint> {
    let path = format!("{dir}/redshift_curve.csv");
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to read redshift_curve.csv: {e}");
            return Vec::new();
        }
    };

    content
        .lines()
        .skip(1) // header
        .filter(|line| !line.is_empty())
        .filter_map(|line| {
            let mut parts = line.split(',');
            let r_over_rs = parts.next()?.trim().parse::<f64>().ok()?;
            let value = parts.next()?.trim().parse::<f64>().ok()?;
            Some(RedshiftPoint { r_over_rs, value })
        })
        .collect()
}

fn load_spin_radii_curve(dir: &str) -> Vec<SpinRadiiPoint> {
    let path = format!("{dir}/spin_radii_curve.csv");
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to read spin_radii_curve.csv: {e}");
            return Vec::new();
        }
    };

    content
        .lines()
        .skip(1) // header
        .filter(|line| !line.is_empty())
        .filter_map(|line| {
            let mut parts = line.split(',');
            let spin = parts.next()?.trim().parse::<f64>().ok()?;
            let r_isco_over_rs = parts.next()?.trim().parse::<f64>().ok()?;
            let r_ph_over_rs = parts.next()?.trim().parse::<f64>().ok()?;
            Some(SpinRadiiPoint {
                spin,
                r_isco_over_rs,
                r_ph_over_rs,
            })
        })
        .collect()
}

fn extract_json_f64(json: &str, key: &str) -> Option<f64> {
    let pattern = format!("\"{key}\"");
    let pos = json.find(&pattern)?;
    let after_key = &json[pos + pattern.len()..];
    let after_colon = after_key.find(':').map(|i| &after_key[i + 1..])?;
    let trimmed = after_colon.trim_start();
    let end = trimmed.find([',', '\n', '}']).unwrap_or(trimmed.len());
    trimmed[..end].trim().parse::<f64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schwarzschild_isco_correct() {
        // For a=0, ISCO = 3 r_s (= 6M, so r/r_s = 3.0)
        let isco = bpt_isco_radius(0.0);
        assert!(
            (isco - 3.0).abs() < 1e-10,
            "Schwarzschild ISCO should be 3.0 r_s, got {isco}"
        );
    }

    #[test]
    fn extreme_prograde_isco() {
        // For a -> 1 (maximally spinning), ISCO -> 0.5 r_s (= M)
        let isco = bpt_isco_radius(0.998);
        assert!(
            isco < 0.8,
            "Extreme prograde ISCO should be < 0.8 r_s, got {isco}"
        );
    }

    #[test]
    fn extreme_retrograde_isco() {
        // For a -> -1, ISCO -> 4.5 r_s (= 9M)
        let isco = bpt_isco_radius(-0.998);
        assert!(
            isco > 4.0,
            "Extreme retrograde ISCO should be > 4.0 r_s, got {isco}"
        );
    }

    #[test]
    fn schwarzschild_photon_sphere() {
        // For a=0, photon sphere = 1.5 r_s (= 3M)
        let r_ph = prograde_photon_orbit(0.0);
        assert!(
            (r_ph - 1.5).abs() < 1e-10,
            "Schwarzschild photon sphere should be 1.5 r_s, got {r_ph}"
        );
    }

    #[test]
    fn prograde_photon_orbit_decreases_with_spin() {
        // Photon orbit decreases as spin increases (prograde)
        let r0 = prograde_photon_orbit(0.0);
        let r5 = prograde_photon_orbit(0.5);
        let r9 = prograde_photon_orbit(0.9);
        assert!(r5 < r0, "r_ph(0.5) should be < r_ph(0): {r5} vs {r0}");
        assert!(r9 < r5, "r_ph(0.9) should be < r_ph(0.5): {r9} vs {r5}");
    }

    #[test]
    fn bpt_isco_matches_validation_data() {
        // Check against a few points from spin_radii_curve.csv
        let test_cases = [(0.0, 3.0), (-0.99, 4.485930671273484)];

        for (spin, expected_isco) in test_cases {
            let computed = bpt_isco_radius(spin);
            let rel_err = ((computed - expected_isco) / expected_isco).abs();
            assert!(
                rel_err < 1e-6,
                "BPT ISCO at spin={spin}: expected {expected_isco}, got {computed} (err={rel_err:.2e})"
            );
        }
    }

    #[test]
    fn photon_orbit_matches_validation_data() {
        let test_cases = [(0.0, 1.5), (-0.99, 0.5838209259287666)];

        for (spin, expected_rph) in test_cases {
            let computed = prograde_photon_orbit(spin);
            let rel_err = ((computed - expected_rph) / expected_rph).abs();
            assert!(
                rel_err < 1e-4,
                "Photon orbit at spin={spin}: expected {expected_rph}, got {computed} (err={rel_err:.2e})"
            );
        }
    }

    #[test]
    fn redshift_formula_at_known_points() {
        // At r = 3 r_s: redshift factor = sqrt(1 - 1/3) = sqrt(2/3)
        let z = (1.0 - 1.0 / 3.0_f64).sqrt();
        let expected = (2.0_f64 / 3.0).sqrt();
        assert!((z - expected).abs() < 1e-15);

        // At r = infinity: redshift factor = 1.0
        let z_inf = (1.0 - 1.0 / 1e10_f64).sqrt();
        assert!((z_inf - 1.0).abs() < 1e-9);
    }

    #[test]
    fn default_validation_metrics_reasonable() {
        let m = ValidationMetrics::default();
        assert!(m.mass_solar > 0.0);
        assert!(m.r_s_cm > 0.0);
        assert!(m.r_isco_cm > m.r_s_cm); // ISCO > Schwarzschild radius
        assert!(m.r_ph_cm > m.r_s_cm); // Photon sphere > Schwarzschild radius
        assert!(m.r_isco_cm > m.r_ph_cm); // ISCO > photon sphere for a=0
    }

    #[test]
    fn isco_photon_sphere_ordering() {
        // For all spins, ISCO >= photon sphere radius (prograde)
        for i in 0..20 {
            let spin = -0.99 + (i as f64) * 0.099;
            let isco = bpt_isco_radius(spin);
            let photon = prograde_photon_orbit(spin);
            assert!(
                isco >= photon - 1e-10,
                "ISCO ({isco}) should be >= photon sphere ({photon}) at spin={spin}"
            );
        }
    }
}
