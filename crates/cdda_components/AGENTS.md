# cdda_components DOX

## Purpose
Owns shared ECS data contracts, schedule labels, messages/events, interned tokens and input/context vocabulary. Gameplay operations live in `cdda_sim`; components do not enforce all gameplay invariants by themselves.

## Ownership
- Dependencies: `cdda_core_types`, Bevy ECS/state/reflect, serde/schemars, fixedbitset and leafwing input vocabulary.
- `lib.rs` declares the module index and re-exports core value types, DefId, WorldPos and token IDs.

## Local Contracts
- **Simulation schedule:** `SimulationTurn` (world phases: TurnTick → Activity → … → SpatialUpdate) and `SimulationAction` (IntentDeclare → IntentResolve for one selected actor) are the logical schedules; `cdda_sim::runtime::SimulationPlugin` owns both chains, the `ActingEntity` selection resource, and outer `Update` ordering (GameSet Input → Sim driver → Render). SimSet labels on Update do not imply logical-turn execution.
- **Time:** `GameTime.turn` counts one-second game turns, matching parsed definition Time. `TURNS_PER_HOUR=3600`, `TURNS_PER_DAY=86400`. Wall-clock pacing belongs to the simulation adapter, not this value.
- **Request/result:** ActionIntent declares work, ActionRequestId correlates it, ActionOutcome reports Completed/Rejected/Failed/Cancelled only after resolution. Submission is not completion. Unsupported actions must never report Completed. Outcome persists until replaced; consumers match request IDs.
- **Relationships:** entity links use immutable Bevy relationship pairs. Reinsert using Commands or synchronous World access inside authoritative commits; never mutate relationship fields in place. Reverse-link maintenance does not enforce capacity, exclusive location, cycle safety or resource conservation.
- Existing tags include IsAlive, Visible, Active, Solid, IsDef, IsPocket, body-part capabilities, status markers and container capability flags. Prefer meaningful query/lifecycle boundaries; a parallel marker is unnecessary when a data component already implies identity.
- Skills, mutations, effects, bionics, morale bonuses, body parts and pockets currently use child entities with linked ownership. Preserve their lifecycle semantics when transferring/despawning parents.
- **Definitions:** IsDef entities live in the main Bevy World; DefinitionWorld is an index resource, not a separate ECS World. Runtime queries sharing definition component types must filter Without<IsDef>. Untyped definition index and hot-reload reference migration are tracked data-layer debt.
- **Messages/events:** DamageEvent/DeathEvent/EquipEvent/UnequipEvent/UseItemEvent use observers; ItemMoveEvent/SoundEvent/SightEvent/SpawnEvent/DefChangedEvent and TurnAdvanced use buffered messages. Explicit simulation phases/transactions own ordering; notifications do not replace validation.
- **Input/context:** production adapters use InputAction/GameAction and BindableAction from cdda_input, not raw keyboard queries in gameplay systems. Ctx transitions use push_ctx/pop_ctx to synchronize ContextStack and focus. The dev app's raw movement adapter is a remaining exception.
- **Invariants:** StackCount::new(0) is an error; zero-count entities must be despawned. WorldPosition has new/get/set (public .0 remains legacy). ActionPoints may enter debt; tick clamps its debt floor to -(speed*2).max(50). Default speed 100. Stats::new clamps to 1..20 (default 8); effective stats may reach 0. MAX_SKILL=10. Invlets use 62 alphanumeric chars; FLOOR_CAP_ML=400000.

## Work Guidance
- Shared data belongs here; per-system execution state belongs in its owning module.
- Share read models/components across domains, but route invariant-sensitive mutations through authoritative operations rather than reimplementing validation in each consumer. Keep dependencies acyclic.
- Use token newtypes for runtime IDs; persistent identities must not be confused with load-local indices or Entity values.
- Assess entity/tag/collection decomposition by lifecycle and query needs; do not assume tags prove disjoint mutable queries.

## Verification
- `cargo check -p cdda_components` and `cargo nextest run -p cdda_components` (cargo test fallback if nextest unavailable).
- Schedule/time changes also require `cargo nextest run -p cdda_sim --test simulation_schedule_test --test calendar_test`.
- Request/relationship changes require simulation transaction and inventory integration tests.

## Child DOX Index
Flat `src/` modules (no nested child DOX):
- `actor`, `stats` — creature/player/NPC identity, stats/AP/health, status tags and skill/mutation/proficiency/bionic/morale/effect/body-part relationships.
- `activity` — progress/phase/type, Crafting/Aiming/Reading/Waiting/Reloading/Interacting and exertion tracking.
- `ai` — planner markers and shared goals; HTN executor state is local to cdda_sim.
- `item` — counts/charges/damage/spoilage, origins, pockets/containers and InsideContainer/WieldedBy/WornOn/MountedOn relationship pairs, invlets, InProgressCraft.
- `def`, `def_markers`, `recipe` — definition projection components, typed DefId markers and recipe entity index.
- `intent` — ActionIntent (Move/MeleeAttack/Pickup/Wield/Drop/Stow/UseItem/Reload/StartCraft/StartRead/Wait/Interact), IntentQueue, ActionRequestCounter/Id and ActionOutcome; `collect_intents` honors the optional `ActingEntity` selection.
- `schedule`, `sim`, `messages`, `events` — schedule labels, positions/physics markers, GameTime, messages/events.
- `input`, `context`, `dev` — input vocabulary, Ctx navigation state and dev viewport markers/resources.
- `tokens` — ItemTypeId, SkillId, AmmoTypeId, BodyPartId, ComestibleId.
