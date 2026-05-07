# CDDA Systems Design — Bevy ECS 0.18

## Architecture Overview

The simulation layer (`cdda_sim`) implements a deterministic turn-based loop
using Bevy ECS 0.18. Systems are organised into **phases** that run in a fixed
order each game tick. Each phase is a Bevy system that takes `&mut World` and
reads/writes ECS components, resources, and messages.

### Design principles

1. **Phases are explicit functions**, not Bevy `Schedule`s — the tick loop
   calls `phase(world)` sequentially. This gives us deterministic ordering,
   simple debugging, and avoids Bevy's system-ordering complexity for the
   tightly-coupled simulation core.
2. **Systems communicate through components and messages** (Bevy `Message`).
   Business logic lives in `crate::logic/*` (pure functions, no ECS).
   Systems are thin orchestrators that read ECS state, call logic functions,
   and write results.
3. **No `&mut` access to relationship components** — relationships are mutated
   by reinsertion via `Commands` so that Bevy hooks keep the relationship
   target in sync.

### Crate dependency chain

```
cdda_core  (pure types, units, IDs, Damage, Stats)
    ↑
cdda_data  (JSON loading, DefRegistry)
    ↑
cdda_actor (creature/player/NPC components)
cdda_item  (item/inventory components)
    ↑
cdda_sim   (components, systems, turn loop, spatial index)
    ↑
cdda_map   (world map, submaps, overmap, pathfinding)
cdda_mod   (mod loading, JSON layering)
    ↑
cdda_app   (binary entry point, Bevy app builder)
```

---

## Tick Order and Dependencies

Each game turn executes the following phases **in order**. Every phase receives
`&mut World` and is responsible for its slice of the simulation.

| #  | Phase                    | System functions                                           | Purpose |
|----|--------------------------|------------------------------------------------------------|---------|
| 0  | **Turn start**           | `turn::tick_move_points`                                   | Grant MP to all actors, rebuild `TurnQueue`, emit `TurnAdvanced` |
| 1  | **Movement**             | `movement::movement_phase`                                 | Resolve movement intents, update `WorldPosition`, spatial index |
| 2  | **Combat**               | `combat::combat_phase`                                     | Resolve melee & ranged attacks, emit `DamageEvent`, `DeathEvent` |
| 3  | **Effects**              | `effects::effects_phase`                                   | Tick/decay status effects, remove expired |
| 4  | **Healing**              | `healing::healing_phase`                                   | Natural HP recovery, first aid application |
| 5  | **Morale**               | `morale::tick_morale_decay`                                | Decay/remove expired morale bonuses |
| 6  | **Bionics**              | `bionics::tick_bionics`                                    | Process bionic power drain / passive effects |
| 7  | **Temperature**          | `temperature::tick_temperature` + `tick_spoilage`          | Body temperature regulation, item spoilage |
| 8  | **Vision**               | `vision::update_vision`                                    | Update line-of-sight / visibility state |
| 9  | **Spawning**             | `spawning::spawning_phase`                                 | Process `SpawnEvent` buffer |
| —  | **Spatial index**        | `spatial::update_spatial_index` (reactive via `Changed<>`) | Maintain `EntitySpatialIndex` |

### Dependency graph

```
Turn start (tick_move_points)
    │
    ▼
Movement phase
    │
    ▼
Combat phase (may emit DeathEvent, DamageEvent)
    │
    ▼
Effects phase (may add/remove StatusEffect entities)
    │
    ▼
Healing phase (reads Health, modifies via BodyPartHp)
    │
    ▼
Morale decay (reads MoraleBonus, removes expired)
    │
    ▼
Bionics tick (reads Bionic, drains/charges power)
    │
    ▼
Temperature + Spoilage (reads BodyTemperature, Spoilable)
    │
    ▼
Vision update (reads WorldPosition, updates visibility)
    │
    ▼
Spawning phase (processes queued SpawnEvents)
    │
    ▼
Spatial index (Changed<WorldPosition> — reactive, no explicit ordering needed)
```

