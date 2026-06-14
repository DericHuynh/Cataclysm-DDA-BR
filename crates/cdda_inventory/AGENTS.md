# cdda_inventory DOX

## Purpose
Owns inventory-side systems: invlet allocation, stack merging, the per-frame
`InventoryBin` cache, item movement between ground / container / wielded, the
body-pocket helper, and the shared `ExaminedItem` resource.

## Ownership
- Item component types (`StackCount`, `CurrentCharges`, `ItemDamage`,
  `InsideContainer`, `ContainerContents`, `WieldedBy`, `WornOn`, `MountedOn`,
  `Pocket`, `Container`, `Invlet`, `INVLET_CHARS`, `FLOOR_CAP_ML`) live in
  `cdda_components::item`.
- The `ItemMoveEvent` / `MoveLocation` message types live in
  `cdda_components::events`. This crate only consumes them.
- Wielding/wearing relationship contracts live in `cdda_item` and
  `cdda_equipment`; this crate coordinates with them but does not own the
  relationship components themselves.
- `cdda_actor` AP costs (`AP_COST_PICKUP`, `AP_COST_WIELD`) are read here.

## Local Contracts
- **Movement contract** — dev pickup/drop and scripted moves emit
  `ItemMoveEvent` messages; `process_item_move_events` (in `src/systems.rs`)
  is the single point that applies the component changes. Six transitions
  are handled: Ground↔Container, Ground↔Wielded, Wielded→Container. On
  Container→Ground and Wielded→Ground the `Invlet` is removed.
- **Invlet allocation** — `assign_invlets_system` queries
  `(InsideContainer, Without<Invlet>)` and `(WieldedBy, Without<Invlet>)`,
  groups items by owning creature (via `pocket::find_creature_for_pocket`),
  merges identical stacks, then assigns a char preferring the item's
  existing char from `fav_chars` before scanning `INVLET_CHARS` (a..z).
  Items with no free char stay without an `Invlet`.
- **Stack merge rule** — two items merge only if `DefOrigin` (or fallback
  `DefStrId`) match, `ItemDamage` match, and `CurrentCharges` match.
  `merge_or_stack` sums `StackCount` and `CurrentCharges` on the target
  and despawns the incoming entity.
- **Body pocket** — `pocket::spawn_body_pocket` creates an `IsPocket` /
  `MountedOn(player)` entity with effectively unlimited volume and weight
  (`u64::MAX / 2`); per-item volume/weight enforcement is deferred.
- **Floor drop cap** — `dev_pickup_drop_system` refuses a drop that would
  exceed `FLOOR_CAP_ML` total volume at the camera's OMT tile.
- **Hand limit** — wielding from a pocket requires `WieldedItems.len() <
  HandCount.0`; otherwise a `tracing::warn!` fires and the action is
  rejected.
- **Position walk** — `effective_position` walks the `InsideContainer`
  chain (hard cap 64 hops) to find a parent with `WorldPosition`.

## Work Guidance
- When moving items, prefer emitting `ItemMoveEvent` over mutating
  relationship components directly. `process_item_move_events` is the only
  place that should resolve the six move transitions. Direct `Commands`
  mutation is used in `inventory_screen_input` for the wield/unwield
  shortcut and is acceptable there because it is UI-scoped.
- Keep `InventoryBin` derivation read-only. Systems that change inventory
  state do not need to touch the bin; `build_inventory_bins` re-scans each
  frame.
- Body pocket is currently a single omnibus entity per player. Multiple
  nested pockets are supported by the helper API but not yet exercised by
  spawning code outside tests.
- `src/mod.rs` is **vestigial and inconsistent** with `src/lib.rs` — it
  references a non-existent `plugin` module. Cargo compiles via `lib.rs`
  only; do not delete `mod.rs` without first removing its stale contents.

## Verification
- `cargo check -p cdda_inventory` — fast type check (passes today; emits
  3 dead-code warnings for `used_invlets`, `allocate_invlet_for`,
  `find_merge_target_for_creature` — the `_q` variants are the ones
  actually called from systems).
- `cargo test -p cdda_inventory` — unit tests live in `src/systems.rs`
  (`mod tests`, ~14 cases covering invlet allocation, removal-on-drop, and
  merge rules for same/diff type/charges/damage).
- `cargo nextest run -p cdda_inventory` — preferred runner per root
  `AGENTS.md` user preferences.
- `tests/inventory_system_test.rs` exercises `can_fit_in_container`,
  `total_container_volume`, and friends against pocket rules.

## Child DOX Index
- `src/examine_resource.rs` — `ExaminedItem(Option<Entity>)` resource set
  by the inventory screen on Examine, read by `cdda_render` examine
  overlay.
- `src/pocket.rs` — body-pocket spawn and `MountedOn` → creature
  resolution helpers (`spawn_body_pocket`, `get_body_pocket`,
  `find_creature_for_pocket`).
- `src/systems.rs` — all inventory systems (`assign_invlets_system`,
  `build_inventory_bins`, `process_item_move_events`,
  `inventory_screen_input`, `dev_pickup_drop_system`), the
  `InventoryBin` resource, and public helpers (`effective_position`,
  `items_at_position`, `items_in_container`, `can_fit_in_container`,
  `total_container_volume`, `total_container_weight`, `merge_or_stack`,
  `pickup_item`, `drop_item`, `transfer_item`).
