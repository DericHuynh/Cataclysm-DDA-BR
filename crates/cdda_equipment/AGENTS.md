# cdda_equipment DOX

## Purpose
Owns equipment component registration and related contracts.

## Ownership
- Equipment-related component registration lives in this crate.
- Item relationships and inventory behavior remain in `cdda_item` and `cdda_inventory`.

## Local Contracts
- Equipment behavior should reuse canonical item and actor relationships.
- Avoid duplicating item, inventory, or actor logic.

## Work Guidance
- Keep this crate focused on equipment-facing component contracts.
- Coordinate with `cdda_item` when adding equipment-related item behavior.

## Verification
- Run `cargo check -p cdda_equipment`.
- Run `cargo test -p cdda_equipment` when equipment behavior changes.

## Child DOX Index
