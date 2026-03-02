# Mobile Architecture Spike

## Rust-first topology

1. Shared Rust domain crate provides:
- learning mode taxonomy (Story/Explorer/Research)
- lesson metadata per thesis pipeline
- simulation and benchmark primitives (when portable)

2. Platform adapters:
- Android: Kotlin layer calls Rust via FFI bridge module.
- iOS: Swift layer calls Rust via C-compatible shim.

3. Network policy:
- Mobile clients read from backend `studio.v1` endpoints.
- Local mode and lesson fallback remains available offline.

## Integration seams

1. Serialization boundary: JSON payloads for fast prototyping.
2. Long-term boundary: generated bindings for strongly typed APIs.
3. Testing boundary: replay fixtures from backend responses and validate parity.

## Milestones

1. Generate contract file from workspace automation (`xtask mobile-contract`).
2. Implement Android proof-of-concept screen with mode toggles and lesson rendering.
3. Implement iOS proof-of-concept screen with matching parity tests.
4. Promote contract to semver and enforce in CI.
