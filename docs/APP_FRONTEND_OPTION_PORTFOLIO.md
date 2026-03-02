# Frontend Option Portfolio

Date: 2026-02-13

This portfolio proposes cross-platform app directions that reuse `open_gororoba`
as backend/data source while keeping frontend implementation inside this repository.

## Design option 1: Studio Command Deck (implemented first)

- Platform: Web SSR first, then desktop shell.
- Stack: Axum + Askama + Rust typed client.
- UX style: Mission-control dashboard with fast run/benchmark/repro actions.
- Core value: Reliable operator UI for evidence pipelines with minimal setup.
- Status: Implemented in `apps/studio_web`.

## Design option 2: Pocket Lab Companion

- Platform: Android + iOS.
- Stack direction: Rust core + lightweight native shell or Rust-centric UI framework.
- UX style: Swipeable "pipeline cards" + offline queue for run requests.
- Novel behavior: "Field notebook" mode to annotate benchmark snapshots with audio notes.
- Status: Architecture spike scaffolded in `apps/mobile_spike`.

## Design option 3: Desktop Mission Graph

- Platform: Linux, BSD, Windows, macOS.
- Stack direction: Rust desktop shell with embedded webview or native Rust UI.
- UX style: Node-link graph for experiment lineage and artifact provenance paths.
- Novel behavior: Time-travel scrubber across run history and regression windows.
- Status: Packaging lane initialized via `tools/xtask` and `scripts/package_desktop_matrix.sh`.

## Design option 4: Evidence Atlas

- Platform: Web + desktop.
- UX style: Layered map view where each thesis pipeline is a region with sub-metrics.
- Novel behavior: Dynamic confidence heatmaps, click-through to raw artifact links.

## Design option 5: Synthesis Arena (game-inspired)

- Platform: WebGL or desktop.
- UX style: Strategy-board visualization of pipeline tradeoffs.
- Novel behavior: "Challenge scenarios" where users tune profile parameters to hit gates.
- Status: Prototype implemented in `apps/synthesis_arena`.

## Design option 6: Physics Engine Sandbox

- Platform: Desktop and web.
- UX style: Real-time simulation panel + split metrics console.
- Novel behavior: Live parameter perturbation with automatic benchmark/repro scoring.
- Status: Prototype implemented in `apps/physics_sandbox`.

## Design option 7: Live Broadcast Wall

- Platform: Web large-screen dashboards.
- UX style: Rotating tiles for active runs, failures, and reproducibility confidence.
- Novel behavior: Automatic "incident story" timeline generated from API history stream.

## Design option 8: Research Story Composer

- Platform: Web + desktop.
- UX style: Narrative editor that drags run results into publication-ready sections.
- Novel behavior: Auto-citation links from each chart panel back to source artifact paths.

## Design option 9: Multi-Agent Operator Console

- Platform: Desktop.
- UX style: Parallel task lanes for backend checks, frontend checks, provenance checks.
- Novel behavior: One-click orchestration with deterministic runbooks and gate receipts.

## Design option 10: Minimal TUI Ops Client

- Platform: Linux/BSD terminals.
- Stack direction: Rust + terminal UI crate.
- UX style: Keyboard-first quick operations and compact status panes.
- Novel behavior: Near-zero resource "watch mode" for long benchmark sessions.

## UI/UX experience themes

1. Fast path first: one action to run quick profile, one action to run full profile.
2. Explainability by default: every metric panel links to source experiment identifiers.
3. Mobile parity: same action language on touch and desktop pointer interactions.
4. Operational calm: predictable color semantics (teal success, amber attention, red failure).
5. Deterministic trust: every page surfaces API version and backend endpoint origin.
6. Layered pedagogy: Story/Explorer/Research switch adapts depth for students through researchers.
7. Visual-first cognition: interactive canvases in both studio and sandbox apps support intuition before formulas.
