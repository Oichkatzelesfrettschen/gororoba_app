#![forbid(unsafe_code)]
#![deny(warnings)]

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Parser, Debug)]
#[command(
    name = "xtask",
    about = "Workspace automation for packaging and contracts"
)]
struct Args {
    #[command(subcommand)]
    command: XtaskCommand,
}

#[derive(Subcommand, Debug)]
enum XtaskCommand {
    DesktopPackage {
        #[arg(
            long,
            default_value = "x86_64-unknown-linux-gnu,x86_64-unknown-freebsd,x86_64-pc-windows-msvc,x86_64-apple-darwin,aarch64-apple-darwin"
        )]
        targets: String,
        #[arg(
            long,
            default_value = "gororoba_studio_web,physics_sandbox,synthesis_arena"
        )]
        bins: String,
        #[arg(long, default_value = "release")]
        profile: String,
        #[arg(long, default_value = "dist/desktop")]
        out_dir: String,
        #[arg(long, default_value_t = true)]
        allow_missing: bool,
        #[arg(long, default_value_t = false)]
        skip_build: bool,
    },
    DesktopManifest {
        #[arg(long, default_value = "dist/desktop")]
        dist_dir: String,
    },
    MobileContract {
        #[arg(
            long,
            default_value = "apps/mobile_spike/contracts/shared_core_contract.json"
        )]
        output: String,
    },
    VerifyDeps {
        #[arg(long, default_value = "Cargo.toml")]
        manifest: String,
    },
}

#[derive(Debug, Serialize)]
struct ArtifactEntry {
    relative_path: String,
    size_bytes: u64,
    sha256: String,
}

#[derive(Debug, Serialize)]
struct DesktopManifest {
    generated_by: String,
    artifact_count: usize,
    artifacts: Vec<ArtifactEntry>,
}

