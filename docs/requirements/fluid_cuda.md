# Fluid CUDA Requirements

## Scope

This lane covers local CUDA capability probing and future local CUDA fluid
execution for `gororoba_app`.

## Current state

- `crates/gororoba_kernel_fluid_cuda` is local and compiles.
- Capability probing is local and validates `cudarc` context creation.
- Metadata probing also checks `nvidia-smi` and `nvcc --version` when present.
- A local CUDA backend object now exists and participates in backend selection
  and frame generation.
- In this phase, simulation stepping still delegates to the local CPU kernel.

## Host requirements

1. NVIDIA driver with a working CUDA-capable device.
2. `cudarc`-compatible driver/runtime environment.
3. `nvidia-smi` is recommended for metadata checks.
4. `nvcc` is optional but used to report toolkit version when available.

## Verification

```bash
scripts/cargo-isolated.sh check -p gororoba_kernel_fluid_cuda -p gororoba_bevy_lbm --features fluid-cuda --all-targets --locked
```

Run this lane separately from other compile-heavy Cargo commands.
