# Mobile Spike (Android + iOS)

This folder contains the architecture spike for mobile clients that consume the
Rust-first shared core and Gororoba backend contracts.

## Goals

1. Reuse Rust domain logic and learning layers across Android/iOS.
2. Keep network contract aligned with `studio.v1`.
3. Support Story/Explorer/Research pedagogical modes in mobile UI.

## Current artifacts

- `ARCHITECTURE.md`: integration topology and milestones.
- `contracts/shared_core_contract.json`: generated interface contract.
- `android/`: Kotlin integration notes.
- `ios/`: Swift integration notes.

## Regenerate contract

```bash
scripts/generate_mobile_contract.sh
```
