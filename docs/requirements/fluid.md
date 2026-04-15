# Fluid Lane Requirements

## Current role

The fluid lane now has a real local engine boundary in:

- `crates/gororoba_kernel_api`
- `crates/gororoba_kernel_fluid`
- `crates/gororoba_kernel_fluid_vulkan`
- `crates/gororoba_kernel_fluid_cuda`
- `crates/gororoba_bevy_lbm`

The current runtime is now local on CPU, and local at the capability/factory
layer for GPU backends:

- local scalar CPU solver path in `crates/gororoba_kernel_fluid`
- local SoA CPU solver path in `crates/gororoba_kernel_fluid`
- local Vulkan backend object plus capability probing in `crates/gororoba_kernel_fluid_vulkan`
- local CUDA backend object plus capability probing in `crates/gororoba_kernel_fluid_cuda`
- local aerodynamic post-processing in `games/fluid_dynamics`
- a generic frame bridge in `crates/gororoba_bevy_lbm/src/compute_bridge.rs`
- a thinner Bevy adapter crate that no longer owns backend-specific policy

## Smallest stable local-kernel boundary

The minimum engine-owned Rust boundary is now captured by
`gororoba_kernel_api::fluid`:

- `GridShape3`
- `FluidBoundaryMode`
- `FluidBoundaryConfig`
- `FluidBackendPreference`
- `CpuKernelFlavor`
- `FluidExecutionConfig`
- `FluidBackendCapabilities`
- `FluidDomainConfig`
- `FluidFieldSnapshot`
- `FluidDiagnosticsSnapshot`
- `AerodynamicSnapshot`
- `FluidRenderFrame`
- `FluidBackendError`
- `FluidKernel`

This is the smallest useful cut because the game only needs:

1. domain/grid creation and voxel mask injection
2. timestep stepping
3. density and velocity readback
4. diagnostics for HUD/results
5. aerodynamic force/coefficient outputs

## Current blockers

1. Local CPU execution is done.
2. Local Vulkan/CUDA backend objects now select and run in-repo, but their
   simulation stepping still delegates to the local CPU kernels in this phase.
3. `SimulationDiagnostics` is still a Bevy-facing mirror rather than a direct
   game-owned snapshot type.

## Verification commands

```bash
scripts/cargo-isolated.sh check -p gororoba_kernel_fluid -p gororoba_bevy_lbm -p fluid_dynamics --all-targets --locked
scripts/cargo-isolated.sh clippy -p gororoba_kernel_fluid -p gororoba_bevy_lbm -p fluid_dynamics --all-targets -- -D warnings
scripts/cargo-isolated.sh test -p gororoba_kernel_fluid --lib --locked
scripts/cargo-isolated.sh check -p gororoba_kernel_fluid_vulkan -p gororoba_bevy_lbm --features fluid-vulkan --all-targets --locked
scripts/cargo-isolated.sh check -p gororoba_kernel_fluid_cuda -p gororoba_bevy_lbm --features fluid-cuda --all-targets --locked
```

Run the GPU feature lanes one at a time.
