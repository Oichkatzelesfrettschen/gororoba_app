# Fluid Vulkan Requirements

## Scope

This lane covers local Vulkan capability probing and future local Vulkan fluid
execution for `gororoba_app`.

## Current state

- `crates/gororoba_kernel_fluid_vulkan` is local and compiles.
- Capability probing is local and uses `ash` directly.
- A local Vulkan backend object now exists and participates in backend
  selection and frame generation.
- In this phase, simulation stepping still delegates to the local CPU kernel.

## Host requirements

1. A Vulkan loader visible to the process.
2. A Vulkan-capable GPU and driver.
3. `vulkaninfo` is recommended for operator sanity checks, but runtime probing
   does not depend on it.

## Verification

```bash
scripts/cargo-isolated.sh check -p gororoba_kernel_fluid_vulkan -p gororoba_bevy_lbm --features fluid-vulkan --all-targets --locked
```

Run this lane separately from other compile-heavy Cargo commands.
