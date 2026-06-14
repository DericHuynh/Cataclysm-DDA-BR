# cdda_activity DOX

## Purpose
Owns player activity systems and multi-turn activities.

## Ownership
- Activity state, activity processing, and related systems live in this crate.
- Activity definitions and data-driven behavior remain coordinated with `cdda_data`.

## Local Contracts
- Activities should model multi-turn player intent without duplicating inventory or actor logic.
- Activity systems should communicate through existing events and components.

## Work Guidance
- Keep activity code focused on lifecycle and progress tracking.
- Reuse actor, inventory, and context contracts instead of reaching into their internals.

## Verification
- Run `cargo check -p cdda_activity`.
- Run `cargo test -p cdda_activity` when activity behavior changes.

## Child DOX Index
