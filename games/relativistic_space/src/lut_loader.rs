// LUT asset loading: parse CSV lookup tables and create GPU textures.
//
// The Blackhole renderer uses 6 CSV lookup tables for various physical
// quantities. Each is loaded as an Nx1 RGBA32Float texture for GPU sampling
// via textureSample() in the WGSL shaders.
//
// CSV formats:
//   emissivity_lut.csv:      u,value (256 entries, no comments)
//   redshift_lut.csv:        u,value (256 entries, no comments)
//   grb_modulation_lut.csv:  time_s,value (512 entries, no comments)
//   hawking_temp_lut.csv:    Mass_g,Temperature_K,Radius_cm (512 entries, # comments)
//   hawking_spectrum_lut.csv: Temperature_K,Red,Green,Blue (256 entries, # comments)
//   spin_radii_lut.csv:      spin,r_isco_over_rs,r_ph_over_rs (64 entries, no comments)

use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// Parsed row from a single-value LUT (emissivity, redshift, GRB modulation).
struct SingleValueRow {
    value: f32,
}

/// Parsed row from the Hawking temperature LUT.
struct HawkingTempRow {
    mass_g: f32,
    temperature_k: f32,
    radius_cm: f32,
}

/// Parsed row from the Hawking spectrum LUT.
struct HawkingSpectrumRow {
    red: f32,
    green: f32,
    blue: f32,
}

/// Parsed row from the spin radii LUT.
struct SpinRadiiRow {
    spin: f32,
    r_isco_over_rs: f32,
    r_ph_over_rs: f32,
}

/// GPU-ready LUT texture handles for the black hole shaders.
#[derive(Resource)]
pub struct LutAssets {
    /// Novikov-Thorne emissivity profile (256x1 R32Float).
    pub emissivity: Handle<Image>,
    /// Gravitational redshift correction (256x1 R32Float).
    pub redshift: Handle<Image>,
    /// GRB temporal modulation (512x1 R32Float).
    pub grb_modulation: Handle<Image>,
    /// Hawking temperature T_H(M) (512x1 RGBA32Float: mass, temp, radius, 0).
    pub hawking_temp: Handle<Image>,
    /// Blackbody spectrum RGB(T) (256x1 RGBA32Float: R, G, B, 1).
    pub hawking_spectrum: Handle<Image>,
    /// Spin-dependent radii (64x1 RGBA32Float: spin, r_isco, r_ph, 0).
    pub spin_radii: Handle<Image>,
}

/// LUT metadata loaded from lut_meta.json.
#[derive(Resource)]
pub struct LutMeta {
    pub emissivity_model: String,
    pub spin: f64,
    pub r_in_over_rs: f64,
    pub r_out_over_rs: f64,
    pub size: u32,
}

impl Default for LutMeta {
    fn default() -> Self {
        Self {
            emissivity_model: "novikov-thorne".into(),
            spin: 0.0,
            r_in_over_rs: 3.0,
            r_out_over_rs: 12.0,
            size: 256,
        }
    }
}

pub struct LutLoaderPlugin;

impl Plugin for LutLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LutMeta>()
            .add_systems(Startup, load_lut_assets)
            .add_systems(Update, verify_lut_images.run_if(run_once));
    }
}

/// One-shot system that verifies all LUT images loaded with correct dimensions.
fn verify_lut_images(luts: Res<LutAssets>, images: Res<Assets<Image>>) {
    let checks = [
        ("emissivity", &luts.emissivity),
        ("redshift", &luts.redshift),
        ("grb_modulation", &luts.grb_modulation),
        ("hawking_temp", &luts.hawking_temp),
        ("hawking_spectrum", &luts.hawking_spectrum),
        ("spin_radii", &luts.spin_radii),
    ];

    for (name, handle) in checks {
        match images.get(handle) {
            Some(img) => {
                info!(
                    "LUT verified: {} = {}x{} {:?}",
                    name,
                    img.width(),
                    img.height(),
                    img.texture_descriptor.format,
                );
            }
            None => {
                warn!("LUT not yet loaded: {name}");
            }
        }
    }
}

/// Parse CSV content, skipping comment lines (starting with '#' or '"#')
/// and the header line.
fn parse_csv_lines(content: &str) -> impl Iterator<Item = &str> {
    content
        .lines()
        .filter(|line| !line.is_empty())
        .filter(|line| !line.starts_with('#') && !line.starts_with("\"#"))
        .skip(1) // skip header row
}

/// Parse a single-value CSV (u,value or time_s,value format).
fn parse_single_value_csv(content: &str) -> Vec<SingleValueRow> {
    parse_csv_lines(content)
        .filter_map(|line| {
            let mut parts = line.split(',');
            let _key = parts.next()?.trim().parse::<f32>().ok()?;
            let value = parts.next()?.trim().parse::<f32>().ok()?;
            Some(SingleValueRow { value })
        })
        .collect()
}

