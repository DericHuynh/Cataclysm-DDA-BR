# cdda_sim

## Purpose
Owns gameplay systems and the canonical headless simulation runtime. The graphical app, headless scenarios, and future replay driver must use the same schedule rather than reproducing its wiring.

## Ownership
- `runtime/` owns the simulation driver, clock adapter, lifecycle types and test helpers.
- Gameplay systems live under `actor`, `ai`, `intent`, `activity`, `combat`, `crafting`, `equipment`, `inventory`, `item`, and `noise`.
- Shared domain components live in `cdda_components`; local execution state may live here.
- Dependencies include `cdda_components`, `cdda_core_types`, `cdda_data`, `cdda_defs_raw`, and the workspace-internal `cdda_htn` planner core. No renderer dependency.

## Local Contracts
- **Canonical wiring:** `runtime::SimulationPlugin` installs gameplay plugins/resources and the persistent `SimulationTurn` schedule. It orders `GameSet::Input → Sim → Render` in outer `Update`; only `drive_simulation` runs in the Sim slot. Register authoritative systems in `SimulationTurn`, not `Update`. World adapters can extend its `SimSet::SpatialUpdate` phase.
- **Time:** one logical turn = one game second, matching definition `Time`. `GameTime` is in `cdda_components::sim`. `SimClock` is ONLY the optional real-time wall accumulator, default 100 ms pacing; it is fail-closed, rejects a zero step, retains bounded catch-up backlog, and clears wall debt while paused.
- **Driver:** `SimulationControl` defaults to `TurnBased` (wait for declared living-actor actions, pending craft/item moves, or ongoing activities). `Manual` advances only via queued `request_steps` or `step_simulation`; `RealTime` consumes wall time. `max_steps_per_update` bounds work without dropping backlog. Explicit requested steps survive pause. `step_simulation(&mut World)` returns false if paused or an installed `AppState` is not `InGame`; headless apps may omit AppState. A raw `world.run_schedule(SimulationTurn)` bypasses the driver gate and is not the supported stepping API.
- **Phase order:** TurnTick → Activity → Effects → Healing → Bionics → Morale → Temperature → Vision → Spawning → Inventory → SpatialUpdate inside `SimulationTurn`. IntentDeclare → IntentResolve run inside `SimulationAction`, which the budget scheduler runs repeatedly per world turn.
- **AP-budget action loop:** after world phases, `step_simulation` repeatedly selects the highest-AP living actor (SimId/Entity stable tie-break, actors with `ActivityProgress` excluded) and runs `SimulationAction` for it (AI declare → collect → resolve). A committed action re-queues the actor while AP > 0, so fast actors act multiple times per world turn; a pass without a committed action parks the actor until the next turn. One declared intent is one action — leftover player budget banks for later turns. Rejected/planless actors cannot loop within a turn (bounded at 64 selections).
- **Intent commit:** `resolve_intents` is an exclusive sequential world commit: validate live state, commit mutations/AP, then publish matching terminal outcome. Subsequent requests see committed positions, ownership and AP. Equal AP orders by SimId ascending (identified entities first), with Entity bits fallback for untagged fixtures/duplicate IDs. Request IDs are allocated in sorted order; replay requires unique SimIds.
- **Action validation:** `inventory::transfer::apply_inventory_action` is the shared transactional boundary for item actions. Move requires an existing position, nonzero one-tile offset and a non-overflowing, unoccupied destination (ECS `Solid` entities; local terrain not yet represented). Pickup/Wield/Drop/Stow validate live exclusive location, ownership-chain/cycle safety, same-z Chebyshev ≤ 1 reach for ground items, HandCount for wielding (missing hands = none), and the exact-tile floor cap for drops; each charges 100 AP once after validation. Move/Wait/Pickup/Wield/Drop/Stow complete; MeleeAttack/UseItem/Reload/StartRead/Interact/StartCraft remain unsupported on the intent path (Failed, no AP). Rejected/Failed charge no AP. A despawned actor has no outcome component, but its rejection is counted. Pocket capacity/weight/restrictions and legacy `ItemMoveEvent`/merge consolidation remain deferred.
- **Submission is not completion:** terminal `ActionOutcome` persists and consumers correlate request IDs. HTN never commits predicted effects; the simulation is authoritative. Planner costs are estimates only.
- **UI boundary:** do not match `GameAction`/read `InputAction` in simulation systems. Screen adapters live in `cdda_render::render::input` and now declare `ActionIntent`s only for wield/stow/pickup/drop (no AP/relationship/`ItemMoveEvent` bypass). Remaining legacy bypasses: dev spawn, pending-craft routing, and the `ItemMoveEvent`/merge consumer path.
- `state` and `test_utils` lib-root aliases are deprecated; use `runtime::*`.

