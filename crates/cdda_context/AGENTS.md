# cdda_context DOX

## Purpose
Owns context state machine, navigation, focus, overlays, and menus.

## Ownership
- Context state, navigation stack, overlay stack, focus, and menu components live in this crate.
- Screen rendering remains in `cdda_render`.

## Local Contracts
- Context transitions should be driven through shared state and navigation contracts.
- Rendering systems should read context state rather than mutate navigation directly.

## Work Guidance
- Keep context code independent from render/input crates.
- Preserve deterministic context behavior for tests and replay.

## Verification
- Run `cargo check -p cdda_context`.
- Run relevant context or screen integration tests when navigation behavior changes.

## Child DOX Index
