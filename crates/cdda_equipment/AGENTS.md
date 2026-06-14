# cdda_equipment DOX

## Purpose
Wield and wear items on creatures. Owns the public equipment API and the Bevy
plugin entry point. Sits in Layer 3 (game logic, Bevy ECS only) per
`crates/AGENTS.md`; no full Bevy dependency, no app/CLI coupling.

## Ownership
- `Cargo.toml` depends on `bevy_ecs` and `bevy_app` only — deliberately thin so
  other Layer 3 crates can pull this API without dragging in the renderer.
- Relationship components themselves (`WieldedBy`/`WieldedItems`,
  `WornOn`/`WornBy`, `MountedOn`/`MountedPockets`) are declared in
  `cdda_components::item`; this crate operates on them but does not own them.
- Per-armor data (incl. raw `encumbrance` field on `ArmourPart`) lives in
  `cdda_components::def`; no encumbrance math is implemented here yet.
- Item containment/inventory helpers remain in `cdda_item` and `cdda_inventory`.

## Local Contracts
- Bevy relationship pairs in use (see `cdda_components/src/item.rs`):
  - `WieldedBy(Entity)` ↔ `WieldedItems(Vec<Entity>)` — single active hand.
  - `WornOn { wearer, slot: Option<String> }` ↔ `WornBy(Vec<Entity>)` — slot-tagged.
  - `MountedOn(Entity)` ↔ `MountedPockets(Vec<Entity>)` — pocket attachment.
- `EquipError` variants (`systems.rs`): `AlreadyWielding(Entity)`,
  `NoFreeHands`, `SlotOccupied(String)`, `ItemTooHeavy`, `ItemTooLarge`,
  `NotEquippable`. `Debug + Clone + PartialEq + Eq`.
- Public API (`systems.rs`): `wield_item`, `unwield`, `wear_item`, `take_off`,
  `available_slots`. All take `&mut World` (not `Commands`) — bodies are
  `todo!()` and not yet wired into any schedule.
- `EquipmentPlugin` (`plugin.rs`) is currently a no-op `Plugin`; it is not
  registered in `cdda_app`. Systems must be invoked manually via the
  free functions in `systems` until the plugin grows a real `build`.
- Mutations must go through `commands.insert()`/`entity_mut().insert()` so Bevy
  relationship hooks keep the target collection in sync — never edit
  `WieldedItems.0` directly.

## Work Guidance
- Keep `Cargo.toml` minimal; do not add `bevy` (full) or app-shell deps.
- When filling in `todo!()` bodies, prefer `World` access over `Commands` to
  match the existing signatures; migrate the whole surface to `Commands` in
  one pass if you change it.
- Encumbrance work belongs in a follow-up: read `ArmourPart::encumbrance` from
  `cdda_components::def` and aggregate per body part. Do not duplicate
  armor data here.
- New equipment-related Bevy events should go in `cdda_events`, not here.

## Verification
- `cargo check -p cdda_equipment` for compile sanity (all `todo!()` resolve).
- `cargo nextest run -p cdda_equipment` (or `cargo test -p cdda_equipment`
  if `nextest` is unavailable) for crate-local tests.
- Workspace tests touching this crate:
  - `tests/wield_wear_test.rs` — relationship hook behavior (runs).
  - `tests/equipment_system_test.rs` — full API behavior; every test is
    `#[ignore = "equipment system not yet implemented"]` and will only pass
    once the `todo!()` bodies are implemented.

## Child DOX Index

- `src/lib.rs` — crate root, re-exports `plugin` and `systems` modules.
- `src/plugin.rs` — `EquipmentPlugin` (no-op Bevy `Plugin` stub).
- `src/systems.rs` — `EquipError` enum, public wield/wear/unwield/take_off
  functions, and `available_slots` query.
