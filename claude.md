# claude.md

## Development handoff notes

- First app implemented: `apps/studio_web` (`gororoba_studio_web`).
- Architecture: Axum router + Askama templates + typed Reqwest backend client.
- Added UX-flow E2E route tests (offline) and cache behavior tests.
- Added shared Rust learning core (`crates/gororoba_shared_core`) with Story/Explorer/Research lesson layers.
- Added desktop packaging automation (`tools/xtask`, `scripts/package_desktop_matrix.sh`).
- Added mobile architecture spike docs/contracts under `apps/mobile_spike`.
- Added physics sandbox prototype app (`apps/physics_sandbox`) with simulation and benchmark APIs.
- Added synthesis arena prototype app (`apps/synthesis_arena`) with deterministic game-like scoring and benchmark APIs.
- Backend contract target: `studio.v1` from `open_gororoba`.
- Backend handoff doc: `docs/OPEN_GOROROBA_BACKEND_ALIGNMENT_HANDOFF.md`.
- Use only workspace-managed dependencies from root `Cargo.toml`.
- Treat all warnings as errors before merge.
