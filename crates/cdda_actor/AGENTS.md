# cdda_actor DOX

## Purpose
Layer 3 game-logic crate that owns per-creature simulation: turn scheduling (`ActionPoints`/`TurnQueue`), movement, bionics, status effects, natural healing, body temperature, spoilage, morale, and vision (line-of-sight / sight events). All actor state machines run during the `GameSet::Sim` phases defined in `cdda_components::schedule::SimSet`.

## Ownership
- Dependencies: `bevy_ecs`, `bevy_reflect`, `bevy_app` (Bevy 0.18, workspace-pinned), `cdda_core_types`, `cdda_components`, `tracing`. Dev-dep: `cdda_sim` (for `TestBed`).
- Components/types live in `cdda_components::actor` (`ActionPoints`, `IsAlive`, `Creature`, `Health`, `Stats`, `Vision`, bionic/morale/effect/bodypart relationships, etc.). This crate only owns the systems and pure functions that mutate them.
- A pure reflection-only Bevy `Plugin` (`ActorPlugin`) is exported from `plugin.rs` and registered by `cdda_app` alongside the system wiring.

## Local Contracts
- Bevy relationships are the canonical way to attach per-actor sub-state: `BionicOf`/`InstalledBionics`, `MoraleBonusOf`/`MoraleBonuses`, `EffectOn`/`ActiveEffects`, `BodyPartOf`/`CreatureBodyParts`, `SkillOf`/`CreatureSkills`, `MutationOf`/`CreatureMutations`, `ProficiencyOf`/`CreatureProficiencies`. Mutate by reinserting via `commands.insert()`, never `&mut` query access.
- Status markers are tag components: `IsAlive`, `Stunned`, `Bleeding`, `OnFire`. Filter via `With<...>`, not bool fields.
- Turn scheduling constants live in `turn.rs` and are public: `MOVE_COST_WALK=100`, `MOVE_COST_RUN=80`, `MOVE_COST_CROUCH=200`, `MOVE_COST_PRONE=600`, `MOVE_COST_DOWNED_MULTIPLIER=3`, `MOVE_COST_ATTACK_BASE=100`, `AP_COST_PICKUP=100`, `AP_COST_WIELD=100`, `AP_COST_CRAFT_TICK=100`, `MOVE_COST_RELOAD_BASE=100`, `MP_MIN_FLOOR=25`. `ActionPoints::tick()` clamps `current` to a debt floor of `-(speed*2).max(50)`.
- `ActorPlugin` registers reflect types only — it does **not** wire systems into any `SystemSet`. Schedule wiring lives in the binary crate `cdda_app::CddaPlugin` (`crates/cdda_app/src/lib.rs`): `tick_move_points` → `SimSet::TurnTick` (throttled 100ms), `effects_phase` → `SimSet::Effects`, `healing_phase` → `SimSet::Healing`, `tick_bionics` → `SimSet::Bionics`, `tick_morale_decay` → `SimSet::Morale`, `temperature_phase` → `SimSet::Temperature`, `update_vision` → `SimSet::Vision`, `movement_phase` → `SimSet::Movement`, `debug_turn_queue` → `SimSet::SpatialUpdate`. All gated on `AppState::InGame`.
- `tick_move_points` writes a `TurnAdvanced` message (from `cdda_components::messages`) and advances `GameTime.turn` by 1.

## Work Guidance
- Implemented: `turn.rs` (`tick_move_points` system, `spend_move_points`, `effective_move_cost`, `TurnQueue` with `pop_highest`/`has_actors_ready`/`highest_mp`, `ActorTurn`, `debug_turn_queue`); `effects.rs` (`apply_effect`/`remove_effect`/`has_effect`/`get_effect_intensity`/`tick_effects`/`effects_phase` — all functional and exercise the `EffectOn`/`ActiveEffects` hooks); `plugin.rs` (register_type calls for ~36 types).
- Stubs (`todo!()`): `bionics.rs` (`activate_bionic`, `deactivate_bionic`, `total_power`, `tick_bionics`), `morale.rs` (`add_morale_bonus`, `calculate_morale`, `apply_morale_effects`; `tick_morale_decay` and `temperature_phase` are no-op shells), `healing.rs` (all of it), `temperature.rs` (all of it), `vision.rs` (all of it), `movement.rs` (`calculate_move_cost`, `attempt_move`, `spend_move_points`, `is_passable`; `movement_phase` is a no-op). Do not treat stub signatures as final — many still take `&mut World` and will likely move to `&mut Query<...>` once the world is ready.
- A relationship target type with `#[relationship_target(relationship = ..., linked_spawn)]` despawn-cascades its members; `bionics_test.rs::bionic_removed` exercises this and expects the creature to survive (no `linked_spawn` on the parent).
- Keep AP cost constants in sync with `cdda_app::startup` (which imports `AP_COST_WIELD` directly) — never duplicate the magic number 100 in callers.

## Verification
- `cargo check -p cdda_actor` for compile sanity.
- `cargo nextest run -p cdda_actor` (fall back to `cargo test -p cdda_actor` if `nextest` is unavailable). 16 integration test files under `tests/`: `actor_tests`, `ap_system_test`, `bionics_system_test`, `bionics_test`, `effects_system_test`, `healing_system_test`, `morale_system_test`, `morale_test`, `movement_system_test`, `movement_test`, `status_effect_test`, `temperature_system_test`, `temperature_test`, `turn_system_test`, `vision_system_test`, `vision_test`. Most `*_system_test.rs` tests are `#[ignore = "… system not yet implemented"]` and will be un-ignored as the matching `todo!()` is filled in.

## Child DOX Index
- `crates/cdda_actor/src/plugin.rs` — `ActorPlugin` Bevy plugin; `register_type` for ~36 actor/bionic/morale/effect/bodypart types. No schedule wiring.
- `crates/cdda_actor/src/turn.rs` — AP scheduling system + constants + `TurnQueue` resource.
- `crates/cdda_actor/src/movement.rs` — `MoveResult`/`MoveBlockReason` enums + `calculate_move_cost`/`attempt_move`/`is_passable`/`movement_phase` (mostly `todo!()`).
- `crates/cdda_actor/src/bionics.rs` — `activate_bionic`/`deactivate_bionic`/`total_power`/`tick_bionics` (all `todo!()`).
- `crates/cdda_actor/src/effects.rs` — `apply_effect`/`remove_effect`/`has_effect`/`get_effect_intensity`/`tick_effects`/`effects_phase` (real, uses `EffectOn`/`ActiveEffects`).
- `crates/cdda_actor/src/healing.rs` — `healing_phase`/`calculate_healing_rate`/`apply_first_aid` (`todo!()`).
- `crates/cdda_actor/src/morale.rs` — `add_morale_bonus`/`calculate_morale`/`tick_morale_decay`/`apply_morale_effects` (`todo!()`; `tick_morale_decay` is a no-op shell).
- `crates/cdda_actor/src/temperature.rs` — `update_body_temperature`/`calculate_total_warmth`/`calculate_insulation`/`spoilage_rate`/`tick_spoilage`/`tick_temperature`/`temperature_phase` (`todo!()`).
- `crates/cdda_actor/src/vision.rs` — `update_vision`/`calculate_vision_range`/`can_see`/`visible_entities` (`todo!()`).
