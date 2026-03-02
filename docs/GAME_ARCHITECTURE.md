# Gororoba Games Platform -- Architecture

## Overview

Four physics games built on Bevy 0.18, wrapping open_gororoba's physics
crates as Bevy plugins. Each game pairs a shared infrastructure layer
(gororoba_bevy_core) with a domain-specific physics plugin.

## Repo Layout

```
gororoba_app/
  crates/
    gororoba_bevy_core/     Shared: camera, HUD, pedagogy, input, GameState
    gororoba_bevy_lbm/      LBM fluid dynamics (wraps lbm_vulkan, lbm_3d)
    gororoba_bevy_algebra/  Cayley-Dickson algebra (wraps cd_kernel, algebra_core)
    gororoba_bevy_gr/       General relativity (wraps gr_core, cosmology_core)
    gororoba_bevy_quantum/  Quantum/Casimir (wraps quantum_core, casimir_core)
  games/
    fluid_dynamics/         Game 1: vehicle design + wind tunnel (LBM)
    non_euclidean/          Game 2: non-Euclidean puzzle (CD algebra)
    relativistic_space/     Game 3: black hole exploration (GR)
    quantum_builder/        Game 4: nanoscale sandbox (quantum)
```

## Plugin Pattern

Each physics crate is wrapped as a Bevy `Plugin`:

- Resources wrap engine state (e.g., `LbmEngineResource` wraps `GororobaEngine`)
- Components represent domain entities (e.g., `VoxelGrid`, `BlackHole`)
- Systems run physics in `FixedUpdate` (deterministic, framerate-independent)
- Readback and rendering systems run in `Update` (display refresh rate)

## Critical Invariant: FixedUpdate vs Update

Physics simulation MUST run in `FixedUpdate` (default 64 Hz). Tying physics
to `Update` causes explosions on fast monitors and stalls on slow ones.

## ash-to-Bevy Bridge (fluid_dynamics only)

The LBM game uses dual Vulkan instances:
1. ash (lbm_vulkan) for LBM compute
2. wgpu (Bevy) for presentation

CPU readback bridge: `GororobaEngine::read_render_pixels()` copies RGBA from
GPU to CPU, then uploads to a Bevy `Image` asset each frame. Acceptable at
720p (~3.5 MB/frame). Games 2/3/4 run entirely in Bevy's wgpu pipeline.

## Shader Translation (relativistic_space)

Blackhole's GLSL shaders are translated to WGSL for Bevy's render pipeline.
Source GLSL lives in ~/Github/Blackhole/shader/. LUT data (CSV) is loaded
at runtime and converted to GPU textures.

## Dependencies

- Bevy 0.18.0, bevy_egui 0.39
- open_gororoba crates via git deps with [patch] override for local dev
- nalgebra 0.33 (matching open_gororoba's statrs 0.18 constraint)

## Implementation Phases

0. Workspace setup and git init (this phase)
1. gororoba_bevy_core (shared infrastructure)
2. gororoba_bevy_lbm (LBM plugin)
3. Game 1: Fluid Dynamics MVP
4. gororoba_bevy_algebra (CD algebra plugin)
5. Game 2: Non-Euclidean Puzzle MVP
6. gororoba_bevy_gr + Game 3: Relativistic Space
7. gororoba_bevy_quantum + Game 4: Quantum Builder
