# open_gororoba Backend Alignment Handoff

Verified on: 2026-02-13

## Purpose

This handoff defines the integration contract between:

1. Frontend workspace: `gororoba_app`
2. Backend source of truth: `open_gororoba`

The goal is deterministic, versioned, Rust-first interoperability for Studio
APIs, pipeline metadata, and artifact linkage.

## System boundary

1. `gororoba_app` is frontend-only and consumes backend data over HTTP.
2. `open_gororoba` owns pipeline execution and canonical experiment registry.
3. Frontend must not duplicate backend business logic that defines pass/fail for
   thesis pipelines.
4. Shared assumptions:
   - API version: `studio.v1`
   - Transport: JSON over HTTP
   - Pipeline IDs: `thesis-1` .. `thesis-4`

## Source-of-truth files (backend)

1. API implementation:
   - `/home/eirikr/Github/open_gororoba/crates/gororoba_cli/src/bin/gororoba_studio.rs`
2. Backend studio requirement doc:
   - `/home/eirikr/Github/open_gororoba/apps/gororoba_studio/README.md`
3. Machine-readable contract inventory:
   - `/home/eirikr/Github/open_gororoba/reports/studio_backend_contract_inventory_2026_02_13.toml`
4. Registry catalog:
   - `/home/eirikr/Github/open_gororoba/registry/experiments.toml`

## Source-of-truth files (frontend)

1. Backend client:
   - `apps/studio_web/src/client.rs`
2. Contract mirror models:
   - `apps/studio_web/src/models.rs`
3. Interface policy:
   - `docs/CROSS_REPO_INTERFACE_POLICY.md`

## Endpoint contract matrix

All endpoints are served by `gororoba-studio` in `open_gororoba`.

1. `GET /api/health`
   - Response: health envelope with `api_version`, `service`, `status`, `unix_seconds`.
2. `GET /api/version`
   - Response: `VersionResponse`
   - Required fields: `api_version`, `service`, `package_version`, `catalog_source`, `pipeline_count`, `catalog_warnings`, `registry_path`.
3. `GET /api/pipelines`
   - Response: `Vec<PipelineDescriptor>`
   - Required fields: `id`, `title`, `hypothesis`, `primary_metric`, `quick_profile`, `full_profile`, `experiment_id`, `lineage_id`, `registry_binary`, `artifact_paths`.
4. `GET /api/history`
   - Response: `Vec<RunResponse>`
   - Required fields: `run_id`, `unix_seconds`, `experiment_id`, `artifact_links`, `profile`, `duration_ms`, `metric_value`, `threshold`, `passes_gate`, `config_snapshot`.
5. `POST /api/run/{experiment_id}`
   - Request body: `{ "profile": "quick" | "full" }`
   - Success: `RunResponse`
   - Failure: `ApiErrorResponse`
6. `POST /api/run-suite`
   - Request body: `{ "profile": "quick" | "full" }`
   - Success: `SuiteResponse`
7. `POST /api/benchmark/{experiment_id}`
   - Request body: `{ "profile": "...", "iterations": <optional usize> }`
   - Success: `BenchmarkResponse`
   - Failure: `ApiErrorResponse`
8. `POST /api/reproducibility/{experiment_id}`
   - Request body: `{ "profile": "...", "iterations": <optional usize>, "tolerance": <optional f64> }`
   - Success: `ReproducibilityResponse`
   - Failure: `ApiErrorResponse`

## Compatibility requirements

1. API version lock:
   - Frontend must verify `VersionResponse.api_version == "studio.v1"`.
2. Unknown pipeline behavior:
   - Backend returns structured error; frontend renders BAD_GATEWAY/NOT_FOUND path.
3. Profile encoding:
   - Must remain lowercase enum values: `quick`, `full`.
4. Numeric compatibility:
   - `duration_ms` accepted as unsigned integer in backend and frontend models.
5. Error envelope stability:
   - Required keys: `api_version`, `error_code`, `message`, `known_ids`, `details`.

## Artifact mapping discipline

Frontend lesson and pipeline panels should resolve artifact references into
canonical backend paths.

1. Registry source:
   - `/home/eirikr/Github/open_gororoba/registry/experiments.toml`
2. Contract inventory map:
   - `/home/eirikr/Github/open_gororoba/reports/studio_backend_contract_inventory_2026_02_13.toml`
3. Frontend display should prefer backend-provided `artifact_paths` and
   `artifact_links` for run output provenance.

## Reproducible sync workflow

From backend repo (`open_gororoba`):

```bash
make studio-check
make studio-run
```

From frontend repo (`gororoba_app`):

```bash
make check
GOROROBA_BACKEND_URL=http://127.0.0.1:8088 cargo run -p gororoba_studio_web
```

Contract smoke checks:

```bash
curl -fsS http://127.0.0.1:8088/api/version
curl -fsS http://127.0.0.1:8088/api/pipelines
curl -fsS http://127.0.0.1:8088/api/history
```

## Acceptance checklist

1. Backend `studio-check` passes with `-D warnings`.
2. Frontend `make check` passes with `-D warnings`.
3. `api_version` is `studio.v1` on live backend.
4. Pipeline count in backend equals number of cards rendered in frontend home.
5. Run/benchmark/repro actions succeed for `thesis-1` in both quick and full profiles.
6. Frontend history table displays run IDs and gate outcomes from backend results.
7. No hardcoded fake backend data is used outside explicit test mocks.

## Testable hypotheses for ongoing alignment

1. Hypothesis A:
   - Claim: frontend models in `apps/studio_web/src/models.rs` remain backward-compatible with backend response fields.
   - Test: run frontend integration tests against backend mock and live backend payload snapshots.
2. Hypothesis B:
   - Claim: backend registry changes can be consumed without frontend code changes as long as endpoint schema is stable.
   - Test: add a new pipeline entry in backend registry and verify frontend list rendering + per-pipeline routing.
3. Hypothesis C:
   - Claim: artifact paths shown in frontend map directly to backend-produced files.
   - Test: for each run response, verify referenced path exists in `/home/eirikr/Github/open_gororoba`.

## Escalation triggers

Pause and reconcile before merge if any condition occurs:

1. `api_version` changes from `studio.v1`.
2. Any required response key is removed or renamed.
3. Backend error envelope shape changes.
4. Pipeline IDs are renamed without migration mapping.
5. Artifact path fields are removed or become non-path identifiers without resolver docs.
