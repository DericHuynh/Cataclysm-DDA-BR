# cdda_ai DOX

## Purpose
Layer 3 game-logic crate. Decides what action an AI-controlled entity takes per tick and translates that decision into movement or combat. The crate is currently a stub — no decision logic or pathfinding is implemented yet; all public functions are `todo!()` or no-ops.

## Ownership
- Lives at `crates/cdda_ai/`. `Cargo.toml` dependencies are `bevy_ecs.workspace`, `bevy_app.workspace`, and `cdda_core_types = { path = "../cdda_core_types" }` — nothing else.
- `AiPlugin` is defined here but is not registered anywhere in the workspace. `cdda_app` calls the free function `cdda_ai::systems::ai_phase` directly and places it in `SimSet::Ai` (see `crates/cdda_app/src/lib.rs`).
- Sibling Layer 3 crates (`cdda_actor`, `cdda_combat`, `cdda_noise`, etc.) supply the components and sensory inputs the AI reads; this crate must not depend on them. Pathfinding, if/when added, must not reach up to `cdda_overmap` (Layer 4) or any Layer 5 app shell crate.

## Local Contracts
- `src/lib.rs` exports two modules only: `pub mod plugin;` and `pub mod systems;`.
- `systems::AiGoal` is a `#[derive(Debug, Clone, Copy, PartialEq, Eq)]` enum with exactly five variants: `Attack { target: Entity }`, `Wander`, `Flee { from: Entity }`, `Guard { position: WorldPos }`, `Hunt { target: Entity }`.
- AI evaluation is split into two free functions in `systems.rs`:
  - `decide_action(world: &World, entity: Entity) -> AiGoal` — read-only; must not mutate world state.
  - `execute_ai_action(world: &mut World, entity: Entity, goal: AiGoal)` — applies the chosen goal by calling `attempt_move`, `resolve_melee_attack`, etc.
- `ai_phase(world: &mut World)` is the per-tick entry point wired into `SimSet::Ai` by `cdda_app`. All three functions are currently stubs.
- Sensory inputs `SoundEvent` and `SightEvent` are buffered `Message` types defined in `cdda_components::events` (file `crates/cdda_components/src/events.rs`), not in `cdda_components::messages`.
- The `AiPlugin::build` body is empty (`fn build(&self, _app: &mut App) {}`); registration is intentionally the caller's job.

## Work Guidance
- Keep `decide_action` pure so the decision half can be unit-tested without a `bevy_app::App`; put all world mutation in `execute_ai_action`.
- If `AiPlugin::build` ever gains real work, mirror the `SimSet::Ai` placement currently used in `cdda_app/src/lib.rs` to keep wiring consistent.
- Doc-comments in `systems.rs` reference `EntitySpatialIndex`, `Health`, `CombatStats`, `MonsterStats`, `Vision`, `Faction`, `NpcPersonality`, and `MonsterFlags`. None are imported yet — treat the doc list as a target contract, not an existing dependency.

## Verification
- `cargo check -p cdda_ai`
- `cargo nextest run -p cdda_ai` (fall back to `cargo test -p cdda_ai` if `nextest` is unavailable)

## Child DOX Index
- `src/plugin.rs` — `AiPlugin` struct and its (currently empty) `Plugin::build` impl.
- `src/systems.rs` — `AiGoal` enum, `decide_action`, `execute_ai_action`, `ai_phase`.

There is no `tests/` directory under this crate.