**NB:** The spatial index update (`spatial::update_spatial_index`) is not called
manually — it runs as a Bevy system triggered by `Changed<WorldPosition>`.
It must be registered in the Bevy app schedule, not in the manual tick loop.

### Turn queue execution loop

```
while queue.has_actors_ready() {
    let actor = queue.pop_highest();
    // run AI for this actor
    let goal = ai::decide_action(world, actor.entity);
    ai::execute_ai_action(world, actor.entity, goal);
    // cost is deducted inside the action
}
// when no actors remain above threshold → next global tick
```

The AI step runs inside the movement/combat loop, interleaving decisions with
action resolution for each actor.

---

## System Reference

### 1. `turn` — Master tick orchestration

**File:** `crates/cdda_sim/src/systems/turn.rs`

The `turn` module owns the `TurnQueue` resource and the top-level game tick.

#### Resources

| Resource | Type | Description |
|---|---|---|
| `TurnQueue` | `Resource` | Priority queue of actors, sorted by MP descending |
| `GameTime` | `Resource` | Global turn counter (`turn: u64`) |

#### Public API

```rust
pub fn tick_move_points(
    mut query: Query<(Entity, &mut MovePoints, &Speed), With<IsAlive>>,
    mut queue: ResMut<TurnQueue>,
    mut game_time: ResMut<GameTime>,
)
```
- **Reads:** `MovePoints`, `Speed`, `IsAlive`
- **Writes:** `MovePoints`, `TurnQueue`, `GameTime`
- **Emits:** `TurnAdvanced` (via `cdda_core::messages::TurnAdvanced`)

```rust
pub fn spend_move_points(
    entity: Entity,
    cost: i32,
    query: &mut Query<&mut MovePoints>,
) -> bool
```
Returns `true` if entity can still act (MP >= `MP_MIN_FLOOR`).

```rust
pub fn effective_move_cost(base_cost: i32, terrain_cost: i32) -> i32
```
Pure function — multiplies base cost by terrain factor.

**Master tick** (`game_tick`):
```rust
pub fn game_tick(world: &mut World) {
    turn::tick_move_points(...)   // implicit via Bevy schedule
    movement::movement_phase(world);
    combat::combat_phase(world);
    effects::effects_phase(world);
    healing::healing_phase(world);
    morale::tick_morale_decay(world);
    bionics::tick_bionics(world);
    temperature::tick_temperature(world);
    temperature::tick_spoilage(world);
    vision::update_vision(world);
    spawning::spawning_phase(world);
}
```

#### Action cost constants

```rust
pub const MOVE_COST_WALK: i32 = 100;
pub const MOVE_COST_RUN: i32 = 80;
pub const MOVE_COST_CROUCH: i32 = 150;
pub const MOVE_COST_ATTACK_BASE: i32 = 100;
pub const MOVE_COST_PICKUP: i32 = 100;
pub const MOVE_COST_RELOAD_BASE: i32 = 100;
pub const MP_MIN_FLOOR: i32 = 25;
```

---

### 2. `movement` — Movement resolution

**File:** `crates/cdda_sim/src/systems/movement.rs`

Resolves creatures' movement intents — pathfinding, collision detection,
terrain cost calculations.

#### Public API

```rust
pub fn movement_phase(world: &mut World)
```

```rust
pub fn calculate_move_cost(
    terrain_cost: i32,
    furniture_mod: i32,
    creature_speed: i32,
    is_swimming: bool,
    is_prone: bool,
    bleeding: bool,
) -> i32
```
Pure formula.

```rust
pub fn attempt_move(
    world: &mut World,
    entity: Entity,
    dx: i32,
    dy: i32,
) -> MoveResult
```
Attempts to move entity by (dx, dy). Checks passability, calls
`spend_move_points`, updates `WorldPosition`.

