# Quantum Lane Requirements

## Current role

The quantum lane now has a local engine boundary in:

- `crates/gororoba_kernel_api`
- `crates/gororoba_kernel_quantum`
- `crates/gororoba_bevy_quantum`

The current CPU implementation still reuses upstream `quantum_core`,
`casimir_core`, and `spin_tomography_core` internally, but the game layer now
consumes only local request/response and diagnostics types:

- lattice sizing and entropy estimation
- Casimir point evaluation
- optional 3D Casimir field generation
- diagnostics for HUD and measurement history

## Smallest stable local-kernel boundary

The minimum engine-owned Rust boundary is now captured by
`gororoba_kernel_api::quantum`:

- `SpinLatticeConfig`
- `CasimirGeometry`
- `CasimirWorldlineConfig`
- `CasimirFieldRequest`
- `CasimirPointSample`
- `CasimirFieldSnapshot`
- `QuantumDomainConfig`
- `QuantumDiagnosticsSnapshot`
- `QuantumKernel`

This is the smallest useful cut because the game only needs:

1. lattice inputs
2. entropy estimate outputs
3. Casimir point/field requests and results
4. diagnostics snapshots for UI and measurement history

## Remaining blockers

1. `gororoba_kernel_quantum` still uses upstream internals instead of a fully
   local implementation.
2. `SpherePlateSphere` remains intentionally unimplemented in the current lane.
3. The 3D field is still cached in the Bevy-facing resource for rendering
   convenience.
4. Measurement flow depends on game-side resource history, so the local kernel
   should only expose deterministic snapshots, not UI state.

## Highest-value next extraction

1. Keep measurement logs, gizmos, and editor interactions in the game crate.
2. Expand `gororoba_kernel_quantum` parity tests so entropy and Casimir outputs
   can be trusted independently of Bevy.
3. Feature-gate unsupported geometries explicitly until they are local.

## Verification commands

```bash
scripts/cargo-isolated.sh check -p gororoba_kernel_api -p gororoba_bevy_quantum -p quantum_builder --all-targets --locked
scripts/cargo-isolated.sh clippy -p gororoba_kernel_api -p gororoba_bevy_quantum -p quantum_builder --all-targets -- -D warnings
scripts/cargo-isolated.sh test -p gororoba_kernel_quantum -p gororoba_bevy_quantum --lib
```
