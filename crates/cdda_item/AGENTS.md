# cdda_item DOX

## Purpose
Item-side Bevy plugin crate. Owns `ItemPlugin`, which registers every item-related ECS component for Bevy reflection. No systems, no game logic.

## Ownership
- `src/plugin.rs` defines `ItemPlugin` (implements `bevy_app::Plugin`); `src/lib.rs` only declares the module.
- The component types and relationship definitions live in `cdda_components/src/item.rs`. This crate only adds reflection registration.
- Consumed by `cdda_app` via `app.add_plugins((..., ItemPlugin, ...))` in `crates/cdda_app/src/lib.rs`.

## Local Contracts
- `ItemPlugin::build` calls `app.register_type::<T>()` for every public type in `cdda_components::item`. The plugin is the single source of truth for "which item types are reflectable."
- `plugin.rs` groups the registrations in this order:
  - Item state: `DefOrigin`, `StackCount`, `CurrentCharges`, `LoadedAmmo`, `Spoilable`, `ItemDamage`
  - Container tags: `Sealed`, `Rigid`, `Watertight`, `PreservesTemp`, `Fireproof`, `GasTight`
  - Relationship pairs: `InsideContainer`↔`ContainerContents`, `WieldedBy`↔`WieldedItems`, `WornOn`↔`WornBy`, `MountedOn`↔`MountedPockets`
  - Pocket system: `Pocket`, `PocketType`, `PocketRestriction`, `AttachmentSlot`, `AttachmentType`, `Container`, `IsPocket`
  - Inventory + crafting: `Invlet`, `ItemQualities`, `InProgressCraft`
- Relationship pairs must be registered together — both the `#[relationship]` and the `#[relationship_target]` side.
- Item game logic (merge/stack, container insert, wield/wear) lives in `cdda_inventory` and `cdda_equipment`, not here.
- Layer 3 boundary: must not depend on render, input, or app-shell crates (see `crates/AGENTS.md`).

## Work Guidance
- New item component → add it to `cdda_components/src/item.rs` first, then add the matching `register_type::<T>()` line in `src/plugin.rs` under the correct group.
- New `Plugin` code belongs here only if it is reflection or setup. Game systems belong in `cdda_inventory` / `cdda_equipment` / `cdda_crafting`.
- Keep the registration list in the same order as the groups in `Local Contracts`; do not reorder without updating this doc.

## Verification
- `cargo check -p cdda_item` for compile sanity. The `bevy_ecs.workspace = true` entry in `Cargo.toml` is not used in source and can be dropped.
- `cargo nextest run -p cdda_item` (or `cargo test -p cdda_item` if `nextest` is unavailable). No crate-local tests exist yet.
- `cargo check -p cdda_app` to confirm the plugin still wires up end-to-end.

## Child DOX Index

- `src/plugin.rs` — `ItemPlugin` implementation: `app.register_type::<T>()` calls grouped by category, matching the contract above.