## Work Guidance
- New behavior goes in its owning subsystem; register it through SimulationPlugin or the subsystem plugin on the canonical schedule.
- Entity relationships maintain reverse links, not gameplay invariants. Use explicit validating operations for transfers, merges, equipment and costs. Reinsert immutable relationships via World or Commands; do not mutate their fields in place.
- Explicit phases/transactions own simulation causality. Events are appropriate for notifications and bounded reactions, not a substitute for action commit ordering.
- Use real persistent App/schedule tests for timing, pause, message cursors, deferred visibility and production wiring. `TestBed::run_system` recreates a system each call and is only an isolated-function helper.

## Verification
- `cargo check -p cdda_sim`.
- `cargo nextest run -p cdda_sim` (fallback cargo test if nextest unavailable).
- `cargo nextest run -p cdda_sim --test simulation_schedule_test --test intent_transaction_test --test htn_integration_test` covers production stepping/frame partition/pause/calendar, sequential commits and the planner integration.
- Workspace integration: `cargo check -p cdda_app` and `cargo nextest run --workspace --exclude cdda_app` (app default dynamic-link test loader is environment-sensitive).
- Cargo discovers top-level `tests/*.rs` only; the migrated nested suites are wired through the `migrated_{actor,combat,inventory}.rs` aggregators (302 restored tests, 83 pre-existing `#[ignore]` stubs). The discovery guard in `migrated_actor.rs` fails if aggregator modules drift.

## Child DOX Index
- `src/runtime/` — `plugin.rs` canonical SimulationPlugin/SimulationControl/SimulationMode/step_simulation; `clock.rs` real-time accumulator; `state.rs` AppState/StartupConfig; `test_utils.rs` isolated TestBed.
- `src/intent/` — stable collection, live sequential validation/commit, correlated outcomes.
- `src/actor/` — AP grant, effects, movement, healing, bionics, morale, temperature and vision (some physiology remains stubbed).
- `src/ai/` — per-marker BT/GOAP placeholder producers and real HTN driver before intent collection.
- `src/ai/htn/` — kernel registry, actor observations, parameterized JSON compound compiler and request/result execution adapter over `cdda_htn`; built domain and ItemCatalog are an immutable resource pair. Startup/reload publication and plan-generation invalidation still need full integration.
- `src/activity/` — per-type activity ticks and cleanup over shared activity components; craft completion messages are handled in the same logical turn.
- `src/crafting/` — recipe/state lookup, pending craft start and completion. Menu index OnEnter registration remains here; no InputAction matching.
- `src/inventory/` — recursive worn/wielded/contents/pocket traversal with dedup, `transfer.rs` transactional Pickup/Wield/Drop/Stow boundary, legacy movement messages, invlets, stacks and bins. Legacy `ItemMoveEvent`/merge consolidation and pocket capacity enforcement remain pending.
- `src/combat/`, `src/equipment/`, `src/item/`, `src/noise.rs` — combat/equipment operations, type registration, noise functions.
- `tests/` — top-level domain tests plus `simulation_schedule_test.rs`, `intent_transaction_test.rs`, `inventory_action_test.rs`, `htn_integration_test.rs`, `inventory_traversal_test.rs`, and the `migrated_{actor,combat,inventory}.rs` discovery aggregators.
