# cdda_app DOX

## Purpose
Owns the binary entry point, Bevy app wiring, and runtime startup.

## Ownership
- `cdda_app/src/main.rs` and `cdda_app/src/lib.rs` live here.
- Runtime assets under `cdda_app/assets/` are also owned by this doc.

## Local Contracts
- App wiring should register plugins in a clear Input → Sim → Render order.
- Runtime behavior should remain configurable through Bevy resources and existing game state.

## Work Guidance
- Keep app code thin: prefer moving behavior into focused subsystem crates.
- When adding startup behavior, document it here or in the owning subsystem doc.

## Verification
- Run `cargo check -p cdda_app`.
- Run `cargo run -p cdda_app` for runtime startup validation when app wiring changes.

## Child DOX Index
