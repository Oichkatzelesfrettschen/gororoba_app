# Cross-Repo Interface Policy

Date: 2026-02-13

This project uses a strict two-repo boundary:

1. `open_gororoba` is the source of truth for:
- Rust backend services and API contracts.
- Registry and experiment metadata.
- Artifact production and evidence data.

2. `gororoba_app` is the source of truth for:
- Front-end application code.
- UI/UX state logic and view composition.
- Front-end tests and app packaging strategy.

## Integration contract

1. `gororoba_app` must treat `open_gororoba` as read-only data/API input.
2. Contracted API is versioned by `GET /api/version` with `api_version = "studio.v1"`.
3. Detailed synchronization checklist is maintained in:
- `docs/OPEN_GOROROBA_BACKEND_ALIGNMENT_HANDOFF.md`
4. Required backend endpoints:
- `GET /api/health`
- `GET /api/version`
- `GET /api/pipelines`
- `GET /api/history`
- `POST /api/run/{experiment_id}`
- `POST /api/run-suite`
- `POST /api/benchmark/{experiment_id}`
- `POST /api/reproducibility/{experiment_id}`

## Path assumptions

1. Backend registry path defaults to:
- `open_gororoba/registry/experiments.toml`
2. Override path is supported with:
- `GOROROBA_EXPERIMENTS_REGISTRY`
3. Artifact links returned by backend are relative repo paths into `open_gororoba`.

## Dependency policy

1. No new system dependencies are permitted for frontend execution.
2. Frontend stack must be Rust-first and cargo-runnable.
3. Tests must be offline and deterministic.