```rust
pub fn spend_move_points(world: &mut World, entity: Entity, amount: i32) -> i32
```
Deduct MP, return remaining.

```rust
pub fn is_passable(world: &World, entity: Entity, position: WorldPos) -> bool
```
Checks terrain move cost, furniture, solid entities at target position.

```rust
pub fn gain_move_points(world: &mut World)
```
Sets MP = speed for all creatures.

#### Components read/written

| Component | Access |
|---|---|
| `MovePoints` | Read/write |
| `Speed` | Read |
| `WorldPosition` | Write |
| `Solid` | Read (via is_passable) |
| `IsAlive` | Read |
| Terrain/Furniture (via `WorldMap` resource) | Read |

#### Events emitted

| Event | Condition |
|---|---|
| `SoundEvent` | On movement (footsteps) |
| (future) collision events | On blocked movement |

---

### 3. `combat` — Combat resolution

**File:** `crates/cdda_sim/src/systems/combat.rs`

Handles melee and ranged combat: hit chance, damage calculation, armour
mitigation, death handling.

#### Public API

```rust
pub fn combat_phase(world: &mut World)
```

```rust
pub fn calculate_melee_hit_chance(
    attacker_stats: &CombatStats,
    weapon_to_hit: i32,
    defender_dodge: i32,
) -> f32
```

```rust
pub fn calculate_melee_damage(
    weapon: &WeaponData,
    stats: &Stats,
    skill_level: u32,
) -> Damage
```

```rust
pub fn apply_damage_to_target(
    world: &mut World,
    target: Entity,
    damage: &Damage,
    armor: &DamageReduction,
) -> Damage
```

```rust
pub fn check_and_handle_death(
    world: &mut World,
    entity: Entity,
) -> bool
```

```rust
pub fn resolve_melee_attack(
    world: &mut World,
    attacker: Entity,
    defender: Entity,
) -> CombatResult
```

```rust
pub fn calculate_ranged_hit_chance(
    gun: &GunData,
    ammo: &AmmoData,
    distance: f64,
    shooter_skill: i32,
) -> f32
```

```rust
pub fn resolve_ranged_attack(
    world: &mut World,
    attacker: Entity,
    target_pos: WorldPos,
    weapon: Entity,
    ammo: Entity,
) -> CombatResult
```

```rust
pub fn melee_combat_phase(world: &mut World)
pub fn ranged_combat_phase(world: &mut World)
```

#### Types

```rust
pub struct CombatResult {
    pub hit: bool,
    pub damage: Damage,
    pub body_part_hit: Option<Entity>,
    pub critical: bool,
}

pub struct MeleeIntent {
    pub attacker: Entity,
    pub defender: Entity,
    pub weapon: Option<Entity>,
}
```

#### Components read/written

| Component | Access |
|---|---|
| `CombatStats` | Read |
| `Stats` | Read |
| `Health` | Write |
| `BodyPartHp` | Write |
| `IsAlive` | Remove on death |
| `WeaponData` | Read |
| `GunData` | Read |
| `AmmoData` | Read |
| `DamageReduction` | Read |
| `WorldPosition` | Read (range calc) |

#### Events emitted

| Event | Condition |
|---|---|
| `DamageEvent` | On successful hit |
| `DeathEvent` | When HP reaches 0 |
| `SoundEvent` | On attack sound |

---

### 4. `effects` — Status effects

**File:** `crates/cdda_sim/src/systems/effects.rs`

Applies, ticks, and removes status effects. Effects are individual entities
related to creatures via `EffectOn`/`ActiveEffects`.

#### Public API

```rust
pub fn effects_phase(world: &mut World)
```

```rust
pub fn apply_effect(
    world: &mut World,
    target: Entity,
    effect_id: EffectId,
    intensity: u32,
    duration: Time,
)
```
Creates a `StatusEffect` entity + `EffectOn(target)` relationship.

