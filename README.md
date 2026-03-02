# gororoba_app

Rust SSR frontend workspace for the Gororoba experiment ecosystem.

## Workspace layout

- `Cargo.toml`: root workspace manifest and dependency policy.
- `apps/studio_web`: full-featured SSR app (Axum + Askama + typed backend client).
- `apps/physics_sandbox`: visual simulation + benchmark app prototype.
- `apps/synthesis_arena`: game-like optimization lab with benchmarkable scoring.
- `crates/gororoba_shared_core`: Rust-first shared domain and learning core.
- `tools/xtask`: reproducible packaging and contract automation lane.
- `apps/mobile_spike`: Android/iOS architecture spike docs and contracts.
- `docs/CROSS_REPO_INTERFACE_POLICY.md`: frontend/backend boundary rules.
- `docs/OPEN_GOROROBA_BACKEND_ALIGNMENT_HANDOFF.md`: cross-repo API/artifact sync checklist.
- `docs/DEPENDENCY_REFERENCE.md`: verified crate versions and API/manual links.
- `docs/DESKTOP_PACKAGING_LANE.md`: packaging lane documentation.
- `docs/APP_PURPOSE_AND_AUDIENCES.md`: mission, personas, and depth model.
- `REQUIREMENTS.md`: reproducible installation/build/test requirements.
- `plans/CROSS_REPO_EXECUTION_STRATEGY_2026_02_13.toml`: cross-repo coordination strategy.

## Studio app

`gororoba_studio_web` provides:

1. Dashboard view of available thesis pipelines.
2. Run, benchmark, and reproducibility actions for each pipeline.
3. API-version-aware rendering against backend contract `studio.v1`.
4. Asset-backed responsive UI (desktop + mobile) with Rust SSR templates.
5. Offline integration tests using a local mock backend router.
6. In-memory dashboard cache with mutation invalidation and configurable TTL.
7. Three layered pedagogy modes:
   - Story (7th grade)
   - Explorer (curious adult)
   - Research (equations, walkthroughs, assumptions, proof sketch, artifact links)
8. Visual-lab canvas overlays for history metrics, oscillator intuition, and phase-space behavior.

## Physics sandbox app

`physics_sandbox` provides:

1. Deterministic oscillator simulation API (`POST /api/simulate`).
2. Benchmark API (`POST /api/benchmark`) with stability and timing metrics.
3. Interactive visual page for controls + charting.
4. Story/Explorer/Research learning layers in one UI.
5. Time-series and phase-space visual plots for analysis.
6. Offline deterministic tests for simulation and API flows.

Default URL after launch: `http://127.0.0.1:8093`.

## Synthesis arena app

`synthesis_arena` provides:

1. Challenge catalog API (`GET /api/challenges`).
2. Deterministic scoring API (`POST /api/evaluate`) with 4 metric gates.
3. Benchmark API (`POST /api/benchmark`) with timing and score statistics.
4. Story/Explorer/Research learning layers in the UI.
5. Visual scoreboard chart for metric values versus gate thresholds.

Default URL after launch: `http://127.0.0.1:8094`.

## Quick start

```bash
cargo run -p gororoba_studio_web
```

Then open `http://127.0.0.1:8090`.

If your backend runs elsewhere:

```bash
GOROROBA_BACKEND_URL=http://127.0.0.1:8088 cargo run -p gororoba_studio_web
```

Adjust frontend cache TTL (milliseconds):

```bash
GOROROBA_UI_CACHE_TTL_MS=1500 cargo run -p gororoba_studio_web
```

Run physics sandbox:

```bash
cargo run -p physics_sandbox
```

Run synthesis arena:

```bash
cargo run -p synthesis_arena
```

Generate mobile contract artifact:

```bash
scripts/generate_mobile_contract.sh
```

Build desktop packaging matrix:

```bash
scripts/package_desktop_matrix.sh
```

## Quality gates

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```
