# Dependency Reference (Rust Workspace)

Verified on: 2026-02-13

This document records crate versions selected for the `gororoba_app` workspace,
plus direct links to primary package records and API manuals.

## Verification method

1. Run `cargo run -p xtask -- verify-deps` from repo root.
2. `xtask verify-deps` queries crates.io API endpoint `https://crates.io/api/v1/crates/<crate>`.
3. For each crate, compare pinned version against latest stable non-prerelease version.
4. Verify docs/manual URLs with `curl -L -o /dev/null -w "%{http_code}"`.
5. Pin exact versions in root `Cargo.toml` under `[workspace.dependencies]`.

## Core dependencies

| Crate | Version | crates.io | API docs/manual |
|---|---:|---|---|
| `axum` | `0.8.8` | <https://crates.io/api/v1/crates/axum> | <https://docs.rs/axum/0.8.8/axum/> |
| `tokio` | `1.49.0` | <https://crates.io/api/v1/crates/tokio> | <https://docs.rs/tokio/1.49.0/tokio/> |
| `reqwest` | `0.13.2` | <https://crates.io/api/v1/crates/reqwest> | <https://docs.rs/reqwest/0.13.2/reqwest/> |
| `serde` | `1.0.228` | <https://crates.io/api/v1/crates/serde> | <https://docs.rs/serde/1.0.228/serde/> |
| `serde_json` | `1.0.149` | <https://crates.io/api/v1/crates/serde_json> | <https://docs.rs/serde_json/1.0.149/serde_json/> |
| `urlencoding` | `2.1.3` | <https://crates.io/api/v1/crates/urlencoding> | <https://docs.rs/urlencoding/2.1.3/urlencoding/> |
| `askama` | `0.15.4` | <https://crates.io/api/v1/crates/askama> | <https://docs.rs/askama/0.15.4/askama/> |
| `clap` | `4.5.58` | <https://crates.io/api/v1/crates/clap> | <https://docs.rs/clap/4.5.58/clap/> |
| `anyhow` | `1.0.101` | <https://crates.io/api/v1/crates/anyhow> | <https://docs.rs/anyhow/1.0.101/anyhow/> |
| `sha2` | `0.10.9` | <https://crates.io/api/v1/crates/sha2> | <https://docs.rs/sha2/0.10.9/sha2/> |
| `tracing` | `0.1.44` | <https://crates.io/api/v1/crates/tracing> | <https://docs.rs/tracing/0.1.44/tracing/> |
| `tracing-subscriber` | `0.3.22` | <https://crates.io/api/v1/crates/tracing-subscriber> | <https://docs.rs/tracing-subscriber/0.3.22/tracing_subscriber/> |
| `tower` | `0.5.3` | <https://crates.io/api/v1/crates/tower> | <https://docs.rs/tower/0.5.3/tower/> |

## Related framework manuals

- Askama manual: <https://askama.readthedocs.io/>
- Axum manual (latest channel, for migration notes): <https://docs.rs/axum/latest/axum/>

## Notes

- `sha2` currently exposes `0.11.0-rc.*` pre-release versions on crates.io; the
  workspace remains pinned to latest stable `0.10.9` until non-rc 0.11 is released.