```rust
pub fn remove_effect(
    world: &mut World,
    target: Entity,
    effect_id: EffectId,
)
```
Despawns all effect entities with matching `effect_id` on the target.

```rust
pub fn has_effect(world: &World, entity: Entity, effect_id: EffectId) -> bool
```

```rust
pub fn get_effect_intensity(world: &World, entity: Entity, effect_id: EffectId) -> u32
```
Sums intensity of all matching effects on the entity.

```rust
pub fn tick_effects(world: &mut World)
```
Decays `StatusEffect.remaining`, despawns expired ones.

#### Components read/written

| Component | Access |
|---|---|
| `StatusEffect` | Read/write |
| `EffectOn` | Insert (on apply) |
| `ActiveEffects` | Read (via relationship target) |
| `IsAlive` | Read (filter) |

#### Events emitted

None directly (effects modify components inline).

---

### 5. `equipment` — Wielding, wearing, inventory management

**File:** `crates/cdda_sim/src/systems/equipment.rs`

Manages equipping/unequipping items. Uses relationships (`WieldedBy`/`WieldedItems`,
`WornOn`/`WornBy`) for bidirectional tracking.

#### Public API

```rust
pub fn wield_item(
    world: &mut World,
    creature: Entity,
    item: Entity,
) -> Result<(), EquipError>
```

```rust
pub fn unwield(world: &mut World, creature: Entity) -> Result<Entity, EquipError>
```

```rust
pub fn wear_item(
    world: &mut World,
    creature: Entity,
    item: Entity,
    slot: Option<String>,
) -> Result<(), EquipError>
```

```rust
pub fn take_off(
    world: &mut World,
    creature: Entity,
    item: Entity,
) -> Result<(), EquipError>
```

```rust
pub fn available_slots(world: &World, creature: Entity) -> Vec<String>
```

#### Types

```rust
pub enum EquipError {
    AlreadyWielding(Entity),
    NoFreeHands,
    SlotOccupied(String),
    ItemTooHeavy,
    ItemTooLarge,
    NotEquippable,
}
```

#### Components read/written

| Component | Access |
|---|---|
| `WieldedBy` | Insert/remove |
| `WieldedItems` | Read |
| `WornOn` | Insert/remove |
| `WornBy` | Read |
| `InsideContainer` | Remove (when picking up from ground) |
| `Pocket`, `PocketRestriction` | Read (for slot validation) |
| `Stats` | Read (for carry capacity) |

#### Events emitted

| Event | Condition |
|---|---|
| `EquipEvent` | On wield/wear |
| `UnequipEvent` | On unwield/take-off |
| `ItemMoveEvent` | On transfer between containers |

---

### 6. `vision` — Line of sight and visibility

**File:** `crates/cdda_sim/src/systems/vision.rs`

Calculates which entities are visible to which observers. Powers AI sight
detection and player field-of-view.

#### Public API

```rust
pub fn update_vision(world: &mut World)
```

```rust
pub fn calculate_vision_range(
    creature_vision: &Vision,
    time_of_day: &str,
    light_level: u32,
    has_night_vision: bool,
) -> i32
```

```rust
pub fn can_see(world: &World, observer: Entity, target: Entity) -> bool
```
Line-of-sight + range check using `EntitySpatialIndex` and terrain opacity.

```rust
pub fn visible_entities(world: &World, observer: Entity) -> Vec<Entity>
```

#### Components read/written

| Component | Access |
|---|---|
| `Vision` | Read |
| `WorldPosition` | Read |
| `Solid` | Read (blocking LOS) |
| Terrain opacity (via `WorldMap`) | Read |
| `SightEvent` | Emit |

#### Events emitted

| Event | Condition |
|---|---|
| `SightEvent` | When observer sees a new entity |

---

### 7. `temperature` — Body temperature and spoilage