/// Parse the Hawking temperature CSV (Mass_g,Temperature_K,Radius_cm).
fn parse_hawking_temp_csv(content: &str) -> Vec<HawkingTempRow> {
    parse_csv_lines(content)
        .filter_map(|line| {
            let mut parts = line.split(',');
            let mass_g = parts.next()?.trim().parse::<f32>().ok()?;
            let temperature_k = parts.next()?.trim().parse::<f32>().ok()?;
            let radius_cm = parts.next()?.trim().parse::<f32>().ok()?;
            Some(HawkingTempRow {
                mass_g,
                temperature_k,
                radius_cm,
            })
        })
        .collect()
}

/// Parse the Hawking spectrum CSV (Temperature_K,Red,Green,Blue).
fn parse_hawking_spectrum_csv(content: &str) -> Vec<HawkingSpectrumRow> {
    parse_csv_lines(content)
        .filter_map(|line| {
            let mut parts = line.split(',');
            let _temperature_k = parts.next()?.trim().parse::<f32>().ok()?;
            let red = parts.next()?.trim().parse::<f32>().ok()?;
            let green = parts.next()?.trim().parse::<f32>().ok()?;
            let blue = parts.next()?.trim().parse::<f32>().ok()?;
            Some(HawkingSpectrumRow { red, green, blue })
        })
        .collect()
}

/// Parse the spin radii CSV (spin,r_isco_over_rs,r_ph_over_rs).
fn parse_spin_radii_csv(content: &str) -> Vec<SpinRadiiRow> {
    parse_csv_lines(content)
        .filter_map(|line| {
            let mut parts = line.split(',');
            let spin = parts.next()?.trim().parse::<f32>().ok()?;
            let r_isco_over_rs = parts.next()?.trim().parse::<f32>().ok()?;
            let r_ph_over_rs = parts.next()?.trim().parse::<f32>().ok()?;
            Some(SpinRadiiRow {
                spin,
                r_isco_over_rs,
                r_ph_over_rs,
            })
        })
        .collect()
}

/// Create an Nx1 R32Float image from single-channel data.
fn create_r32_texture(values: &[f32]) -> Image {
    let width = values.len() as u32;
    let data: Vec<u8> = values.iter().flat_map(|v| v.to_le_bytes()).collect();

    Image::new(
        Extent3d {
            width,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::R32Float,
        default(),
    )
}

/// Create an Nx1 RGBA32Float image from 4-channel interleaved data.
fn create_rgba32_texture(pixel_data: &[[f32; 4]]) -> Image {
    let width = pixel_data.len() as u32;
    let data: Vec<u8> = pixel_data
        .iter()
        .flat_map(|rgba| rgba.iter().flat_map(|v| v.to_le_bytes()))
        .collect();

    Image::new(
        Extent3d {
            width,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba32Float,
        default(),
    )
}

/// Load all LUT CSV files at startup and create GPU textures.
fn load_lut_assets(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let lut_dir = "games/relativistic_space/assets/luts";

    // Emissivity LUT: u,value -> R32Float
    let emissivity = load_single_value_lut(lut_dir, "emissivity_lut.csv", &mut images);

    // Redshift LUT: u,value -> R32Float
    let redshift = load_single_value_lut(lut_dir, "redshift_lut.csv", &mut images);

    // GRB modulation LUT: time_s,value -> R32Float
    let grb_modulation = load_single_value_lut(lut_dir, "grb_modulation_lut.csv", &mut images);

    // Hawking temperature LUT: Mass_g,Temperature_K,Radius_cm -> RGBA32Float
    let hawking_temp = load_hawking_temp_lut(lut_dir, &mut images);

    // Hawking spectrum LUT: Temperature_K,Red,Green,Blue -> RGBA32Float
    let hawking_spectrum = load_hawking_spectrum_lut(lut_dir, &mut images);

    // Spin radii LUT: spin,r_isco_over_rs,r_ph_over_rs -> RGBA32Float
    let spin_radii = load_spin_radii_lut(lut_dir, &mut images);

    // Parse metadata
    let meta = load_lut_meta(lut_dir);

    commands.insert_resource(LutAssets {
        emissivity,
        redshift,
        grb_modulation,
        hawking_temp,
        hawking_spectrum,
        spin_radii,
    });
    commands.insert_resource(meta);

    info!("LUT assets loaded successfully");
}

/// Load a single-value CSV (u,value) and return a texture handle.
fn load_single_value_lut(dir: &str, filename: &str, images: &mut Assets<Image>) -> Handle<Image> {
    let path = format!("{dir}/{filename}");
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to read LUT {path}: {e}; creating fallback");
            return images.add(create_r32_texture(&[0.0]));
        }
    };

    let rows = parse_single_value_csv(&content);
    if rows.is_empty() {
        warn!("LUT {path} has no data rows; creating fallback");
        return images.add(create_r32_texture(&[0.0]));
    }

    let values: Vec<f32> = rows.iter().map(|r| r.value).collect();
    let image = create_r32_texture(&values);
    info!("Loaded {filename}: {} entries", values.len());
    images.add(image)
}

