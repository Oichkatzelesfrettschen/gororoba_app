# Bevy-Heavy Test Deep Dive

## Why the Bevy lane feels slow

The slow path in this workspace is not the pure math itself. It is the
combination of:

1. Bevy's large dependency graph and default feature surface
2. `--all-targets` expanding compile scope
3. test-profile linking for game crates
4. cold-cache rebuilds after lockfile or cache isolation changes
5. math/runtime logic living inside Bevy crates instead of pure kernel crates

The algebra migration already proved the fastest pattern:

1. put real math in a pure local kernel crate
2. test that crate directly
3. keep the Bevy crate thin
4. run narrower Bevy checks for adapter validation

## Optimizations already present

1. repo-local Cargo isolation in `scripts/cargo-isolated.sh`
2. default half-CPU worker policy for build jobs, test threads, and Rayon
3. `target-cpu=native` on x86_64
4. local SoA LBM solver now extracted into `crates/gororoba_kernel_fluid`
   with Rayon and autovectorization-friendly layout
5. local algebra zero-divisor search using Rayon
6. `mold`-accelerated linking on Linux x86_64 when available
7. a narrower default workspace member set so plain `cargo check` stops pulling
   every Bevy game by default
8. wrapper crates no longer pull `gororoba_bevy_core` unnecessarily, which
   trims egui/UI compile drag from algebra/GR/quantum/fluid adapter crates
9. dev-profile optimization is now concentrated on numeric kernels and fluid
   backends instead of globally forcing all dependencies to `opt-level = 3`

## Missing optimizations still worth adding

1. physical-core Rayon pool initialization for heavy math tests
   Reference: `open_gororoba/crates/algebra_analysis/src/test_support.rs`
2. backend capability detection and explicit CPU SIMD / Vulkan / CUDA reporting
   Reference: `open_gororoba/crates/gororoba_gpu_bridge/src/lib.rs`
3. backend capability detection and explicit CPU SIMD / Vulkan / CUDA reporting
   in the new local fluid lane as well as the remaining domains
4. optional `sccache` rollout once the host incremental interaction is fully
   standardized

## Most leverage for this repository

1. move optimized math out of Bevy crates first
2. validate pure kernels with locked test runs
3. keep Bevy verification mostly to `check`/`clippy` unless adapter behavior
   truly needs runtime execution
4. only use full Bevy-heavy test runs for changed adapter paths
5. keep moving validation from Bevy adapter crates into pure kernel crates;
   fluid now has that seam, but the upstream AoS fallback and GPU bridge still
   keep `gororoba_bevy_lbm` heavier than the algebra/GR/quantum adapters
