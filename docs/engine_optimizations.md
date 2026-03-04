# Engine Optimizations

This document records the core architecture decisions and optimizations
in the gororoba_app engine. Each section explains the problem, the
failed naive approach, the root cause, and the production solution.

---

## 1. UI Interaction: bevy_egui Multipass Schedule Resolution

### Problem

All egui widgets (sliders, buttons, combo boxes, window drag) were
visually rendered but completely non-interactive. `Response::clicked()`
and `Response::hovered()` returned false for every widget. The pointer
event pipeline was confirmed working: `primary_clicked()` returned true,
`is_pointer_over_area()` returned true, but `is_using_pointer()` was
always false.

### Root Cause

bevy_egui 0.39.1 defaults to multipass mode
(`enable_multipass_for_primary_context: true`). In this mode the
`PrimaryEguiContext` entity receives an `EguiMultipassSchedule`
component. This causes `begin_pass_system` and `end_pass_system` (which
filter with `Without<EguiMultipassSchedule>`) to skip the primary
context entirely.

Instead, `run_egui_context_pass_loop_system` runs in PostUpdate and
calls `ctx.run()`, which internally executes:

```
begin_pass(input)          // clears this_pass.widgets, runs hit_test
  -> EguiPrimaryContextPass schedule
end_pass()                 // swaps this_pass <-> prev_pass
```

When UI systems are registered in the `Update` schedule (which runs
before PostUpdate), widgets are created outside any active pass.
`begin_pass()` then clears `this_pass.widgets`, erasing them. The
`EguiPrimaryContextPass` schedule runs with no UI systems, producing
an empty widget set. `end_pass()` swaps this empty set into
`prev_pass`. On the next frame `hit_test()` queries `prev_pass.widgets`
and finds nothing -- hence no click, no hover, no drag.

### Solution

Move all egui UI systems from `Update` to `EguiPrimaryContextPass`:

```rust
// WRONG: widgets created outside the pass, erased by begin_pass
app.add_systems(Update, my_ui_system.run_if(egui_ready));

// CORRECT: widgets created inside ctx.run() -> begin_pass/end_pass
app.add_systems(EguiPrimaryContextPass, my_ui_system.run_if(egui_ready));
```

Non-egui systems (camera controllers, gizmo renderers, input handlers,
physics stepping) remain in `Update`. The `EguiGlobalSettings {
enable_absorb_bevy_input_system: true }` resource is still inserted
before `GororobaCorePlugin` to prevent camera systems from consuming
pointer events when egui wants focus.

### Files Changed

| File | Schedule Change |
|------|----------------|
| `crates/gororoba_bevy_core/src/hud.rs` | `Update` -> `EguiPrimaryContextPass` |
| `crates/gororoba_bevy_core/src/pedagogy.rs` | `Update` -> `EguiPrimaryContextPass` |
| `games/fluid_dynamics/src/ui.rs` | `Update` -> `EguiPrimaryContextPass` |
| `games/non_euclidean/src/ui.rs` | `Update` -> `EguiPrimaryContextPass` |
| `games/relativistic_space/src/ui.rs` | `Update` -> `EguiPrimaryContextPass` |
| `games/quantum_builder/src/ui.rs` | `Update` -> `EguiPrimaryContextPass` |

### Key Source References

- `bevy_egui-0.39.1/src/lib.rs:358` -- `enable_multipass_for_primary_context` defaults to `true`
- `bevy_egui-0.39.1/src/lib.rs:565` -- `insert_schedule_if_multipass` hook on `PrimaryEguiContext`
- `bevy_egui-0.39.1/src/lib.rs:1847` -- `begin_pass_system` filters `Without<EguiMultipassSchedule>`
- `bevy_egui-0.39.1/src/lib.rs:1916` -- `run_egui_context_pass_loop_system` calls `ctx.run()`
- `egui-0.33.3/src/context.rs:818` -- `ctx.run()` calls `begin_pass -> closure -> end_pass`
- `egui-0.33.3/src/context.rs:469` -- `begin_pass` clears `this_pass.widgets`
- `egui-0.33.3/src/context.rs:478` -- `hit_test` queries `prev_pass.widgets`
- `egui-0.33.3/src/context.rs:2540` -- `end_pass` swaps `prev_pass <-> this_pass`

---

## 2. LBM Mass Conservation: Density-Corrected Neumann Outlet

### Problem

