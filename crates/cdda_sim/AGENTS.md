# cdda_sim

## Purpose
The single workspace crate that owns every game-logic subsystem plus the runtime harness (`AppState`, `GameTime`, `TestBed`). Formed by consolidating 9 thin game-logic crates (`cdda_actor`, `cdda_ai`, `cdda_activity`, `cdda_combat`, `cdda_crafting`, `cdda_equipment`, `cdda_inventory`, `cdda_item`, `cdda_noise`) with the original `cdda_sim` runtime.

## Ownership
- Bevy deps: `bevy_ecs`, `bevy_reflect`, `bevy_app`, `bevy_state`, `bevy_input` (with `keyboard`), plus `serde`, `cdda_core_types`, `cdda_components`, `cdda_events`, `cdda_context`, `cdda_data`, `tracing`.
- Public surface is organised as submodules. Consumers reach into `cdda_sim::<area>::…` directly.

## Local Contracts
- **One crate, nine game-logic submodules, one runtime harness.** `cdda_sim::runtime` is the `AppState` + `TestBed` harness. The other nine submodules own one gameplay concern each.
- **Flat re-exports at the lib root** keep two old call sites alive: `cdda_sim::state` → `cdda_sim::runtime::state`, and `cdda_sim::test_utils` → `cdda_sim::runtime::test_utils`. Both are `#[deprecated]` and will be removed in a follow-up.
- All callers reach this crate through the consolidated public surface at `cdda_sim::<area>::…`. There are no deprecation shim crates anymore — the migration is complete.

## Work Guidance
- New code in a game-logic area goes into the matching submodule under `crates/cdda_sim/src/<area>/`. If the area is genuinely new, add a new submodule and declare it in `src/lib.rs`.
- `runtime/` is the only submodule that other crates typically import directly. Everything else goes through the consolidated public surface at `cdda_sim::<area>::…`.
- The two `#[deprecated]` re-exports (`cdda_sim::state` and `cdda_sim::test_utils`) should disappear in a future commit. The compiler will point the author at the right path on each call.

## Verification
- `cargo check -p cdda_sim` for compile sanity.
- `cargo nextest run -p cdda_sim` runs the consolidated test suite (fall back to `cargo test -p cdda_sim` if `nextest` is unavailable). The pre-consolidation tests under `crates/cdda_actor/tests/`, `crates/cdda_combat/tests/`, and `crates/cdda_inventory/tests/` are now under `crates/cdda_sim/tests/{actor,combat,inventory}/`. Other tests live alongside the code they test in the relevant submodule.
- `cargo nextest run --workspace` runs everything: this crate, the data plane, the renderer, and the integration tests (fall back to `cargo test --workspace` if `nextest` is unavailable).
- Cross-crate impact: changes to `TestBed` (in `runtime/test_utils.rs`) ripple to every crate that uses it. The harness API is the most volatile surface in this crate.

## Child DOX Index
- `src/runtime/` — `AppState`, `TurnState`, `GameTime`, `StartupConfig`, `LoadingStatus`, and the `TestBed` test harness. The most-consumed submodule; treat its public API as the workspace's test contract.
- `src/actor/` — Creature turn scheduling, movement, bionics, effects, healing, temperature, morale, vision.
- `src/ai/` — Monster/NPC decision making and the `AiGoal` enum.
- `src/activity/` — Multi-turn player activities (`PlayerActivity`, `ActivityPhase`, `ActivityActor`, `ActivityTracker`) and the `CRAFT_COMPLETE_HOOK` seam.
- `src/combat/` — Damage, hit/miss, melee, ranged.
- `src/crafting/` — Recipe lookup, component consumption, progress.
- `src/equipment/` — Wielding, wearing, encumbrance.
- `src/inventory/` — Stacks, invlets, binned lookups, item movement, the `ExaminedItem` resource, the `InventoryBin` cache.
- `src/item/` — `ItemPlugin` for type registration.
- `src/noise.rs` — 3D simplex noise matching CDDA master.
- `tests/actor/`, `tests/combat/`, `tests/inventory/` — per-submodule test directories (moved from the corresponding old crate's `tests/`). Plus flat test files at the top of `tests/` for the cross-submodule suites (ammo, armor, body part, calendar, food, item damage, monster, recipe, tool, wield/wear).
