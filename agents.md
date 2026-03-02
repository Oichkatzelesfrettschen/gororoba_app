# agents.md

## Frontend orchestration policy

1. Keep frontend implementation in `gororoba_app`.
2. Treat `open_gororoba` as backend/data source of truth.
3. Enforce `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace --all-targets`.
4. Keep all dependency updates documented in `docs/DEPENDENCY_REFERENCE.md`.
5. Keep roadmap status synced in `plans/ULTRA_ROADMAP_2026_02_13.toml`.
6. Keep route-flow E2E tests offline by using local mock backend routers.
7. Preserve cache correctness: shell-data cache may be reused for read views but must be invalidated after mutation routes.
8. Keep Story/Explorer/Research learning layers synchronized between `gororoba_shared_core` and `apps/studio_web`.
9. Route desktop packaging and mobile contract generation through `tools/xtask` and scripts in `scripts/`.
10. Keep `apps/physics_sandbox` deterministic and benchmark-friendly with stable JSON outputs.
11. Keep `apps/synthesis_arena` deterministic with challenge/evaluate/benchmark APIs and visual scoreboard parity.
12. Keep backend alignment references current in `docs/OPEN_GOROROBA_BACKEND_ALIGNMENT_HANDOFF.md`.
