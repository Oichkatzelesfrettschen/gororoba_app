# Requirements

## Scope

This repository is a Rust workspace containing:

- self-contained engine/kernel crates
- Bevy game binaries
- operator/teaching apps

It may still use `open_gororoba` for selected upstream-backed domains during the
transition, but the target direction is local runtime ownership inside
`gororoba_app`.

## Toolchain requirements

1. Rust toolchain with Cargo compatible with edition `2024`.
2. A host graphics stack suitable for Bevy 0.18.
3. Network access for initial dependency resolution when `Cargo.lock` changes.
4. No Python or Node runtime is required for the Rust build/test lanes.
5. Optional upstream parity work may require a sibling checkout at
   `../open_gororoba`, but the local algebra lane builds without it.

## Build discipline

1. Treat warnings as errors in lint lanes.
2. Prefer locked commands after the lockfile is updated intentionally.
3. Keep module-specific requirements current under `docs/requirements/`.
4. When adding new workspace crates, refresh `Cargo.lock` and rerun locked gates.
5. Use repo-local Cargo isolation when multiple projects are active on the same
   machine.
6. The isolated Cargo wrapper defaults to half of detected CPUs for build jobs,
   test threads, and Rayon threads; override with `GOROROBA_CARGO_JOBS`,
   `CARGO_BUILD_JOBS`, `RUST_TEST_THREADS`, or `RAYON_NUM_THREADS` if needed.
7. On Linux x86_64, the isolated Cargo wrapper prefers `clang` plus `mold`
   automatically when they are installed to reduce Bevy-heavy link time.
8. `sccache` is supported as an opt-in accelerator via `GOROROBA_USE_SCCACHE=1`,
   but it is not enabled by default because some host environments still inject
   incompatible incremental settings.

## Primary commands

From repository root:

```bash
scripts/cargo-isolated.sh fmt --all --check
scripts/cargo-isolated.sh clippy --workspace --all-targets -- -D warnings
scripts/cargo-isolated.sh test --workspace --all-targets --locked
scripts/cargo-isolated.sh check --workspace --all-targets --locked
```

Fast local kernel loop:

```bash
scripts/cargo-isolated.sh check
scripts/cargo-isolated.sh clippy -p gororoba_kernel_api -p gororoba_kernel_algebra -p gororoba_kernel_gr -p gororoba_kernel_quantum --all-targets -- -D warnings
```

Run key binaries:

```bash
cargo run -p non_euclidean
cargo run -p fluid_dynamics
cargo run -p relativistic_space
cargo run -p quantum_builder
cargo run -p interaction_arena
cargo run -p gororoba_studio_web
cargo run -p physics_sandbox
cargo run -p synthesis_arena
```

## Module requirements

- Local algebra kernel lane: `docs/requirements/algebra.md`
- Local fluid migration lane: `docs/requirements/fluid.md`
- Local fluid Vulkan requirements: `docs/requirements/fluid_vulkan.md`
- Local fluid CUDA requirements: `docs/requirements/fluid_cuda.md`
- Local relativity migration lane: `docs/requirements/gr.md`
- Local quantum migration lane: `docs/requirements/quantum.md`
- Game architecture reference: `docs/GAME_ARCHITECTURE.md`
- Bevy-heavy performance analysis: `docs/BEVY_HEAVY_TESTS.md`

## Sanity checks

1. The root README, this file, and `docs/GAME_ARCHITECTURE.md` must agree on the
   repository role.
2. Runtime dependencies on `open_gororoba` should be called out explicitly by
   module until each lane is localized. The fluid CPU lane is now local; the
   local Vulkan/CUDA lanes currently provide probing plus backend surfaces, but
   not full local execution yet.
3. New local kernels must expose stable Rust APIs before game integration.
4. `scripts/cargo-isolated.sh` should be the default verification entry point
   when other repositories or agents are building at the same time.
5. The workspace Bevy dependency is intentionally feature-slimmed; new code
   should add Bevy features only when a concrete API requires them.
6. Compile-heavy Cargo commands should still be serialized per repository; do
   not run feature-heavy Bevy/CUDA/Vulkan lanes in parallel against the same
   `CARGO_HOME` and target directory.