The LBM wind tunnel showed uniform flow: no disturbance developed
around solid obstacles, drag and lift coefficients stayed at zero, and
the velocity field decayed to rest. The upstream lbm_3d solver uses
`rem_euclid` for all three spatial dimensions, making the domain
unconditionally periodic with no inlet or outlet.

### Naive Approach (Failed)

A simple zero-gradient (Neumann) outlet copies distributions from the
adjacent interior slice (`x = nx-2`) to the outlet face (`x = nx-1`).
While this sustains flow in the short term, it introduces a systematic
mass drift: the outlet density deviates from the freestream density by
a small amount each timestep. Over thousands of steps the accumulated
error causes a global pressure gradient that destabilizes the
simulation (NaN divergence at approximately 2000 steps on a 32x16x16
domain with tau=0.8, u=0.05).

### Root Cause

The zero-gradient copy does not enforce any constraint on the total
mass leaving the domain. When the outlet density drifts above the
inlet density, mass accumulates faster than it drains, creating a
compressive wave that amplifies with each reflection off the inlet.

### Solution

Density-corrected Neumann outlet: after copying distributions from
`x = nx-2`, rescale them so the outlet face density equals the
freestream density `rho_0`:

```
scale = rho_0 / local_rho(x=nx-2)
f_outlet[q] = f_source[q] * scale
```

This preserves the velocity profile from the interior (wakes exit
smoothly) while clamping the density to prevent mass accumulation. In
the perturbation formulation the rescaling becomes:

```
h_outlet[q] = (h_source[q] + w_q) * scale - w_q
```

The inlet uses a standard equilibrium Dirichlet condition, resetting
all 19 distributions to the BGK equilibrium at freestream velocity and
density. Both inlet and outlet exclude wall rows (`y in 1..ny-1`,
`z in 1..nz-1`) to avoid corner conflicts with bounce-back planes.

### Boundary Application Order (Per Substep)

1. Collision + streaming (BGK relaxation + lattice propagation)
2. Solid voxel bounce-back (obstacle no-slip walls)
3. Equilibrium inlet (x=0) + density-corrected outlet (x=nx-1)
4. Wall bounce-back on Y and Z planes (overwrites corners)

### Verification

- `inlet_outlet_maintains_freestream`: velocity > 0.01 at domain
  center after 200 steps
- `sustained_drag_with_inlet_outlet`: nonzero drag after 600 steps
  with 4x4x4 obstacle
- `outlet_density_corrected`: outlet face density matches rho_0
  within 1e-10 (f64) / 1e-4 (f32) after 50 steps
- `long_running_stability`: no NaN after 3000 steps with obstacle,
  sustained flow and nonzero drag

### Files

- `crates/gororoba_bevy_lbm/src/resources.rs:253` -- f64 inlet/outlet BC
- `crates/gororoba_bevy_lbm/src/soa_solver.rs:419` -- f32 inlet/outlet BC

---

## 3. LBM Cache-Line Efficiency: Structure-of-Arrays Memory Layout

### Problem

The upstream `lbm_3d::LbmSolver3D` uses Array-of-Structures (AoS)
layout: 19 consecutive f64 values per cell (`f[cell*19 + q]`). This
means streaming in direction `q` touches memory locations 19*8 = 152
bytes apart, defeating hardware prefetchers and causing one cache miss
per cell per direction. On a 64x32x32 domain (65536 cells), each
streaming pass performs approximately 1.2 million random-stride
accesses.

The collision phase has a separate pass over all cells, doubling the
memory traffic (read f, compute, write f).

### Solution

Structure-of-Arrays layout with fused pull-collide:

**SoA Layout.** One contiguous `Vec<f32>` per lattice direction:
`h[q][cell_idx]`. Streaming in direction `q` reads linearly through
`h[q]`, hitting sequential cache lines. On a CPU with 64-byte cache
lines, 16 consecutive f32 values share one line, giving 16:1 spatial
locality improvement over AoS.

**Fused Pull-Collide.** Instead of separate streaming and collision
passes, a single "pull" pass reads the 19 source values for each
destination cell, computes macroscopic quantities (rho, u), evaluates
the BGK equilibrium, and writes the post-collision result directly to
the scratch buffer. This halves the number of full-domain sweeps per
timestep and keeps the working set in L1/L2 cache.

**Pre-Computed Neighbor Table.** A `NeighborTable` maps each
`(destination_cell, direction)` pair to the source cell index,
pre-computing the `rem_euclid` modular arithmetic at initialization.
This eliminates integer division from the hot loop (rem_euclid compiles
to a branch + conditional add on x86, which is unpredictable when
coordinates wrap).