/// Load the Hawking temperature LUT and return a texture handle.
fn load_hawking_temp_lut(dir: &str, images: &mut Assets<Image>) -> Handle<Image> {
    let path = format!("{dir}/hawking_temp_lut.csv");
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to read Hawking temp LUT: {e}; creating fallback");
            return images.add(create_rgba32_texture(&[[0.0; 4]]));
        }
    };

    let rows = parse_hawking_temp_csv(&content);
    if rows.is_empty() {
        warn!("Hawking temp LUT has no data rows; creating fallback");
        return images.add(create_rgba32_texture(&[[0.0; 4]]));
    }

    let pixel_data: Vec<[f32; 4]> = rows
        .iter()
        .map(|r| [r.mass_g, r.temperature_k, r.radius_cm, 0.0])
        .collect();

    let image = create_rgba32_texture(&pixel_data);
    info!("Loaded hawking_temp_lut.csv: {} entries", pixel_data.len());
    images.add(image)
}

/// Load the Hawking spectrum LUT and return a texture handle.
fn load_hawking_spectrum_lut(dir: &str, images: &mut Assets<Image>) -> Handle<Image> {
    let path = format!("{dir}/hawking_spectrum_lut.csv");
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to read Hawking spectrum LUT: {e}; creating fallback");
            return images.add(create_rgba32_texture(&[[0.0; 4]]));
        }
    };

    let rows = parse_hawking_spectrum_csv(&content);
    if rows.is_empty() {
        warn!("Hawking spectrum LUT has no data rows; creating fallback");
        return images.add(create_rgba32_texture(&[[0.0; 4]]));
    }

    let pixel_data: Vec<[f32; 4]> = rows.iter().map(|r| [r.red, r.green, r.blue, 1.0]).collect();

    let image = create_rgba32_texture(&pixel_data);
    info!(
        "Loaded hawking_spectrum_lut.csv: {} entries",
        pixel_data.len()
    );
    images.add(image)
}

/// Load the spin radii LUT and return a texture handle.
fn load_spin_radii_lut(dir: &str, images: &mut Assets<Image>) -> Handle<Image> {
    let path = format!("{dir}/spin_radii_lut.csv");
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to read spin radii LUT: {e}; creating fallback");
            return images.add(create_rgba32_texture(&[[0.0; 4]]));
        }
    };

    let rows = parse_spin_radii_csv(&content);
    if rows.is_empty() {
        warn!("Spin radii LUT has no data rows; creating fallback");
        return images.add(create_rgba32_texture(&[[0.0; 4]]));
    }

    let pixel_data: Vec<[f32; 4]> = rows
        .iter()
        .map(|r| [r.spin, r.r_isco_over_rs, r.r_ph_over_rs, 0.0])
        .collect();

    let image = create_rgba32_texture(&pixel_data);
    info!("Loaded spin_radii_lut.csv: {} entries", pixel_data.len());
    images.add(image)
}

/// Parse lut_meta.json for metadata about the LUT generation parameters.
fn load_lut_meta(dir: &str) -> LutMeta {
    let path = format!("{dir}/lut_meta.json");
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to read LUT metadata: {e}; using defaults");
            return LutMeta::default();
        }
    };

    // Minimal JSON parsing without serde_json dependency.
    // Extract key fields from the known lut_meta.json structure.
    let mut meta = LutMeta::default();

    if let Some(val) = extract_json_string(&content, "emissivity_model") {
        meta.emissivity_model = val;
    }
    if let Some(val) = extract_json_f64(&content, "spin") {
        meta.spin = val;
    }
    if let Some(val) = extract_json_f64(&content, "r_in_over_rs") {
        meta.r_in_over_rs = val;
    }
    if let Some(val) = extract_json_f64(&content, "r_out_over_rs") {
        meta.r_out_over_rs = val;
    }
    if let Some(val) = extract_json_f64(&content, "size") {
        meta.size = val as u32;
    }

    info!(
        "LUT meta: model={}, spin={}, size={}",
        meta.emissivity_model, meta.spin, meta.size
    );
    meta
}

