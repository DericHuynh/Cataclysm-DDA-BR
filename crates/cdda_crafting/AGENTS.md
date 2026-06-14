# cdda_crafting DOX

## Purpose
Owns crafting recipes, component consumption, and progress tracking.

## Ownership
- Crafting systems, recipe lookup, and crafting activity integration live in this crate.
- Recipe definitions and item data remain owned by `cdda_data` and `cdda_item`/`cdda_inventory`.

## Local Contracts
- Crafting should consume recipe definitions through the definition registry.
- Crafting progress should integrate with `cdda_activity` without creating circular dependencies.

## Work Guidance
- Keep recipe lookup separate from activity lifecycle.
- Use existing inventory and item relationships for component consumption.

## Verification
- Run `cargo check -p cdda_crafting`.
- Run `cargo test -p cdda_crafting` when crafting behavior changes.

## Child DOX Index
