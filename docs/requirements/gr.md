# Relativity Lane Requirements

## Current role

The relativity lane now has a local engine boundary in:

- `crates/gororoba_kernel_api`
- `crates/gororoba_kernel_gr`
- `crates/gororoba_bevy_gr`

The current CPU implementation still reuses upstream `gr_core` math
internally, but the game layer now consumes only the local kernel-facing
surface:

- black hole metric selection
- shadow boundary generation
- geodesic stepping
- event horizon / ISCO / time dilation helpers

## Smallest stable local-kernel boundary

The minimum engine-owned Rust boundary is now captured by
`gororoba_kernel_api::relativity`:

- `MetricFamily`
- `RelativityDomainConfig`
- `GeodesicKind`
- `GeodesicSnapshot`
- `ShadowSnapshot`
- `RelativityDiagnosticsSnapshot`
- `RelativityKernel`

This is the smallest useful cut because the game only needs:

1. one spacetime configuration per domain
2. a shadow curve for rendering
3. a mutable set of geodesic states for stepping
4. scalar helpers for time dilation, event horizon, and ISCO

## Remaining blockers

1. `gororoba_kernel_gr` still uses upstream `gr_core` internally rather than a
   fully local solver implementation.
2. Shadow and diagnostics are still cached in Bevy-facing resource state for
   rendering convenience.
3. The game still mixes shader/LUT concerns with metric concerns, so the local
   kernel split must stay math-only and leave rendering assets in the game.

## Highest-value next extraction

1. Keep expanding tests in `gororoba_kernel_gr` until shadow generation and
   geodesic stepping can be validated without Bevy.
2. Keep the ray-traced material and LUT assets in `games/relativistic_space`.
3. Replace remaining upstream-internal calls inside `gororoba_kernel_gr` with
   local implementations once parity coverage is strong enough.

## Verification commands

```bash
scripts/cargo-isolated.sh check -p gororoba_kernel_api -p gororoba_bevy_gr -p relativistic_space --all-targets --locked
scripts/cargo-isolated.sh clippy -p gororoba_kernel_api -p gororoba_bevy_gr -p relativistic_space --all-targets -- -D warnings
scripts/cargo-isolated.sh test -p gororoba_kernel_gr -p gororoba_bevy_gr --lib
```
