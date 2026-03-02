# Gororoba Games -- License Policy

## Summary

Games and Bevy plugin crates that link GPL-2.0-only open_gororoba crates
inherit the GPL-2.0-only license. Crates that link only MIT/Apache-2.0
dependencies may use permissive licenses.

## Per-Crate Licenses

| Crate                  | License        | Reason                                    |
|------------------------|----------------|-------------------------------------------|
| gororoba_bevy_core     | GPL-2.0-only   | Shared infrastructure; workspace license  |
| gororoba_bevy_lbm      | GPL-2.0-only   | Links lbm_vulkan (GPL-2.0-only)           |
| gororoba_bevy_algebra  | GPL-2.0-only   | Links cd_kernel (GPL-2.0-only)            |
| gororoba_bevy_gr       | MIT            | gr_core and cosmology_core are MIT        |
| gororoba_bevy_quantum  | GPL-2.0-only   | Links casimir_core (GPL-2.0-only)         |
| fluid_dynamics         | GPL-2.0-only   | Links gororoba_bevy_lbm (GPL)             |
| non_euclidean          | GPL-2.0-only   | Links gororoba_bevy_algebra (GPL)         |
| relativistic_space     | GPL-2.0-only   | Links gororoba_bevy_core (GPL)            |
| quantum_builder        | GPL-2.0-only   | Links gororoba_bevy_quantum (GPL)         |

## Existing Apps (unchanged)

studio_web, physics_sandbox, and synthesis_arena remain Apache-2.0 OR MIT.
They communicate with open_gororoba via HTTP API boundaries only and do
NOT link any GPL crates.

## Rationale

GPL-2.0-only propagates through static linking. Rust `use` + Cargo
dependency = static linking. The HTTP API boundary is a process boundary,
so the existing web apps are not derivative works.
