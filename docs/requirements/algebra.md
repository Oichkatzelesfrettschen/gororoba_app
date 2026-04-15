# Algebra Lane Requirements

## Scope

This module documents the local algebra kernel path used by:

- `crates/gororoba_kernel_api`
- `crates/gororoba_kernel_algebra`
- `crates/gororoba_bevy_algebra`
- `games/non_euclidean`

## Build requirements

1. Standard Rust toolchain compatible with edition `2024`.
2. No sibling `open_gororoba` checkout is required for this lane.
3. Bevy-capable graphics environment is required to run `non_euclidean`.

## Verification commands

```bash
scripts/cargo-isolated.sh check -p gororoba_kernel_api -p gororoba_kernel_algebra -p gororoba_bevy_algebra -p non_euclidean --all-targets --locked
scripts/cargo-isolated.sh test -p gororoba_kernel_api -p gororoba_kernel_algebra -p gororoba_bevy_algebra -p non_euclidean --all-targets --locked
scripts/cargo-isolated.sh clippy -p gororoba_kernel_api -p gororoba_kernel_algebra -p gororoba_bevy_algebra -p non_euclidean --all-targets -- -D warnings
```

## Functional expectations

1. `non_euclidean` must compute associator-driven distortion from real algebra elements.
2. Zero-divisor portals must be derived from the local kernel search results.
3. Portal placement must use deterministic projection from higher-dimensional
   signatures into 3D offsets.
4. Puzzle checks should prefer real algebraic predicates over hardcoded outcomes.

## Notes

1. This lane is the template for future local kernel extraction in the fluid,
   relativity, quantum, and strategy domains.
2. If a new algebra capability is discovered here and validated, it can be
   proposed upstream to `open_gororoba` later as a separate follow-on step.
