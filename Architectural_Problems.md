# Architectural Problems

Problems identified from a Bevy ECS perspective. Ordered by impact.

---

## 1. Exclusive World Access Kills Parallelism

**Files:** `cdda_sim/src/systems/movement.rs`, `combat.rs`, `effects.rs`, etc.  
**All simulation phase functions take `world: &mut World`.**

Bevy's scheduler can run independent systems in parallel across threads. Exclusive world access (`&mut World`) opts every system out of this — the entire sim set becomes single-threaded and serialised. The correct pattern is fine-grained `Query<&mut Component>` parameters, which tell Bevy exactly what each system reads and writes, enabling parallelism and static conflict detection.

```rust
// Current — locks everything
pub fn movement_phase(world: &mut World) { ... }

// Correct — Bevy can schedule this alongside non-conflicting systems
pub fn movement_phase(
    mut movers: Query<(Entity, &mut WorldPosition, &mut MovePoints), With<MoveIntent>>,
    terrain: Res<WorldMap>,
    spatial: ResMut<EntitySpatialIndex>,
) { ... }
```

---

## 2. Dead `game_tick` Hides Missing Phases

**File:** `cdda_sim/src/systems/turn.rs` lines 200–213.

`game_tick` is a free function that calls the full phase pipeline (movement → combat → effects → healing → morale → bionics → temperature → spoilage → vision → spawning). It is **never registered as a Bevy system**. The app only registers a subset of those phases individually. Result:

- `healing_phase`, `morale::tick_morale_decay`, `tick_bionics`, `tick_temperature`, `tick_spoilage`, `update_vision` are **never called**.
- `game_tick` is dead code that will silently double-execute everything if someone mistakenly registers it.

Either delete `game_tick` and register each phase individually in `cdda_app`, or register `game_tick` as the sole exclusive system and remove the individual registrations. Do not maintain both.

---

## 3. IsDef Not Filtered from Gameplay Queries

**Files:** `cdda_sim/src/systems/turn.rs`, `movement.rs`, `combat.rs`, etc.

Definition entities are spawned with `IsDef` during `DataLoading` and remain in the World. Gameplay systems query by component presence (`With<IsAlive>`, `With<Creature>`) but add no `Without<IsDef>`. Any definition entity that happens to have those components will be included in gameplay system queries — turn ticks, movement, damage.

`CURRENT_ARCHITECTURE.md` describes `DefaultQueryFilters` with `Without<IsDef>` being configured globally, but this is **not implemented anywhere**. Until it is, every gameplay `Query` must manually add `Without<IsDef>` as a filter.

---

## 4. Spatial Index Uses `/` Instead of `div_euclid`

**File:** `cdda_sim/src/spatial.rs` lines 44–49.

```rust
fn cell(pos: WorldPos) -> (i32, i32, i32) {
    (
        pos.x / CELL_SIZE,   // wrong for negative coords
        pos.y / CELL_SIZE,
        ...
    )
}
```

Integer division on signed values truncates toward zero. `-5 / 16 == 0`, but the correct cell is `-1`. Entities at negative coordinates get placed in the wrong spatial cell, causing missed lookups and phantom query results. `CURRENT_ARCHITECTURE.md` explicitly mandates `div_euclid` for all horizontal coordinate arithmetic.

Fix: `pos.x.div_euclid(CELL_SIZE)`.

---

## 5. Dual Parallel State Machines

**Files:** `cdda_sim/src/state.rs` (`AppState`), `cdda_ui/src/screen.rs` (`Screen`).

The app runs two disconnected state machines:

- `AppState`: `MainMenu → DataLoading → WorldGen → InGame` (sim lifecycle)
- `Screen`: `MainMenu → DevWorldgen → Gameplay` (render/UI lifecycle)

These are never explicitly synchronised. `AppState::InGame` must correspond to `Screen::Gameplay`, but nothing enforces that mapping. If a transition fires in one and not the other, the sim runs without a renderer, or the renderer runs with no world.

Consolidate into a single state type, or add an explicit transition system that drives `Screen` from `AppState` changes.

---

## 6. TurnQueue Rebuilt Every Bevy Frame

**File:** `cdda_sim/src/systems/turn.rs`, `tick_move_points`.

`tick_move_points` clears and rebuilds `TurnQueue` every time Bevy calls it, which is every frame. At 60 fps this means 60 game turns per second regardless of what the game is doing. Game speed is coupled to frame rate.

A roguelike's discrete turn loop should not be driven by the render loop. The sim should either:
- Run one full turn per frame and gate rendering on turn completion, or
- Use a fixed timestep / accumulator so turns advance independently of frame rate.

---

## 7. Relationship Components Are Not Immutable

**Files:** `cdda_item/src/components.rs`, `cdda_actor/src/components.rs`.

Bevy 0.14+ relationships (`#[relationship]`) maintain two-way consistency via hooks that fire on insert/remove. Mutating the relationship field through `&mut` bypasses those hooks, leaving the inverse side stale. `WornOn`, `InsideContainer`, `WieldedBy`, `BionicOf`, etc. can all be silently corrupted this way.