**File:** `crates/cdda_sim/src/systems/temperature.rs`

Manages body temperature regulation for creatures and spoilage progression
for items.

#### Public API

```rust
pub fn tick_temperature(world: &mut World)
pub fn tick_spoilage(world: &mut World)
```

```rust
pub fn update_body_temperature(
    world: &mut World,
    entity: Entity,
    ambient_temp_celsius: f64,
)
```

```rust
pub fn calculate_total_warmth(world: &World, entity: Entity) -> i32
```
Sum of warmth values from worn items.

```rust
pub fn calculate_insulation(
    armour_parts: &[ArmourPart],
    material_thickness: f32,
) -> f32
```

```rust
pub fn spoilage_rate(
    temp_celsius: f64,
    is_sealed: bool,
    preserves_temp: bool,
) -> f64
```

#### Components read/written

| Component | Access |
|---|---|
| `BodyTemperature` | Write |
| `WornBy` | Read (for warmth) |
| `ArmourPart` | Read |
| `Spoilable` | Write (decay remaining time) |
| `Sealed` | Read |
| `PreservesTemp` | Read |
| `IsAlive` | Read (filter) |
| Ambient temperature (from `WorldMap` / weather) | Read |

#### Events emitted

None directly.

---

### 8. `healing` — Natural healing and first aid

**File:** `crates/cdda_sim/src/systems/healing.rs`

Natural HP recovery over time and first aid application.

#### Public API

```rust
pub fn healing_phase(world: &mut World)
```

```rust
pub fn natural_healing_tick(world: &mut World, entity: Entity)
```

```rust
pub fn apply_first_aid(
    world: &mut World,
    healer: Entity,
    patient: Entity,
    body_part: Entity,
    bandage_quality: u32,
    disinfectant_applied: bool,
) -> i32
```

```rust
pub fn healing_rate(
    health: &Health,
    is_sleeping: bool,
    nutrition_level: f32,
) -> f32
```

#### Components read/written

| Component | Access |
|---|---|
| `Health` | Read/write |
| `BodyPartHp` | Write |
| `BodyPartBroken` | Read |
| `BodyPartSevered` | Read |
| `IsAlive` | Read (filter) |

#### Events emitted

None directly.

---

### 9. `bionics` — Bionic/CBM system

**File:** `crates/cdda_sim/src/systems/bionics.rs`

Manages bionic activation, deactivation, passive power drain, and power
storage tracking.

#### Public API

```rust
pub fn tick_bionics(world: &mut World)
```

```rust
pub fn activate_bionic(
    world: &mut World,
    creature: Entity,
    bionic: Entity,
) -> Result<(), String>
```

```rust
pub fn deactivate_bionic(
    world: &mut World,
    creature: Entity,
    bionic: Entity,
)
```

```rust
pub fn total_power(world: &World, entity: Entity) -> Energy
```

#### Components read/written

| Component | Access |
|---|---|
| `Bionic` | Read/write (active flag) |
| `BionicOf` | Read |
| `InstalledBionics` | Read |
| `Energy` (resource or component) | Read/write |
| `IsAlive` | Read (filter) |

#### Events emitted

None directly.

---

### 10. `crafting` — Recipe crafting

**File:** `crates/cdda_sim/src/systems/crafting.rs`

Validates and executes crafting recipes. Pure validation with component
consumption.

#### Public API

```rust
pub fn can_craft(
    world: &World,
    creature: Entity,
    recipe_id: RecipeId,
) -> Result<(), String>
```
Checks skill requirements, tool availability, component availability.

```rust
pub fn calculate_craft_time(
    skill_set: &SkillSet,
    has_required_tools: bool,
) -> Time
```

```rust
pub fn consume_components(
    world: &mut World,
    creature: Entity,
    recipe_id: RecipeId,
) -> Result<(), String>
```
Removes required items from inventory.

