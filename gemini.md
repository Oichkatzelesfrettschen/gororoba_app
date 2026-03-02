# gemini.md

## Repository sanity checklist

1. Verify backend endpoint availability via `GOROROBA_BACKEND_URL`.
2. Run workspace quality gates from repository root.
3. Confirm docs are synchronized:
   - `README.md`
   - `REQUIREMENTS.md`
   - `docs/DEPENDENCY_REFERENCE.md`
   - `docs/OPEN_GOROROBA_BACKEND_ALIGNMENT_HANDOFF.md`
   - `plans/ULTRA_ROADMAP_2026_02_13.toml`
4. Verify cache behavior by exercising two sequential `GET /` calls (cache hit) and one mutation route (cache invalidation).
5. Verify learning mode parity (`story`, `explorer`, `research`) across dashboard and pipeline routes.
6. Verify `physics_sandbox` endpoints (`/api/simulate`, `/api/benchmark`) are deterministic and healthy.
7. Verify `scripts/generate_mobile_contract.sh` regenerates `apps/mobile_spike/contracts/shared_core_contract.json`.
8. Verify desktop packaging lane creates `dist/desktop/manifest.json` and `dist/desktop/checksums.txt`.
9. Verify `synthesis_arena` endpoints (`/api/challenges`, `/api/evaluate`, `/api/benchmark`) are deterministic and healthy.
10. Keep frontend and backend contract boundaries aligned with `docs/CROSS_REPO_INTERFACE_POLICY.md` and the backend handoff doc.
