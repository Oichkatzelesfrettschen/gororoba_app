# App Purpose And Audiences

Verified on: 2026-02-13

## What this app is for

`gororoba_app` is a Rust-first frontend workspace for the `open_gororoba`
experiment ecosystem. It gives operators, learners, and researchers one place
to:

1. Run and benchmark experiment pipelines exposed by the backend.
2. Inspect reproducibility and gate outcomes.
3. Learn the underlying science and math at different depth levels.
4. Preserve artifact lineage back to repository paths and references.

## Who it is for

1. Story Mode (7th grade):
   - Plain-language summaries.
   - Analogy-driven explanations.
   - Hands-on mini activities.
2. Explorer Mode (curious adult):
   - Equation hints and walkthroughs.
   - Guided interpretation of plots and metrics.
   - Practical artifact-reading guidance.
3. Research Mode (advanced user):
   - Full equations.
   - Derivation steps, assumptions, and proof sketches.
   - Artifact paths and external reference links.

## What it does today

1. Studio web app (`apps/studio_web`):
   - Runs pipelines and suite actions through backend API `studio.v1`.
   - Displays run history, benchmark summaries, and reproducibility checks.
   - Provides Story/Explorer/Research learning panels per pipeline.
   - Includes browser visual lab (history chart + oscillator + phase portrait).
2. Physics sandbox (`apps/physics_sandbox`):
   - Deterministic damped-oscillator simulation endpoint.
   - Benchmark endpoint with timing/stability metrics.
   - Interactive controls and two live plots (time-series + phase plot).
   - Story/Explorer/Research content in the same UI shell.
3. Synthesis arena (`apps/synthesis_arena`):
   - Challenge catalog with gate/weight profiles.
   - Deterministic evaluation endpoint with four metric gates.
   - Benchmark endpoint with score and duration statistics.
   - Story/Explorer/Research learning panels plus visual scoreboard chart.

## Science and math depth policy

Depth is now explicit, and each pipeline lesson includes:

1. Core equation.
2. Equation walkthrough steps.
3. Derivation steps.
4. Assumptions.
5. Proof sketch.
6. Artifact links and reference links.

This makes the app usable for early learners while retaining a path to
research-grade interpretation.