```rust
pub fn available_recipes(
    world: &World,
    creature: Entity,
) -> Vec<RecipeId>
```
Filters all recipes by creature's skill levels.

#### Components read/written

| Component | Access |
|---|---|
| `SkillSet` | Read |
| `InsideContainer` | Read (inventory scan) |
| `ContainerContents` | Read |
| `StackCount` | Write (consumption) |

#### Events emitted

None directly (item consumption happens inline).

---

### 11. `morale` — Morale system

**File:** `crates/cdda_sim/src/systems/morale.rs`

Manages morale bonuses (temporary modifiers from food, drugs, events) and
their effect on creature stats.

#### Public API

```rust
pub fn add_morale_bonus(
    world: &mut World,
    creature: Entity,
    reason: String,
    amount: i32,
    duration: Time,
) -> Entity
```
Creates a `MoraleBonus` entity + `MoraleBonusOf(creature)` relationship.

```rust
pub fn calculate_morale(world: &World, entity: Entity) -> i32
```
Sums base morale + all active bonus amounts.

```rust
pub fn tick_morale_decay(world: &mut World)
```
Decays `MoraleBonus.remaining`, removes expired.

```rust
pub fn apply_morale_effects(world: &mut World, entity: Entity)
```
Applies morale modifiers to combat stats (e.g. -20% damage at low morale).

#### Components read/written

| Component | Access |
|---|---|
| `Morale` | Read/write |
| `MoraleBonus` | Read/write |
| `MoraleBonusOf` | Insert (on add) |
| `MoraleBonuses` | Read |
| `CombatStats` | Write (modified by morale) |
| `IsAlive` | Read (filter) |

#### Events emitted

None directly.

---

### 12. `ai` — Creature AI

**File:** `crates/cdda_sim/src/systems/ai.rs`

Decision-making for all non-player creatures. Reads world state (sensory
messages, spatial index, combat stats) and writes movement/combat intents.

#### Public API

```rust
pub fn ai_phase(world: &mut World)
```

```rust
pub fn decide_action(world: &World, entity: Entity) -> AiGoal
```

```rust
pub fn execute_ai_action(world: &mut World, entity: Entity, goal: AiGoal)
```

#### Types

```rust
pub enum AiGoal {
    Attack { target: Entity },
    Wander,
    Flee { from: Entity },
    Guard { position: WorldPos },
    Hunt { target: Entity },
}
```

#### Components read/written

| Component | Access |
|---|---|
| `CombatStats` | Read |
| `Vision` | Read |
| `Health` | Read (flee when low) |
| `MonsterFlags` | Read |
| `MonsterStats` | Read |
| `WorldPosition` | Read |
| `Faction` | Read |
| `NpcPersonality` | Read |
| `MovePoints` | Write (action costs) |
| `IsAlive` | Read (filter) |

#### Events emitted

None directly (AI writes intent components that other systems consume).

---

### 13. `spawning` — Entity spawning

**File:** `crates/cdda_sim/src/systems/spawning.rs`

Processes `SpawnEvent` messages to create new monster/item entities from
definition templates. Uses `EntityCloner` for efficient def→instance cloning.

#### Public API

```rust
pub fn spawning_phase(world: &mut World)
```
Reads `SpawnEvent` message buffer and spawns entities.

```rust
pub fn spawn_monster(
    world: &mut World,
    template_id: MonsterId,
    position: WorldPos,
    faction: FactionId,
) -> Entity
```

```rust
pub fn spawn_item(
    world: &mut World,
    template_id: ItemId,
    position: WorldPos,
    count: u32,
) -> Entity
```

```rust
pub fn spawn_from_group(
    world: &mut World,
    group_id: ItemGroupId,
    position: WorldPos,
) -> Vec<Entity>
```
Spawns items from an item group definition.

#### Components read/written

