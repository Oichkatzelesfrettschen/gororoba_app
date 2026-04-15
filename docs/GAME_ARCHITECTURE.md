# Gororoba Engine Architecture

## Overview

`gororoba_app` is a self-contained Rust game-engine microcosm that is being
built out domain by domain. The repository contains both gameplay code and the
local engine/kernel interfaces that those games depend on.

`open_gororoba` is the reference kernel for research, discovery, and parity
validation. It is not the desired steady-state runtime dependency for game
features here.

## Current architecture

```text
gororoba_app/
  crates/
    gororoba_kernel_api/       Stable engine-facing traits and snapshot types
    gororoba_kernel_algebra/   Local Cayley-Dickson kernel
    gororoba_kernel_fluid/     Local CPU fluid kernels and backend factory
    gororoba_kernel_fluid_vulkan/ Local Vulkan capability/probe lane
    gororoba_kernel_fluid_cuda/ Local CUDA capability/probe lane
    gororoba_bevy_core/        Shared Bevy camera, HUD, states, pedagogy
    gororoba_bevy_algebra/     Bevy algebra systems using local kernel crates
    gororoba_bevy_lbm/         Fluid bridge on local fluid APIs
    gororoba_bevy_gr/          Relativity bridge on local contracts
    gororoba_bevy_quantum/     Quantum bridge on local contracts
    gororoba_bevy_game_semantics/ Shared gameplay semantics
  games/
    fluid_dynamics/
    non_euclidean/
    relativistic_space/
    quantum_builder/
    interaction_arena/
  apps/
    studio_web/
    physics_sandbox/
    synthesis_arena/
```

## Boundary rules

1. Local engine/gameplay APIs live in `gororoba_app`.
2. Games should depend on local kernel traits and local Bevy bridge crates.
3. `open_gororoba` is allowed as a reference source, parity oracle, and future
   upstream destination for novel findings.
4. Direct runtime dependence on `open_gororoba` should be retired slice by slice.

## Algebra lane

The first localized runtime lane is algebra:

- `gororoba_kernel_api` defines stable algebra and projection types.
- `gororoba_kernel_algebra` provides a self-contained Cayley-Dickson kernel.
- `gororoba_bevy_algebra` wraps the local kernel for ECS use.
- `games/non_euclidean` consumes that local path.

This lane now supports:

- local multiplication and associator evaluation
- local zero-divisor search for dimensions >= 16
- deterministic coefficient projection into 3D portal placement

## Remaining migration lanes

These domains still need deeper local implementation work:

- fluid Vulkan/CUDA execution backends still need native stepping imported
  beyond the current local hybrid backend objects
- general relativity internals
- quantum/Casimir internals
- strategy/game-theory evaluation

## Real-math gameplay constraints

1. Simulation math belongs in kernel crates, not ad hoc game systems.
2. Bevy systems should consume snapshots, diagnostics, and projected structures.
3. Puzzle success criteria should be based on real invariants, not placeholder
   heuristics.
4. Higher-dimensional structures must be projected into 3D deterministically.

## Quality policy

1. Physics/math stepping belongs in `FixedUpdate` unless there is a strong reason
   otherwise.
2. Presentation and gizmo/HUD refresh belongs in `Update`.
3. Clippy runs with `-D warnings`.
4. Locked build/test commands are the expected steady-state verification path.