Fix: add `#[component(immutable)]` to every relationship component. This makes Bevy reject `&mut RelComponent` at compile time, forcing reinsertion.

```rust
#[derive(Component)]
#[component(immutable)]          // ← add this
#[relationship(relationship_target = WornBy)]
pub struct WornOn { ... }
```

---

## 8. No `Reflect` on Any Component

**Files:** every `components.rs` file.

Zero components derive `Reflect`. This blocks:

- **`bevy-inspector-egui`** — the `WorldInspectorPlugin` already added in `cdda_app` shows nothing.
- **Save/load** — Bevy's reflect-based serialization (`ReflectSerialize`/`ReflectDeserialize`) cannot operate on unregistered types.
- **Scene hot-reload** and dynamic patching.

All `#[derive(Component)]` items should also derive `Reflect`, and `world_setup.rs` should call `app.register_type::<T>()` for each.

Note: the existing `world.register_component::<T>()` calls in `world_setup.rs` register storage layout — that is **not** the same as `app.register_type::<T>()` for reflection.

---

## 9. `GameSet` Is Too Coarse for Safe Ordering

**File:** `cdda_core/src/schedule.rs`, `cdda_app/src/lib.rs`.

`GameSet` has three variants: `Input`, `Sim`, `Render`. All ten simulation phases (`tick_move_points`, `ai_phase`, `movement_phase`, …) land in `GameSet::Sim`. Their order is enforced today only through ad-hoc `.after()` chains on each registration call. Add one new system without the right `.after()` and its ordering relative to every existing system is undefined.

The simulation has a well-known total order. Express it as sets:

```rust
#[derive(SystemSet, ...)]
pub enum SimSet {
    TurnTick, AI, Movement, Combat, Effects, Healing,
    Bionics, Morale, Spawning, SpatialUpdate,
}
app.configure_sets(Update, (
    SimSet::TurnTick, SimSet::AI, SimSet::Movement,
    SimSet::Combat, SimSet::Effects, SimSet::Healing,
    SimSet::Bionics, SimSet::Morale, SimSet::Spawning,
    SimSet::SpatialUpdate,
).chain().in_set(GameSet::Sim));
```

New systems then declare membership (`in_set(SimSet::Combat)`) rather than chaining off specific functions.

---

## 10. `components/mod.rs` Re-Exports Hide Ownership

**File:** `cdda_sim/src/components/mod.rs`.

```rust
pub use cdda_actor::components::{Health, Stats, CombatStats, ...};
pub use cdda_item::components::{StackCount, CurrentCharges, ...};
```

Systems import from `cdda_sim::components::*` and lose track of which crate owns each type. This makes it hard to audit what each crate provides, introduces implicit coupling, and breaks if components move between crates. Systems should import directly from their owning crates (`cdda_actor::components`, `cdda_item::components`).

---

## 11. Vec / HashMap in Components Causes Coarse Change Detection

**Files:** `cdda_actor/src/components.rs`, `cdda_item/src/components.rs`.

```rust
pub struct SkillSet    { pub skills: HashMap<SkillId, SkillLevel> }
pub struct Mutations   { pub active: Vec<MutationState> }
pub struct ProficiencySet { pub proficiencies: Vec<...> }
```

Modifying one skill marks the entire `SkillSet` as `Changed`, triggering every system that reacts to `Changed<SkillSet>`. Skills, mutations, and proficiencies are independent sub-entities by nature — they should each be a separate entity with a relationship back to the actor (`SkillOf`, `MutationOf`). This gives per-item change detection and queries like `Query<&Skill, With<Changed<Skill>>>`.

---

## 12. `run_if` Closures Instead of `in_state()`

**File:** `cdda_app/src/lib.rs` lines 59–70.

```rust
fn in_ingame(state: Res<State<AppState>>) -> bool {
    *state.get() == AppState::InGame
}
// ...
tick_move_points.run_if(in_ingame)
```

Bevy provides `run_if(in_state(AppState::InGame))` for exactly this. The hand-rolled closures bypass Bevy's state-change optimisation (which caches and skips the condition check when the state hasn't changed) and add noise. They also don't compose with `OnEnter`/`OnExit` scheduling.

---

## Summary

| # | Problem | Impact |
|---|---------|--------|
| 1 | Exclusive world access in all sim phases | No parallelism; all sim serialised |
| 2 | `game_tick` dead code hides unregistered phases | Healing, morale, bionics, vision never run |
| 3 | `IsDef` not filtered in gameplay queries | Defs silently participate in sim |
| 4 | Spatial index uses `/` not `div_euclid` | Wrong cells for negative coordinates |
| 5 | Two disconnected state machines | Sim/render can desync |
| 6 | TurnQueue rebuilt every frame | Turn speed tied to frame rate |
| 7 | Relationship components not immutable | Silent two-way consistency bugs |
| 8 | No `Reflect` on components | Inspector blank; save/load impossible |
| 9 | `GameSet::Sim` too coarse | System ordering fragile under extension |
| 10 | Re-exports in `components/mod.rs` | Ownership obscured |
| 11 | Vec/HashMap collections in components | Coarse change detection |
| 12 | Manual `run_if` closures for state | Misses Bevy caching; noise |