/// Extract a string value from flat JSON by key name.
fn extract_json_string(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{key}\"");
    let pos = json.find(&pattern)?;
    let after_key = &json[pos + pattern.len()..];
    // Skip whitespace and colon
    let after_colon = after_key.find(':').map(|i| &after_key[i + 1..])?;
    let trimmed = after_colon.trim_start();
    if trimmed.starts_with('"') {
        let start = 1;
        let end = trimmed[start..].find('"')?;
        Some(trimmed[start..start + end].to_string())
    } else {
        None
    }
}

/// Extract a numeric value from flat JSON by key name.
fn extract_json_f64(json: &str, key: &str) -> Option<f64> {
    let pattern = format!("\"{key}\"");
    let pos = json.find(&pattern)?;
    let after_key = &json[pos + pattern.len()..];
    let after_colon = after_key.find(':').map(|i| &after_key[i + 1..])?;
    let trimmed = after_colon.trim_start();
    // Read until comma, newline, or closing brace
    let end = trimmed.find([',', '\n', '}']).unwrap_or(trimmed.len());
    trimmed[..end].trim().parse::<f64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_single_value_csv_basic() {
        let csv = "u,value\n0.0,0.5\n0.5,0.75\n1.0,1.0\n";
        let rows = parse_single_value_csv(csv);
        assert_eq!(rows.len(), 3);
        assert!((rows[0].value - 0.5).abs() < 1e-6);
        assert!((rows[2].value - 1.0).abs() < 1e-6);
    }

    #[test]
    fn parse_single_value_csv_empty() {
        let csv = "u,value\n";
        let rows = parse_single_value_csv(csv);
        assert!(rows.is_empty());
    }

    #[test]
    fn parse_hawking_temp_csv_with_comments() {
        let csv = "# Comment line\n\
                   \"# Another comment\"\n\
                   Mass_g,Temperature_K,Radius_cm\n\
                   1.0e14,1.2e12,1.5e-14\n\
                   2.0e14,6.0e11,3.0e-14\n";
        let rows = parse_hawking_temp_csv(csv);
        assert_eq!(rows.len(), 2);
        assert!((rows[0].temperature_k - 1.2e12).abs() < 1e6);
    }

    #[test]
    fn parse_hawking_spectrum_csv_with_comments() {
        let csv = "# Hawking Spectrum LUT\n\
                   \"# Columns: ...\"\n\
                   Temperature_K,Red,Green,Blue\n\
                   1000.0,1.0,0.01,0.0001\n";
        let rows = parse_hawking_spectrum_csv(csv);
        assert_eq!(rows.len(), 1);
        assert!((rows[0].red - 1.0).abs() < 1e-6);
        assert!((rows[0].green - 0.01).abs() < 1e-6);
    }

    #[test]
    fn parse_spin_radii_csv_basic() {
        let csv = "spin,r_isco_over_rs,r_ph_over_rs\n\
                   -0.99,4.486,0.584\n\
                   0.0,3.0,1.5\n\
                   0.99,0.618,0.5\n";
        let rows = parse_spin_radii_csv(csv);
        assert_eq!(rows.len(), 3);
        assert!((rows[1].r_isco_over_rs - 3.0).abs() < 1e-6);
    }

    #[test]
    fn create_r32_texture_dimensions() {
        let values = vec![0.0, 0.5, 1.0];
        let img = create_r32_texture(&values);
        assert_eq!(img.width(), 3);
        assert_eq!(img.height(), 1);
        // R32Float = 4 bytes per pixel
        assert_eq!(img.data.as_ref().unwrap().len(), 12);
    }

    #[test]
    fn create_rgba32_texture_dimensions() {
        let data = vec![[1.0, 0.5, 0.25, 1.0], [0.0, 0.0, 0.0, 0.0]];
        let img = create_rgba32_texture(&data);
        assert_eq!(img.width(), 2);
        assert_eq!(img.height(), 1);
        // RGBA32Float = 16 bytes per pixel
        assert_eq!(img.data.as_ref().unwrap().len(), 32);
    }

    #[test]
    fn extract_json_string_basic() {
        let json = r#"{ "model": "novikov-thorne", "spin": 0.0 }"#;
        assert_eq!(
            extract_json_string(json, "model"),
            Some("novikov-thorne".into())
        );
    }

    #[test]
    fn extract_json_f64_basic() {
        let json = r#"{ "spin": 0.5, "size": 256 }"#;
        assert!((extract_json_f64(json, "spin").unwrap() - 0.5).abs() < 1e-10);
        assert!((extract_json_f64(json, "size").unwrap() - 256.0).abs() < 1e-10);
    }

    #[test]
    fn extract_json_f64_scientific() {
        let json = r#"{ "mass": 1.988470e+33 }"#;
        let val = extract_json_f64(json, "mass").unwrap();
        assert!((val - 1.988470e+33).abs() / val < 1e-6);
    }
}