#[derive(Debug, Serialize)]
struct MobileContract {
    version: String,
    learning_modes: Vec<String>,
    studio_api_version: String,
    pipelines: Vec<String>,
    sandbox_endpoints: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CratesApiResponse {
    #[serde(rename = "crate")]
    crate_meta: CrateMeta,
    versions: Vec<CrateVersion>,
}

#[derive(Debug, Deserialize)]
struct CrateMeta {
    newest_version: String,
}

#[derive(Debug, Deserialize)]
struct CrateVersion {
    num: String,
    yanked: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    match args.command {
        XtaskCommand::DesktopPackage {
            targets,
            bins,
            profile,
            out_dir,
            allow_missing,
            skip_build,
        } => desktop_package(
            parse_csv(&targets),
            parse_csv(&bins),
            &profile,
            Path::new(&out_dir),
            allow_missing,
            skip_build,
        ),
        XtaskCommand::DesktopManifest { dist_dir } => {
            write_manifest(Path::new(&dist_dir))?;
            Ok(())
        }
        XtaskCommand::MobileContract { output } => {
            write_mobile_contract(Path::new(&output))?;
            Ok(())
        }
        XtaskCommand::VerifyDeps { manifest } => {
            verify_deps(Path::new(&manifest))?;
            Ok(())
        }
    }
}

fn parse_csv(input: &str) -> Vec<String> {
    input
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn desktop_package(
    targets: Vec<String>,
    bins: Vec<String>,
    profile: &str,
    out_dir: &Path,
    allow_missing: bool,
    skip_build: bool,
) -> Result<()> {
    if targets.is_empty() || bins.is_empty() {
        bail!("targets and bins must both be non-empty");
    }

    fs::create_dir_all(out_dir)
        .with_context(|| format!("failed creating output directory {}", out_dir.display()))?;

    let mut copied = 0usize;

    for target in &targets {
        for bin in &bins {
            if !skip_build {
                let status = Command::new("cargo")
                    .arg("build")
                    .arg("--profile")
                    .arg(profile)
                    .arg("-p")
                    .arg(bin)
                    .arg("--target")
                    .arg(target)
                    .status()
                    .with_context(|| {
                        format!(
                            "failed to launch cargo build for target={} bin={}",
                            target, bin
                        )
                    })?;
                if !status.success() {
                    if allow_missing {
                        eprintln!(
                            "warning: build failed for target={} bin={}; continuing due to --allow-missing",
                            target, bin
                        );
                        continue;
                    }
                    bail!("build failed for target={} bin={}", target, bin);
                }
            }

            let source = build_binary_path(target, profile, bin);
            if !source.exists() {
                if allow_missing {
                    eprintln!(
                        "warning: binary not found at {}; continuing due to --allow-missing",
                        source.display()
                    );
                    continue;
                }
                bail!("binary not found after build: {}", source.display());
            }

            let target_dir = out_dir.join(target);
            fs::create_dir_all(&target_dir)
                .with_context(|| format!("failed creating {}", target_dir.display()))?;
            let destination = target_dir.join(binary_name(bin, target));
            fs::copy(&source, &destination).with_context(|| {
                format!(
                    "failed copying binary from {} to {}",
                    source.display(),
                    destination.display()
                )
            })?;
            copied += 1;
        }
    }

    write_manifest(out_dir)?;
    println!("desktop packaging finished: {} artifacts copied", copied);
    Ok(())
}

fn build_binary_path(target: &str, profile: &str, bin: &str) -> PathBuf {
    PathBuf::from("target")
        .join(target)
        .join(profile)
        .join(binary_name(bin, target))
}

fn binary_name(bin: &str, target: &str) -> String {
    if target.contains("windows") {
        format!("{bin}.exe")
    } else {
        bin.to_string()
    }
}

fn write_manifest(dist_dir: &Path) -> Result<()> {
    let mut files = Vec::new();
    collect_files(dist_dir, dist_dir, &mut files)?;
    files.sort();

    let mut artifacts = Vec::new();
    for path in files {
        if path.ends_with("manifest.json") || path.ends_with("checksums.txt") {
            continue;
        }
        let absolute = dist_dir.join(&path);
        let metadata = fs::metadata(&absolute)
            .with_context(|| format!("failed reading metadata for {}", absolute.display()))?;
        let digest = sha256_file(&absolute)?;
        artifacts.push(ArtifactEntry {
            relative_path: path,
            size_bytes: metadata.len(),
            sha256: digest,
        });
    }

    let manifest = DesktopManifest {
        generated_by: "xtask.desktop-manifest.v1".to_string(),
        artifact_count: artifacts.len(),
        artifacts,
    };

    let json = serde_json::to_string_pretty(&manifest)?;
    let manifest_path = dist_dir.join("manifest.json");
    fs::write(&manifest_path, json)
        .with_context(|| format!("failed writing {}", manifest_path.display()))?;

    let mut checksums = String::new();
    for artifact in &manifest.artifacts {
        checksums.push_str(&format!(
            "{}  {}\n",
            artifact.sha256, artifact.relative_path
        ));
    }
    fs::write(dist_dir.join("checksums.txt"), checksums).context("failed writing checksums.txt")?;

    println!(
        "manifest generated at {} with {} artifacts",
        manifest_path.display(),
        manifest.artifact_count
    );
    Ok(())
}

fn collect_files(root: &Path, current: &Path, out: &mut Vec<String>) -> Result<()> {
    if !current.exists() {
        return Ok(());
    }

    for entry in
        fs::read_dir(current).with_context(|| format!("failed reading {}", current.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(root, &path, out)?;
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .with_context(|| format!("failed computing relative path for {}", path.display()))?
            .to_string_lossy()
            .to_string();
        out.push(relative);
    }
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)
        .with_context(|| format!("failed opening for hash: {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    let digest = hasher.finalize();
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    Ok(out)
}

fn verify_deps(manifest: &Path) -> Result<()> {
    let content = fs::read_to_string(manifest)
        .with_context(|| format!("failed reading manifest {}", manifest.display()))?;
    let mut deps = parse_workspace_dependency_versions(&content)?;
    deps.sort();

    if deps.is_empty() {
        bail!(
            "no workspace dependency versions found under [workspace.dependencies] in {}",
            manifest.display()
        );
    }

    let client = reqwest::blocking::Client::builder()
        .user_agent("gororoba_xtask/verify_deps")
        .build()
        .context("failed building reqwest client")?;

    let mut mismatches = Vec::new();
    println!("verifying {} dependencies from crates.io", deps.len());
    for (name, pinned) in &deps {
        let url = format!("https://crates.io/api/v1/crates/{name}");
        let response = client
            .get(&url)
            .send()
            .with_context(|| format!("request failed for crate {name}"))?
            .error_for_status()
            .with_context(|| format!("crates.io returned error status for crate {name}"))?
            .json::<CratesApiResponse>()
            .with_context(|| format!("failed parsing crates.io response for crate {name}"))?;

        let latest_stable = response
            .versions
            .iter()
            .find(|version| !version.yanked && !is_prerelease(&version.num))
            .map(|version| version.num.clone())
            .unwrap_or_else(|| response.crate_meta.newest_version.clone());

        if pinned == &latest_stable {
            println!(
                "ok  {:24} pinned={} latest_stable={}",
                name, pinned, latest_stable
            );
        } else {
            println!(
                "out {:24} pinned={} latest_stable={}",
                name, pinned, latest_stable
            );
            mismatches.push(format!(
                "crate {name} pinned at {pinned} but latest stable is {latest_stable}"
            ));
        }
    }

    if mismatches.is_empty() {
        println!("dependency verification complete: all pinned versions are latest stable");
        return Ok(());
    }

    bail!(
        "dependency version mismatch detected:\n{}",
        mismatches.join("\n")
    )
}

fn parse_workspace_dependency_versions(content: &str) -> Result<Vec<(String, String)>> {
    let mut in_section = false;
    let mut out = Vec::new();

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.starts_with('[') {
            in_section = line == "[workspace.dependencies]";
            continue;
        }
        if !in_section || line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((raw_name, raw_value)) = line.split_once('=') else {
            continue;
        };
        let name = raw_name.trim();
        let value = raw_value.trim();

        if value.starts_with('"') {
            if let Some(parsed) = parse_quoted(value) {
                out.push((name.to_string(), parsed));
            }
            continue;
        }

        if value.starts_with('{')
            && let Some(parsed) = parse_inline_version_field(value)
        {
            out.push((name.to_string(), parsed));
        }
    }

    Ok(out)
}

fn parse_inline_version_field(value: &str) -> Option<String> {
    let marker = "version";
    let marker_pos = value.find(marker)?;
    let after_marker = &value[marker_pos + marker.len()..];
    let eq_pos = after_marker.find('=')?;
    let after_eq = after_marker[eq_pos + 1..].trim_start();
    parse_quoted(after_eq)
}

fn parse_quoted(value: &str) -> Option<String> {
    let first = value.find('"')?;
    let remaining = &value[first + 1..];
    let second = remaining.find('"')?;
    Some(remaining[..second].to_string())
}

fn is_prerelease(version: &str) -> bool {
    let without_build = version.split('+').next().unwrap_or(version);
    without_build.contains('-')
}

fn write_mobile_contract(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed creating {}", parent.display()))?;
    }
    let contract = MobileContract {
        version: "mobile_spike.v1".to_string(),
        learning_modes: vec![
            "story".to_string(),
            "explorer".to_string(),
            "research".to_string(),
        ],
        studio_api_version: "studio.v1".to_string(),
        pipelines: vec![
            "thesis-1".to_string(),
            "thesis-2".to_string(),
            "thesis-3".to_string(),
            "thesis-4".to_string(),
        ],
        sandbox_endpoints: vec!["/api/simulate".to_string(), "/api/benchmark".to_string()],
    };

    let json = serde_json::to_string_pretty(&contract)?;
    fs::write(path, json).with_context(|| format!("failed writing {}", path.display()))?;
    println!("wrote mobile contract {}", path.display());
    Ok(())
}
