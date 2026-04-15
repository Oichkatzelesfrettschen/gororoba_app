# gororoba_app

Rust workspace for the Gororoba engine microcosm.

`gororoba_app` is no longer just an SSR frontend shell. It is a mixed workspace
with:

- self-contained local engine/kernel crates inside this repository
- Bevy game/plugin crates that turn those kernels into playable 3D systems
- operator and teaching apps that expose selected simulations and workflows

`open_gororoba` remains the reference physics/math kernel for research,
discovery, and parity checks. The direction for this repository is one-way:
study upstream, then implement the game-facing runtime locally here. Today,
that transition has started with the local algebra kernel used by
`non_euclidean`.

## Workspace layout

- `Cargo.toml`: root workspace manifest and dependency policy
- `crates/gororoba_kernel_api`: local engine-facing Rust traits and snapshot types
- `crates/gororoba_kernel_algebra`: local Cayley-Dickson algebra kernel
- `crates/gororoba_kernel_fluid`: local CPU fluid kernels and shared factory
- `crates/gororoba_kernel_fluid_vulkan`: local Vulkan capability/probe lane
- `crates/gororoba_kernel_fluid_cuda`: local CUDA capability/probe lane
- `crates/gororoba_bevy_core`: shared Bevy camera, HUD, pedagogy, and state
- `crates/gororoba_bevy_algebra`: Bevy algebra bridge using the local kernel
- `crates/gororoba_bevy_lbm`: fluid gameplay bridge with a local SoA kernel path
- `crates/gororoba_bevy_gr`: relativity gameplay bridge on local contracts
- `crates/gororoba_bevy_quantum`: quantum gameplay bridge on local contracts
- `games/fluid_dynamics`: Bevy fluid game
- `games/non_euclidean`: Bevy non-Euclidean algebra game
- `games/relativistic_space`: Bevy relativity game
- `games/quantum_builder`: Bevy quantum sandbox
- `games/interaction_arena`: strategy/game-semantics game
- `apps/studio_web`: SSR operator app
- `apps/physics_sandbox`: simulation app
- `apps/synthesis_arena`: scoring/teaching app
- `tools/xtask`: reproducible utility lane
- `docs/GAME_ARCHITECTURE.md`: current engine architecture
- `REQUIREMENTS.md`: root reproducible build requirements
- `docs/requirements/algebra.md`: module-specific requirements for the local algebra lane

## Current boundary

- `gororoba_app` owns gameplay runtime, Bevy integration, and local engine APIs.
- `open_gororoba` is used as a reference kernel, discovery backend, and future
  upstream target for novel findings.
- Runtime mirroring is being removed domain by domain, not all at once.
- Algebra is the first fully local gameplay kernel slice here.
- Fluid now has a fully local CPU runtime with scalar and SoA kernels plus
  local Vulkan/CUDA backend objects selected through a shared
  backend/capability contract.
- GR and quantum now route gameplay through local kernel APIs even though their
  current CPU implementations still reuse upstream internals.

## Quick start

Run the non-Euclidean game:

```bash
cargo run -p non_euclidean
```

Run the operator app:

```bash
cargo run -p gororoba_studio_web
```

Other entry points:

```bash
cargo run -p fluid_dynamics
cargo run -p relativistic_space
cargo run -p quantum_builder
cargo run -p interaction_arena
cargo run -p physics_sandbox
cargo run -p synthesis_arena
```

## Quality gates

```bash
scripts/cargo-isolated.sh fmt --all --check
scripts/cargo-isolated.sh clippy --workspace --all-targets -- -D warnings
scripts/cargo-isolated.sh test --workspace --all-targets --locked
scripts/cargo-isolated.sh check --workspace --all-targets --locked
```

## Current status

- Local kernel extraction now includes `gororoba_kernel_api`,
  `gororoba_kernel_algebra`, `gororoba_kernel_fluid`,
  `gororoba_kernel_fluid_vulkan`, and `gororoba_kernel_fluid_cuda`.
- `gororoba_bevy_algebra` and `games/non_euclidean` now use the local algebra
  kernel path.
- `gororoba_bevy_gr` and `gororoba_bevy_quantum` now consume local kernel
  contracts instead of upstream-facing game resources.
- Bevy is pinned to an explicit feature set so the workspace no longer pulls
  audio, glTF, or picking by default.
- CUDA and Vulkan fluid lanes are feature-gated so default builds stay on the
  fast CPU path.
- Cargo builds are isolated per project via repo-local `CARGO_HOME` and
  `target-gororoba_app` when using `scripts/cargo-isolated.sh`.
- The isolated Cargo wrapper defaults build/test/Rayon thread counts to half of
  detected CPUs unless you override them with environment variables.