| Component | Access |
|---|---|
| `IsDef`, `DefStrId` (via `DefinitionWorld`) | Read (def lookup) |
| `WorldPosition` | Insert |
| `StackCount` | Insert |
| `Health` | Insert (for creatures) |
| `Faction` | Insert |
| `IsAlive` | Insert |
| `Solid` | Insert (for creatures) |

#### Events emitted

None (spawning consumes `SpawnEvent` messages).

---

### 14. `inventory` — Container operations

**File:** `crates/cdda_sim/src/systems/inventory.rs`

Low-level container operations — picking up, dropping, transferring items
between containers. These are utility functions called by other systems
(equipment, looting, crafting).

#### Public API

```rust
pub fn pickup_item(
    commands: &mut Commands,
    collector: Entity,
    item: Entity,
    item_query: &Query<(&WorldPosition, Option<&StackCount>)>,
) -> Option<ItemMoveEvent>
```

```rust
pub fn drop_item(
    commands: &mut Commands,
    container: Entity,
    item: Entity,
    drop_pos: WorldPos,
) -> Option<ItemMoveEvent>
```

```rust
pub fn transfer_item(
    commands: &mut Commands,
    item: Entity,
    from_container: Entity,
    to_container: Entity,
) -> Option<ItemMoveEvent>
```

```rust
pub fn effective_position(
    item: Entity,
    positions: &Query<&WorldPosition>,
    containers: &Query<&InsideContainer>,
) -> Option<WorldPos>
```

```rust
pub fn items_at_position(
    pos: WorldPos,
    positions: &Query<(&WorldPosition, Entity), With<StackCount>>,
) -> Vec<Entity>
```

```rust
pub fn items_in_container(
    container: Entity,
    inside: &Query<(Entity, &InsideContainer)>,
) -> Vec<Entity>
```

```rust
pub fn can_fit_in_container(world: &World, container: Entity, item: Entity) -> bool
```
Volume/weight/flag check against container pocket.

```rust
pub fn total_container_volume(world: &World, container: Entity) -> Volume
```
Sum of item volumes in container.

```rust
pub fn total_container_weight(world: &World, container: Entity) -> Weight
```
Sum of item weights in container.

```rust
pub fn merge_or_stack(world: &mut World, target: Entity, incoming: Entity) -> bool
```
Merge if same type, return true if merged.

#### Components read/written

| Component | Access |
|---|---|
| `InsideContainer` | Insert/remove |
| `WorldPosition` | Insert/remove |
| `ContainerContents` | Read |
| `StackCount` | Read/write (merge) |
| `Pocket`, `PocketRestriction` | Read (fit check) |

#### Events emitted

| Event | Condition |
|---|---|
| `ItemMoveEvent` | On any item move |

---

### 15. `spatial` — Spatial index maintenance

**File:** `crates/cdda_sim/src/systems/spatial.rs`

Reactive systems that keep the `EntitySpatialIndex` resource in sync with
entity positions. These run on `Changed<WorldPosition>` and
`RemovedComponents<WorldPosition>`.

#### Public API

```rust
pub fn update_spatial_index(
    mut spatial: ResMut<EntitySpatialIndex>,
    query: Query<(Entity, &WorldPosition), Changed<WorldPosition>>,
)
```

```rust
pub fn cleanup_spatial_index(
    mut spatial: ResMut<EntitySpatialIndex>,
    mut removals: RemovedComponents<WorldPosition>,
)
```

#### Components read/written

| Component | Access |
|---|---|
| `WorldPosition` | Read (Changed) |
| `EntitySpatialIndex` (resource) | Write |

#### Events emitted

None.

---

## Data Flow Between Systems

### Turn start → Movement → Combat

```
TurnQueue.pop_highest()
    │
    ▼
AI: decide_action(entity)
    │
    ├── AiGoal::Attack → movement_phase (approach) → combat_phase (attack)
    ├── AiGoal::Wander → movement_phase (random step)
    ├── AiGoal::Flee   → movement_phase (away from threat)
    ├── AiGoal::Hunt   → movement_phase (toward target, pathfinding)
    └── AiGoal::Guard  → movement_phase (stay near position)
```

