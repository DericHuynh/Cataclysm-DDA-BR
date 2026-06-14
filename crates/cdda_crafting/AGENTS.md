# cdda_crafting DOX

## Purpose
Layer 3 crafting crate. Owns recipe validation, component consumption, AP-driven in-progress craft execution, and the crafting menu UI state (`CraftState`, `CategoryIndex`, `PendingCraft`).

## Ownership
- Recipe def components (`RecipeResult`, `RecipeComponents`, `RecipeQualities`, `RecipeTime`, `RecipeCategory`, `RecipeSubcategory`, `RecipeResultCount`, `RecipeSkillUsed`, `RecipeDifficulty`) and the `RecipeIndex` resource live in `cdda_components` / `cdda_data`. This crate only consumes them.
- Item defs and stacks live in `cdda_data` / `cdda_inventory`; this crate reads them via `DefinitionWorld`, `ItemTypeRegistry`, `MountedPockets`, `ContainerContents`, `WieldedItems`, `WorldPosition`, `ItemQualities`, `StackCount`.
- Player activity scheduling and the per-turn AP tick live in `cdda_activity`; this crate only attaches a `PlayerActivity` with `ActivityActor::Craft(CraftActor { .. })`.
- Crate has no `tests/` directory; coverage lives in the workspace `tests/` suite (see Verification).

## Local Contracts
- **AP formula** (`systems.rs::start_craft`): `ap_total = (RecipeTime.0 as i32 * 100).max(100)`. `RecipeTime` is in turns at the speed=100 baseline; `.max(100)` enforces a one-tick minimum. `continue_crafts` spends `AP_COST_CRAFT_TICK` per turn via `cdda_activity::systems::tick_one`.
- **`CRAFT_COMPLETE_HOOK` seam** (`cdda_activity::CRAFT_COMPLETE_HOOK: OnceLock<CompleteCraftFn>`): `CraftingPlugin::build` sets it to `complete_craft` so `cdda_activity::CraftActor::finish` can finalize crafts without importing `cdda_crafting` (avoids a Layer 2↔3 cycle). Tests must set this hook themselves.
- **`craft:in_progress:{result_id}` interning** (`systems.rs::start_craft`): the spawned `InProgressCraft` entity receives `ItemType(item_type_registry.intern(&format!("craft:in_progress:{result_id}")))`, giving the in-progress craft a unique, queryable item type while preserving the original result id.
- **Availability model** (`collect_available_items`): union of `MountedPockets` body pockets, `WieldedItems`, direct `ContainerContents` on player (back-compat), and any entity in the same OMT tile as the player.
- **Component consumption** (`consume_items`): decrements `StackCount`; despawns on zero. Despawning auto-removes `InsideContainer` / `ContainerContents` / `Invlet` relationships.
- **Craft completion** (`complete_craft`): despawns the `InProgressCraft` entity and spawns the result item into the first body pocket (or player fallback) via the `spawn_item_from_def` stub (TODO: move to `cdda_overmap_gen::spawning`).
- Menu runs only when `bevy_state::in_state(Ctx::CraftingMenu)`; `process_pending_craft` is scheduled in `SimSet::Activity`; `build_craft_state` runs on `OnEnter(Ctx::CraftingMenu)` and after each craft.

## Work Guidance
- Keep recipe lookup (read of `RecipeIndex` + `DefinitionWorld`) and activity lifecycle (mutate `PlayerActivity`, `InProgressCraft::ap_spent`) in their existing functions; do not collapse them.
- `do_craft` is a legacy no-AP helper kept for tests and dev commands — prefer `start_craft` for gameplay paths.
- `on_examine_item_changed` must remain idempotent (gated on `ExaminedItem::is_changed`) and only push the `BindableAction::HotkeyR` "resume craft" action.
- `build_craft_state` preserves `show_all`, `last_message`, `filter`, `filtering` across rebuilds; do not reset these.
- Display helpers (`display_category`, `display_subcategory`) strip `CC_` / `CSC_` and the category short-name prefix — keep them in sync with upstream CDDA enum naming.

## Verification
- `cargo check -p cdda_crafting` for compile sanity.
- `cargo nextest run -p cdda_crafting` falls back to `cargo test -p cdda_crafting` when `nextest` is unavailable (per root AGENTS.md).
- Workspace coverage: `tests/crafting_time_test.rs` (imports `cdda_crafting::systems::{continue_crafts, start_craft}` and sets `CRAFT_COMPLETE_HOOK` to `cdda_crafting::systems::complete_craft`); `tests/recipe_test.rs` exercises recipe def components downstream.

## Child DOX Index
- `src/lib.rs` — crate root; declares `pub mod input`, `pub mod plugin`, `pub mod systems`.
- `src/plugin.rs` — `CraftingPlugin`: registers `CRAFT_COMPLETE_HOOK`, initializes `CraftState` / `PendingCraft` / `RecipeIndex` / `CategoryIndex` resources, schedules `OnEnter(Ctx::CraftingMenu) → build_craft_state`, and gates `crafting_menu_input` + `process_pending_craft` on `in_state(Ctx::CraftingMenu)`.
- `src/input.rs` — `crafting_menu_input` (per-frame keyboard + `InputAction` reader covering `Filter` / `Navigate*` / `Confirm` / `HotkeyPress('a')` / `Cancel`) and `process_pending_craft` (exclusive world-mutating drain of `PendingCraft` → `start_craft` → `build_craft_state`).
- `src/systems.rs` — domain logic: `CategoryIndex`, `CraftEntry`, `CraftState` (+ `visible` / `visible_count` / `focused_entry`), `PendingCraft`, `display_category` / `display_subcategory`, `collect_available_items`, `count_available`, `has_quality`, `check_can_craft`, `consume_items`, `find_dev_player`, `start_craft`, `complete_craft`, `resume_craft`, `on_examine_item_changed`, `continue_crafts`, `do_craft`, `build_craft_state`, `slot_has_alternatives`, and the `spawn_item_from_def` stub.