**Rayon Parallelism.** The pull-collide kernel runs via
`(0..n).into_par_iter()` with each cell writing to a unique scratch
index. No synchronization is needed. Raw pointers wrapped in a
`SendPtr` newtype provide the `Send + Sync` bounds required by rayon
without runtime overhead.

**Target-CPU Native.** `.cargo/config.toml` sets
`rustflags = ["-C", "target-cpu=native"]` for x86_64, enabling
AVX2/FMA autovectorization of the equilibrium computation (3 FMA
operations per direction: `cu * inv_cs_sq`, `cu * cu * inv_2cs4`,
`u_sq * half_inv_cs_sq`).

### Performance

41.9 MLUPS (million lattice updates per second) on a 64x32x32 domain,
measured on a Zen 4 processor with 96 MB V-Cache. The upstream f64 AoS
solver achieves approximately 3-5 MLUPS on the same hardware.

### Files

- `crates/gororoba_bevy_lbm/src/soa_solver.rs:100` -- `NeighborTable`
- `crates/gororoba_bevy_lbm/src/soa_solver.rs:280` -- `fused_stream_collide`
- `.cargo/config.toml` -- `target-cpu=native`

---

## 4. LBM Floating-Point Stability: f32 Perturbation Formulation

### Problem

Running the LBM solver in f32 (for SIMD width and memory bandwidth)
causes catastrophic precision loss. In a D3Q19 lattice at rest
(`rho = 1.0`, `u = 0`), the distribution values are the lattice
weights themselves:

```
f_0 = 1/3   ~ 0.3333
f_1 = 1/18  ~ 0.0556
f_7 = 1/36  ~ 0.0278
```

At a typical freestream velocity (`u = 0.05`), the equilibrium
perturbation is:

```
f_1^eq - f_1^rest = w_1 * rho * (c_1*u/c_s^2 + ...) ~ 0.0083
```

This perturbation is 6.7x smaller than the rest value. In f32 with
approximately 7 significant digits, the subtraction
`f_new - f_old ~ 0.064 - 0.056 = 0.008` loses 1-2 significant digits.
Over hundreds of timesteps the accumulated truncation error overwhelms
the physical signal, producing either numerical viscosity (damped flow)
or outright instability.

### Solution

Store perturbation distributions `h_i = f_i - w_i` instead of the
full distributions `f_i`. At rest, all `h_i = 0`. The physical signal
(density fluctuations, velocity-induced asymmetry) is directly
represented without the equilibrium bias consuming precision.

The BGK collision operator transforms naturally:

```
Standard:  f_new = f - (f - f_eq) / tau
Perturb:   h_new = h - (h - h_eq) / tau

where h_eq = f_eq - w = w * [(rho - 1) + rho * velocity_terms]
```

Macroscopic recovery also shifts:

```
rho = 1 + sum_i h_i    (since sum_i w_i = 1)
u_k = (1/rho) * sum_i h_i * c_i^k   (since sum_i w_i * c_i^k = 0)
```

Bounce-back is weight-invariant: since `w_q = w_opp(q)` for all D3Q19
directions, reflecting `h` values is identical to reflecting `f` values
with no weight correction needed.

The density-corrected outlet requires converting back to full
distributions for the rescaling:

```
f_src = h_src + w
h_dst = f_src * scale - w = (h_src + w) * scale - w
```

### Precision Verification

At `rho = 1.0`, `u = 0.05`, the maximum equilibrium perturbation
magnitude across all 19 directions is `|h_eq| < 0.01` (verified in
`soa_perturbation_precision` test). This leaves 5+ significant digits
of f32 precision for the collision dynamics, compared to 1-2 digits
in the naive formulation.

The `soa_long_running_stability` test confirms no NaN or divergence
after 3000 steps with a solid obstacle, and the
`soa_outlet_density_corrected` test verifies outlet density accuracy
within `1e-4` of the freestream value.

### Files

- `crates/gororoba_bevy_lbm/src/soa_solver.rs:208` -- `equilibrium_perturbation`
- `crates/gororoba_bevy_lbm/src/soa_solver.rs:229` -- `compute_macroscopic` (h -> rho, u)
- `crates/gororoba_bevy_lbm/src/soa_solver.rs:457` -- outlet rescaling in perturbation form