### Combat → Death → Spawning

```
Combat phase
    ├── DamageEvent (broadcast)
    ├── Health.current -= damage
    └── If Health.current <= 0:
         ├── DeathEvent (broadcast)
         ├── Remove IsAlive
         ├── SpawnEvent (for loot drops, via DeathDrops)
         └── Effects phase (remove applied effects next tick)
```

### Vision → AI

```
Vision phase updates visibility state for each creature
    │
    ▼
AI phase reads visible entities + sound events
    ├── SightEvent → AI can react to newly seen entities
    ├── SoundEvent → AI can investigate sounds
    └── Faction checks → determine friend/foe
```

### Temperature → Spoilage → Effects

```
Temperature phase
    ├── Updates BodyTemperature (warmth from worn items, ambient temp)
    └── Spoilable items decay based on local temperature
         └── When Spoilable.remaining <= 0:
              └── Replace item with rotten version (ItemId replacement)
```

### Morale → Combat stats

```
Morale phase
    ├── Decay/remove expired MoraleBonus entities
    ├── Recalculate Morale value
    └── Apply modifiers to CombatStats (e.g. damage penalty at low morale)
         └── Affects combat phase hit/damage calculations
```

### Equipment → Inventory

```
Equipment system
    ├── wield_item: removes InsideContainer, inserts WieldedBy
    ├── wear_item: removes InsideContainer, inserts WornOn
    ├── unwield: removes WieldedBy, inserts InsideContainer(fallback) or WorldPosition
    └── take_off: removes WornOn, inserts InsideContainer or WorldPosition
         └── All operations emit ItemMoveEvent
```

---

## Message Flow Diagram

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│  TurnStart   │────▶│  Movement    │────▶│  Combat      │
│ (tick_mp)    │     │              │     │              │
└──────────────┘     └──────────────┘     └──────┬───────┘
                                                  │
                    ┌─────────────────────────────┘
                    ▼
          ┌──────────────────┐
          │  DamageEvent     │
          │  DeathEvent      │
          │  SoundEvent      │
          └──────┬───────────┘
                 │
    ┌────────────┼────────────┐
    ▼            ▼            ▼
┌────────┐ ┌──────────┐ ┌──────────┐
│Effects │ │ AI (reads│ │ Spawning │
│phase   │ │ sensory  │ │ (loot)   │
│        │ │ messages)│ │          │
└────────┘ └──────────┘ └──────────┘

SightEvent ──▶ AI decision (Hunt/Attack/Flee)
SoundEvent ──▶ AI investigation (Wander toward source)
ItemMoveEvent ──▶ Equipment/Inventory tracking
SpawnEvent ──▶ Spawning phase (deferred creation)
```

---

## Registration in the Bevy App

Systems must be registered in `cdda_app` (or `cdda_sim::plugins::SimPlugin`)
as follows:

### Bevy systems (reactive, use schedule)

```rust
app.add_systems(Update, (
    spatial::update_spatial_index,
    spatial::cleanup_spatial_index,
    turn::tick_move_points,
    turn::debug_turn_queue,
).chain().in_set(GameSet::Sim));
```

### Manual tick loop (called from `game_tick`)

The manual loop is called from a Bevy system that observes input/state:

```rust
fn main_game_loop(world: &mut World) {
    let turn_state = world.resource::<TurnState>();
    if *turn_state != TurnState::PlayerActed {
        return;
    }
    game_tick(world);  // calls all phases in order
    world.insert_resource(TurnState::WaitingForInput);
}
```

Or called directly from the Bevy runner when in headless/testing mode:

```rust
fn test_game_tick(world: &mut World) {
    for _ in 0..100 {
        game_tick(world);
    }
}
```
