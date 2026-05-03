# CDDA-BR — Target Architecture

> The architecture we are building toward. This document captures architectural
> decisions from the analysis of the current codebase (see `CURRENT_ARCHITECTURE.md`)
> and represents the desired end-state.
>
> **Target Bevy version: 0.18** (released 2026-01-13).
>
> This document is the canonical forward-looking reference. It should be updated
> whenever a structural decision changes. When a section of this document is fully
> implemented, it should be reflected in `CURRENT_ARCHITECTURE.md` and this document
> updated accordingly.

---

## Core Architectural Pivot: The Anti-Corruption Layer

The single most important structural change from the current codebase: **CDDA JSON
format knowledge is isolated behind an Anti-Corruption Layer (ACL) in `cdda_data`.**

Currently, `cdda_core::defs/` defines structs that are direct mirrors of CDDA JSON:
field names match CDDA naming (`looks_like`, `copy_from`, `abstract_`), every field
is `Option<T>` because CDDA's format is sparse, and a massive `cdda_types.rs` file
exists solely to handle CDDA's inconsistent JSON shapes (`CddaColor` can be string
OR array OR object, `UseAction` has 5 different JSON representations, etc.).

In the target architecture:

- **`cdda_core`** contains pure domain types with zero IO knowledge. No `serde`,
  no `schemars`, no `#[serde(rename)]`, no `Option<T>`-for-every-field. Fast
  compile times, pure computation.

- **`cdda_data`** contains the raw CDDA JSON types (the current `defs/`), the
  copy-from resolver, and **the ACL**: translation from raw CDDA types into pure
  domain types. This is the only place in the codebase that knows CDDA field names,
  JSON shapes, or serialization quirks.

- **`cdda_sim`, `cdda_map`, `cdda_render`** never see a raw CDDA type. They
  operate exclusively on pure domain types and ECS components.

```
┌──────────────────────────────────────────────────────────────┐
│ cdda_sim / cdda_map / cdda_render / cdda_input / cdda_audio   │
│                                                               │
│ Pure domain types. Numeric IDs. ECS components.               │
│ No strings in hot paths. No serde on domain types.            │
├──────────────────────────────────────────────────────────────┤
│ cdda_core                                                     │
│                                                               │
│ Units (Volume, Weight, Time, Energy) — pure newtypes, no serde │
│ Coords (Pos<S,O>, ZLevel, Direction, Facing)                  │
│ Numeric ID types (ItemId, MonsterId, TerrainId, ...)          │
│ Domain value types (Damage, Stats, FlagSet)                   │
│ Component templates (ItemBase, Weapon, Armor, Container, ...) │
│                                                                │
│ ZERO dependencies. ZERO serde. ZERO schemars.                 │
│ Compiles in under 1 second.                                   │
├──────────────────────────────────────────────────────────────┤
│ cdda_data                                                     │
│                                                               │
│ Raw CDDA JSON types (the current cdda_core::defs/)            │
│ Copy-from resolver (extend/delete/relative/proportional)      │
│ Custom serde for CDDA string formats ("250 ml", "1 kg")       │
│ ── THE ACL ──                                                 │
│   raw CDDA def → pure domain type translation                  │
│   string → numeric ID mapping (built at load, used for save)  │
│   component template construction                              │
│ Save format serialization                                     │
│ Hot-reloadable JSON file watcher                              │
│                                                                │
│ Dependencies: cdda_core, serde, serde_json, schemars          │
└──────────────────────────────────────────────────────────────┘
```

---

## 1. Numeric ID System

### 1.1 Two ID Types for Two Semantics

```rust
// cdda_core/src/id.rs

/// A definition index. Never freed, never recycled.
/// Used for the static DefRegistry populated at load time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DefIdx(pub u32);

/// A generation-counted handle. Prevents ABA problems when
/// world entities are despawned and IDs recycled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GenId {
    pub index: u32,
    pub generation: u32,
}

// Per-category concrete ID types — NOT generic over T.
// Concrete types enable Vec<T> storage with O(1) indexing.
// The type system prevents passing a MonsterId where an ItemId is expected.

pub struct ItemId(pub DefIdx);
pub struct MonsterId(pub DefIdx);
pub struct TerrainId(pub DefIdx);
pub struct FurnitureId(pub DefIdx);
pub struct RecipeId(pub DefIdx);
pub struct ItemGroupId(pub DefIdx);
pub struct FieldId(pub DefIdx);
pub struct MutationId(pub DefIdx);
pub struct BionicId(pub DefIdx);
pub struct EffectId(pub DefIdx);
pub struct FactionId(pub DefIdx);
pub struct SkillId(pub DefIdx);
pub struct VehiclePartId(pub DefIdx);
pub struct MapgenPaletteId(pub DefIdx);
pub struct OvermapTerrainId(pub DefIdx);
pub struct AmmoTypeId(pub DefIdx);
```

### 1.2 IdSlab — Dense Typed Storage

```rust
// cdda_core/src/id_slab.rs

/// Dense generational storage. O(1) lookup by GenId, O(1) iteration.
/// Array-backed with generational protection against ABA.
pub struct IdSlab<T> {
    entries: Vec<Option<(u32, T)>>,   // (generation, value)
    free: Vec<u32>,                    // free indices
    next_generation: u32,
}

impl<T> IdSlab<T> {
    pub fn insert(&mut self, value: T) -> GenId { /* ... */ }
    pub fn get(&self, id: GenId) -> Option<&T> { /* ... */ }
    pub fn get_mut(&mut self, id: GenId) -> Option<&mut T> { /* ... */ }
    pub fn remove(&mut self, id: GenId) -> bool { /* ... */ }
    pub fn iter(&self) -> impl Iterator<Item = (GenId, &T)> { /* ... */ }
}
```

### 1.3 DefRegistry — Dense Vec Storage

```rust
// cdda_core/src/registry.rs

pub struct DefRegistry {
    // Each category stored as Vec<T> indexed by DefIdx.0.
    // Lookup is a single bounds check, no hashing.

    pub items: Vec<ItemTemplate>,
    pub monsters: Vec<MonsterTemplate>,
    pub terrain: Vec<TerrainTemplate>,
    pub furniture: Vec<FurnitureTemplate>,
    pub recipes: Vec<RecipeTemplate>,
    pub item_groups: Vec<ItemGroupTemplate>,
    pub mapgen_palettes: Vec<MapgenPaletteTemplate>,
    pub overmap_terrains: Vec<OvermapTerrainTemplate>,
    pub fields: Vec<FieldTemplate>,
    pub vehicle_parts: Vec<VehiclePartTemplate>,
    pub mutations: Vec<MutationTemplate>,
    pub bionics: Vec<BionicTemplate>,
    pub effects: Vec<EffectTemplate>,
    pub factions: Vec<FactionTemplate>,
    pub skills: Vec<SkillTemplate>,
    pub scenarios: Vec<ScenarioTemplate>,

    // String ↔ numeric maps. Built at load. Only used by the ACL for save/reload.
    pub(crate) item_ids: HashMap<String, DefIdx>,
    pub(crate) item_names: Vec<String>,
    pub(crate) monster_ids: HashMap<String, DefIdx>,
    pub(crate) monster_names: Vec<String>,
    // ... per-category reverse maps ...
}

impl DefRegistry {
    /// O(1) lookup by numeric ID.
    pub fn item(&self, id: ItemId) -> &ItemTemplate {
        &self.items[id.0 .0 as usize]
    }

    /// String lookup. Only used at load/save boundaries, never in hot paths.
    pub fn item_by_name(&self, name: &str) -> Option<ItemId> {
        self.item_ids.get(name).map(|&idx| ItemId(idx))
    }

    /// Reverse lookup for save serialization.
    pub fn item_name(&self, id: ItemId) -> &str {
        &self.item_names[id.0 .0 as usize]
    }
}
```

---

## 2. Component-Based Templates (Not Monolithic Defs)

CDDA's `ItemDef` is 12+ archetypes crammed into one struct through sparsity.
In the target architecture, item definitions decompose into composable behaviors:

```rust
// cdda_core/src/templates/item.rs

/// Every item has a base. Non-optional.
pub struct ItemBase {
    pub name: String,
    pub description: String,
    pub volume: Volume,
    pub weight: Weight,
    pub material: Vec<MaterialId>,
    pub symbol: char,
    pub color: Color,
    pub flags: FlagSet,
    pub phase: Phase,
    pub category: ItemCategoryId,
}

/// Behaviors are OPTIONAL and composable. Each maps to ECS components
/// stamped onto items at spawn time.

pub struct WeaponBehavior {
    pub damage: Damage,
    pub to_hit: i32,
    pub techniques: Vec<TechniqueId>,
    pub reach: u32,
}

pub struct ArmorBehavior {
    pub coverage: HashMap<BodyPart, f64>,
    pub encumbrance: u32,
    pub protection: Damage,
    pub material_thickness: f64,
    pub warmth: i32,
    pub environmental_protection: u32,
}

pub struct ContainerBehavior {
    pub max_volume: Volume,
    pub max_weight: Weight,
    pub max_item_length: Length,
    pub sealed: bool,
    pub rigid: bool,
    pub pocket_type: PocketType,
}

pub struct FoodBehavior {
    pub calories: u32,
    pub quench: i32,
    pub fun: i32,
    pub spoils_in: Time,
    pub vitamins: Vec<(VitaminId, u32)>,
    pub comestible_type: ComestibleType,
}

pub struct ToolBehavior {
    pub max_charges: u32,
    pub charges_per_use: u32,
    pub qualities: Vec<ToolQuality>,
    pub revert_to: Option<ItemId>,
}

pub struct AmmoBehavior {
    pub ammo_type: AmmoTypeId,
    pub damage: Damage,
    pub count: u32,
    pub effects: Vec<AmmoEffectId>,
}

pub struct MagazineBehavior {
    pub ammo_type: Vec<AmmoTypeId>,
    pub capacity: u32,
    pub reload_time: u32,
    pub compatible_weapons: Vec<ItemId>,
}

pub struct BookBehavior {
    pub intelligence_required: u32,
    pub skill: SkillId,
    pub max_level: u32,
    pub fun: i32,
    pub time: Time,
    pub chapters: u32,
}

pub struct GunModBehavior {
    pub install_time: Time,
    pub modifies: Vec<GunModSlot>,
}

pub struct DrugBehavior {
    pub effects: Vec<DrugEffect>,
    pub duration: Time,
    pub addiction_potential: u32,
}

/// A complete item template. Only populated behaviors exist.
pub struct ItemTemplate {
    pub base: ItemBase,
    pub weapon: Option<WeaponBehavior>,
    pub armor: Option<ArmorBehavior>,
    pub container: Option<ContainerBehavior>,
    pub food: Option<FoodBehavior>,
    pub tool: Option<ToolBehavior>,
    pub ammo: Option<AmmoBehavior>,
    pub magazine: Option<MagazineBehavior>,
    pub book: Option<BookBehavior>,
    pub gun_mod: Option<GunModBehavior>,
    pub drug: Option<DrugBehavior>,
}
```

### 2.1 Spawning Items as ECS Entities

```rust
// cdda_sim/src/systems/spawning.rs

fn spawn_item(
    commands: &mut Commands,
    template: &ItemTemplate,
    pos: WorldPos,
    count: u32,
) -> Entity {
    let mut entity = commands.spawn((
        template.base.clone(),
        WorldPosition(pos),
        StackCount(count),
    ));

    if let Some(w) = &template.weapon { entity.insert(w.clone()); }
    if let Some(a) = &template.armor   { entity.insert(a.clone()); }
    if let Some(c) = &template.container { entity.insert(c.clone()); }
    if let Some(f) = &template.food    { entity.insert(f.clone()); }
    if let Some(t) = &template.tool    { entity.insert(t.clone()); }
    if let Some(a) = &template.ammo    { entity.insert(a.clone()); }
    if let Some(m) = &template.magazine { entity.insert(m.clone()); }
    if let Some(b) = &template.book    { entity.insert(b.clone()); }
    if let Some(g) = &template.gun_mod { entity.insert(g.clone()); }
    if let Some(d) = &template.drug    { entity.insert(d.clone()); }

    entity.id()
}
```

Systems query for exactly what they need:

```rust
// Only entities with Weapon get melee capability
fn melee_damage_system(
    weapons: Query<(&WeaponBehavior, &ItemBase)>,
) { /* ... */ }

// Only entities with Container can hold items
fn container_insert_system(
    containers: Query<(&mut ContainerBehavior, &Children)>,
) { /* ... */ }
```

Bevy's archetype-based storage means entities with `(ItemBase, WeaponBehavior, ToolBehavior)`
live in a different archetype from `(ItemBase, FoodBehavior)`, and systems only iterate
over matching archetypes. No branching on optional fields in hot loops.

### 2.2 Monsters as Templates (Same Pattern)

```rust
pub struct MonsterTemplate {
    pub base: MonsterBase,
    pub stats: Stats,
    pub combat_stats: MonsterCombatStats,
    pub body_type: BodyType,
    pub species: SpeciesId,
    pub vision: Vision,
    pub armor: ArmorSet,
    pub special_attacks: Vec<SpecialAttackId>,
    pub death_drops: ItemGroupId,
    pub upgrade_path: Option<(MonsterId, Time)>,
    pub flags: FlagSet,
    pub factions: Vec<FactionId>,
}
```

### 2.3 Bevy Relationships for Inventory Trees

Bevy 0.16 shipped first-class entity relationships (ChildOf, Children,
Relationship trait). Consider modeling inventory pockets as entities linked
via relationships rather than hand-rolling recursive trees inside a single
Inventory component. Benefits: free change detection on pocket contents, free
despawn propagation, and free bevy-inspector-egui visibility. Tradeoff: more
entities equals more archetype fragmentation. Measure with realistic inventory
loads before committing.

---

## 3. Map Storage: Struct-of-Arrays

The `Tile` struct is replaced with SoA layout for cache efficiency:

```rust
// cdda_map/src/tile_grid.rs

pub const BUBBLE_DIM: usize = 132;
pub const BUBBLE_SIZE: usize = BUBBLE_DIM * BUBBLE_DIM;

/// A 132×132 grid of tiles stored as Struct-of-Arrays.
/// All arrays are BUBBLE_SIZE elements, laid out contiguously.
/// Total ~200 KB per bubble — fits in L3 cache on consumer CPUs.
pub struct BubbleGrid {
    /// Terrain IDs. Always populated (every tile has terrain).
    pub terrains: Box<[TerrainId; BUBBLE_SIZE]>,

    /// Furniture IDs. Sparse — most are None.
    pub furniture: Box<[Option<FurnitureId>; BUBBLE_SIZE]>,

    /// Traps. Very sparse.
    pub traps: Box<[Option<TrapId>; BUBBLE_SIZE]>,

    /// Fields. Sparse — each tile has a SmallVec.
    pub fields: Box<[SmallVec<[FieldEntry; 2]>; BUBBLE_SIZE]>,

    /// Item heads: u32 index into item_store, or u32::MAX for none.
    /// Items on the same tile form a linked list through `next_on_tile`.
    item_heads: Box<[u32; BUBBLE_SIZE]>,
    item_store: ItemStore,
}

impl BubbleGrid {
    #[inline]
    pub fn terrain(&self, x: u32, y: u32) -> TerrainId {
        self.terrains[morton_index(x, y)]
    }

    /// Iterate terrain IDs in Morton order (spatial locality).
    pub fn iter_terrains(&self) -> impl Iterator<Item = TerrainId> + '_ {
        self.terrains.iter().copied()
    }
}
```

### 3.1 Morton (Z-order) Curve Indexing

```rust
/// Morton encoding: interleave x and y bits so that points close in
/// 2D space are close in memory. Significantly improves cache behavior
/// for spatial queries (FOV, pathfinding, mapgen).
///
/// Not applied preemptively — benchmark row-major vs Morton with
/// `perf stat -e cache-misses` before switching.
#[inline]
fn morton_index(x: u32, y: u32) -> usize {
    (morton_encode(x) | (morton_encode(y) << 1)) as usize
}

fn morton_encode(mut n: u32) -> u64 {
    // 32-bit number → 64-bit with zero-interleaved bits
    let mut result: u64 = 0;
    for i in 0..16 {
        result |= ((n as u64 & (1 << i)) << i) as u64;
    }
    result
}
```

### 3.2 Dense Item Store

Items on the ground use a dense pool with free-list, not per-tile Vecs:

```rust
pub struct ItemStore {
    items: Vec<ItemOnGround>,
    free: Vec<u32>,
}

pub struct ItemOnGround {
    pub item_id: ItemId,
    pub count: u32,
    pub charges: u32,
    pub age: Time,
    pub next_on_tile: u32,  // u32::MAX = end of list
}
```

No per-tile heap allocations. A reality bubble with 50 ground items allocates
50 entries in the store, not 17,424 empty Vecs.

---

## 4. Event-Driven System Communication

Systems communicate through Bevy events, not direct mutation:

```rust
// cdda_sim/src/events.rs

#[derive(Event)]
pub struct DamageEvent {
    pub target: Entity,
    pub amount: i32,
    pub kind: DamageKind,
    pub source: Option<Entity>,
}

#[derive(Event)]
pub struct DeathEvent {
    pub entity: Entity,
    pub cause: DeathCause,
    pub position: WorldPos,
}

#[derive(Event)]
pub struct SoundEvent {
    pub position: WorldPos,
    pub volume: u32,
    pub description: String,
    pub cause: SoundCause,
}

#[derive(Event)]
pub struct SpawnEvent {
    pub template_id: MonsterId,
    pub position: WorldPos,
    pub faction: FactionId,
}

#[derive(Event)]
pub struct ItemMoveEvent {
    pub item: Entity,
    pub from: Location,
    pub to: Location,
}

#[derive(Event)]
pub struct DefChangedEvent {
    pub category: DefCategory,
    pub ids: Vec<u32>,       // numeric IDs of changed defs
}
```

### 4.1 Event Flow Example

```
Combat System → DamageEvent → Health System → DeathEvent → Loot System
                                        ↓
                                  SoundEvent → AI System (sound reaction)
                                        ↓
                                  Achievement System (kill tracking)
```

Systems are decoupled. The combat system doesn't know health, sound, or
achievements exist. Adding a new reaction is a new event reader, no existing
code changes.

### 4.2 Turn State and the Hot-Reload Boundary

```rust
// cdda_app/src/tick.rs

#[derive(Resource)]
pub enum TurnState {
    WaitingForInput,
    PlayerActed,      // input committed, sim about to run
    Simulating,       // simulation in progress
    Animating,        // rendering inter-turn animations
}

#[hot]
fn game_tick(
    world: &mut World,
    rng: ResMut<SeededRng>,
    turn_state: ResMut<TurnState>,
    mut patch_events: EventReader<PatchApplied>,
) {
    if !patch_events.is_empty() {
        patch_events.clear();
        info!("Hot patch applied");
    }

    match *turn_state {
        TurnState::WaitingForInput => { /* wait */ }
        TurnState::PlayerActed => {
            // The hot boundary. subsecond unwinds to here on code changes.
            subsecond::call(|| {
                *turn_state = TurnState::Simulating;
                run_ai_phase(world, rng);
                run_movement_phase(world);
                run_combat_phase(world);
                run_effects_phase(world);
                advance_turn(world, rng);
                *turn_state = TurnState::Animating;
            });
        }
        TurnState::Simulating => { /* shouldn't reach here */ }
        TurnState::Animating => { /* inter-turn animation */ }
    }
}
```

The player action commit is the persistence point. AI, movement, combat run
inside `subsecond::call()`. If any inner function is hot-patched mid-execution,
subsecond unwinds to the `subsecond::call()` boundary and retries. The player's
action is already committed. The RNG is deterministic, so retry produces
identical random rolls. The world converges to the correct state.

---

## 5. Spatial Index for Dynamic Entities

Separate from the tile grid — for fast radius queries on moving entities:

```rust
// cdda_map/src/spatial.rs

#[derive(Resource)]
pub struct EntitySpatialIndex {
    /// Grid acceleration: divide the bubble into 4×4 tile cells.
    /// Each cell stores entities in that region.
    cells: Box<[SmallVec<[Entity; 8]>; (BUBBLE_DIM / 4) * (BUBBLE_DIM / 4)]>,
    /// Reverse map: entity → current cell index.
    entity_cells: HashMap<Entity, u32>,
}

impl EntitySpatialIndex {
    pub fn update_position(&mut self, entity: Entity, old_pos: BubblePos, new_pos: BubblePos) {
        let old_cell = cell_index(old_pos.x / 4, old_pos.y / 4);
        let new_cell = cell_index(new_pos.x / 4, new_pos.y / 4);
        if old_cell != new_cell {
            self.cells[old_cell].retain(|&e| e != entity);
            self.cells[new_cell].push(entity);
            self.entity_cells.insert(entity, new_cell as u32);
        }
    }

    pub fn query_radius(&self, center: BubblePos, radius: u32) -> Vec<Entity> {
        let cell_radius = (radius / 4) as i32 + 1;
        let center_cx = center.x as i32 / 4;
        let center_cy = center.y as i32 / 4;
        let mut result = Vec::new();
        for cy in (center_cy - cell_radius)..=(center_cy + cell_radius) {
            for cx in (center_cx - cell_radius)..=(center_cx + cell_radius) {
                if let Some(cell) = self.cells.get(cell_index_usize(cx, cy)) {
                    result.extend(cell.iter());
                }
            }
        }
        result
    }

    pub fn despawn(&mut self, entity: Entity) {
        if let Some(&cell) = self.entity_cells.get(&entity) {
            self.cells[cell as usize].retain(|&e| e != entity);
            self.entity_cells.remove(&entity);
        }
    }
}
```

A radius-30 query checks ~200 cells instead of 17,424 tiles.

For overmap-scale queries (nearest city), `rstar` R*-tree remains the tool.

---

## 6. Mod Architecture with Sharded ID Space

Mods must not perturb core ID assignments. Solution: shard the numeric ID space.

```rust
// cdda_data/src/mod_shard.rs

pub const ID_SHARD_CORE: u32 = 0x0000_0000;
pub const ID_SHARD_MOD_START: u32 = 0x8000_0000;
// Core items: indices 0..0x7FFF_FFFF (~2 billion, effectively unlimited)
// Mod items:  indices 0x8000_0000..0xFFFF_FFFF

pub struct ModShard {
    pub mod_id: String,
    pub base_index: u32,
    pub count: u32,
    /// string → offset within this shard
    pub name_to_offset: HashMap<String, u32>,
    /// offset → string (for save/reload)
    pub offset_to_name: Vec<String>,
}
```

Each mod gets a contiguous block of `u32` space allocated at load time.
Mod dependencies are resolved topologically; shard allocation follows
dependency order. The global `ItemId` is `base_index + local_offset`.

Save files store `(mod_id, local_offset)` pairs so saves remain valid
across different mod configurations, as long as the mod itself hasn't
changed its internal IDs.

---

## 7. Save/Load with Change Detection

Bevy's per-component change detection enables incremental saves:

```rust
// cdda_sim/src/systems/save.rs

fn incremental_save(
    changed_positions: Query<(Entity, &WorldPosition), Changed<WorldPosition>>,
    changed_health: Query<(Entity, &Health), Changed<Health>>,
    changed_tiles: Query<&Tile, Or<(Changed<Terrain>, Changed<FieldSet>)>>,
    // ... per-component Changed<T> queries
) {
    // Only mutated entities and tiles are written.
    // Full saves every N turns for compaction.
}
```

### 7.1 Write-Ahead Log Layout

```
save/
├── full/0000147/         ← full save every 100 turns
│   ├── world.json        ← player, time, resources
│   ├── creatures.bin     ← all creature entities
│   └── tiles.bin         ← all loaded submaps
└── incr/
    ├── 0000148.delta     ← only changed components since last save
    ├── 0000149.delta
    └── 0000150.delta
```

Delta format:

```
[TURN 148]
entity 42: Health(78→65), WorldPosition((12,3,0)→(11,3,0))
entity 891: despawned
entity 892: spawned { template: zombie_child, pos: (45, 12, 0), faction: zombies }
tile (34,56,0): Terrain(t_floor→t_rubble), Furniture(f_chair→None)
```

Rollback: replay deltas since last full save. Compaction: write new full save,
truncate deltas.

### 7.2 Save Atomicity

Every write uses write-to-temp-then-rename. An interrupted save never corrupts
existing data. The save/load module in `cdda_data` is the only place that
serializes numeric IDs back to string names via the reverse maps.

---

## 8. Three-Tier Hot Reload

| Tier | What | Mechanism | Latency | Trigger |
|------|------|-----------|---------|---------|
| **T1: Data** | JSON defs (items, monsters, field values) | ACL data reload + `DefChanged` events | <100ms | File watcher |
| **T2: Mapgen replay** | Regenerate current OMT with new palette/data | `#[hot(rerun_on_hot_patch)]` system | <500ms | T1 or manual |
| **T3: Code** | Rust systems, logic functions, AI scoring | `dx serve --hotpatch` + subsecond + ThinLink | 130–500ms | File watcher |

### 8.1 T1: Data Hot Reload

```
1. notify::Watcher detects change in data/core/items/melee.json
2. Debounce (50ms) to coalesce burst writes
3. Parse changed file(s) → RawValue
4. Resolve copy-from for affected defs
5. Translate through ACL → ItemTemplates
6. Update DefRegistry atomically (swap Vec<T> entries)
7. Emit DefChangedEvent for each affected numeric ID
8. Systems react: UI updates, entity validation, mapgen invalidation
```

### 8.2 T2: Mapgen Replay

```rust
#[hot(rerun_on_hot_patch = true)]
fn regenerate_current_omt(
    world: &mut World,
    registry: Res<DefRegistry>,
    player_pos: Res<PlayerPosition>,
    spatial: Res<EntitySpatialIndex>,
) {
    let omt = player_pos.to_omt();
    // Despawn entities in OMT
    // Re-run mapgen with current templates
    // Re-spawn items, monsters, furniture from templates
    // Spatial index updates automatically via position change detection
}
```

### 8.3 T3: Code Hot Reload via Subsecond

```rust
// cdda_app/Cargo.toml
[features]
hot = ["bevy_simple_subsecond_system", "dioxus-devtools"]

[dependencies]
bevy_simple_subsecond_system = { version = "0.2", optional = true }
dioxus-devtools = { version = "0.7", optional = true }


// cdda_app/src/main.rs
fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);

    #[cfg(feature = "hot")]
    {
        app.add_plugins(bevy_simple_subsecond_system::SimpleSubsecondPlugin::default());
        dioxus_devtools::connect_subsecond();
    }

    app.add_systems(Update, game_tick);
    app.run();
}
```

Hot boundaries live in `cdda_app` as thin wrappers:

```rust
// cdda_app/src/hot_boundaries.rs

#[hot]
pub fn run_ai_phase(world: &mut World, rng: &mut SeededRng) {
    cdda_sim::systems::ai::think_phase(world, rng);
}

#[hot]
pub fn run_combat_phase(world: &mut World) {
    cdda_sim::systems::combat::resolve_phase(world);
}

#[hot]
pub fn run_movement_phase(world: &mut World) {
    cdda_sim::systems::movement::resolve_phase(world);
}
```

When logic in `cdda_sim` changes, the dependent wrapper in `cdda_app` is
recompiled by ThinLink and hot-patched by subsecond. Workspace hot-patching
(merged Feb 2026) tracks the full dependency DAG.

### 8.4 Hot-Reload Constraints and Limitations

Subsecond is experimental. Documented constraints relevant to this architecture:

- Workspace limitation: bevy_simple_subsecond_system does not support workspace setups. The upstream Bevy integration (merged June 2025) may lift this. Verify against Bevy 0.18.
- Only functions that exist at launch can be patched. Adding a brand-new function requires a cold restart. T3 supports modifying existing functions, not adding new ones.
- All hotpatched systems run as exclusive systems. Not an issue for cdda_sim single-threaded tick loop.
- Struct layout changes cause UB. Adding or removing fields requires a restart. See Section 8.5.
- No new statics or global initializers. Changes to static initializers are not observed.

### 8.5 Component Modification Discipline

Subsecond cannot handle struct layout changes (UB risk). For hot-reload
compatibility during development:

- **Prefer adding new components** over modifying existing struct fields.
- **Tweak numbers in JSON** (T1) for balance; change logic in Rust (T3) for behavior.
- **Struct layout changes require a restart.** Acceptable — these are rare once
  components stabilize.

```rust
// Instead of adding a field to Weapon (struct layout change → restart):
// pub struct Weapon { ..., bleed_chance: f32 }  ← DON'T

// Add a new component (no layout change to existing types):
#[derive(Component, Reflect)]
pub struct WeaponBleedChance(pub f32);  // ← hot-reload safe
```

### 8.6 Development Profiles

```toml
# Cargo.toml
[profile.dev-fast]
# For use with dx serve --hotpatch
opt-level = 1          # slightly optimized, still thin-linkable
debug = 1              # line numbers only
lto = false
codegen-units = 256    # maximum parallelism
incremental = true
```

```bash
# Full hot-patch: code AND assets
dx serve --hotpatch --profile dev-fast

# Fast compile (ThinLink, but process restart on change)
dx serve --profile dev-fast

# Standard Bevy dev
cargo run
```

---

## 9. Improved Crate Structure

```
cdda-rs/
├── Cargo.toml
├── .cargo/
│   └── config.toml
│
├── crates/
│   ├── cdda_core/          # Pure domain types. ZERO deps.
│   │   ├── src/
│   │   │   ├── id.rs           # DefIdx, GenId, per-category newtypes
│   │   │   ├── id_slab.rs      # IdSlab<T>: dense generational storage
│   │   │   ├── registry.rs     # DefRegistry: Vec<T> + name maps
│   │   │   ├── coords/         # Pos<S,O>, ZLevel, Direction, Facing
│   │   │   ├── units/          # Volume, Weight, Time, Energy, Length
│   │   │   │                   # NO serde — pure newtypes, const constructors
│   │   │   ├── damage.rs       # Damage struct
│   │   │   ├── stats.rs        # Stats struct
│   │   │   ├── flags.rs        # FlagSet
│   │   │   ├── rng.rs          # SeededRng
│   │   │   ├── templates/      # Component templates (pure, no serde)
│   │   │   │   ├── item.rs     # ItemBase, WeaponBehavior, ArmorBehavior, ...
│   │   │   │   ├── monster.rs  # MonsterBase, MonsterCombatStats, ...
│   │   │   │   ├── terrain.rs  # TerrainTemplate
│   │   │   │   ├── furniture.rs
│   │   │   │   ├── recipe.rs
│   │   │   │   └── ...
│   │   │   └── error.rs
│   │   └── tests/
│   │
│   ├── cdda_data/          # JSON IO, copy-from, THE ACL
│   │   ├── src/
│   │   │   ├── raw_defs/       # Raw CDDA JSON types (moved from cdda_core::defs)
│   │   │   │   ├── item.rs     # RawItemDef: serde-annotated, matches CDDA JSON
│   │   │   │   ├── monster.rs  # RawMonsterDef
│   │   │   │   ├── terrain.rs  # RawTerrainDef
│   │   │   │   ├── cdda_types.rs  # CddaColor, RawValue, UseAction, ... CDDA quirks
│   │   │   │   └── ...
│   │   │   ├── cdda_serde/     # Custom serde for CDDA string formats
│   │   │   │   ├── volume.rs   # "250 ml" ↔ Volume
│   │   │   │   ├── weight.rs   # "1 kg" ↔ Weight
│   │   │   │   ├── time.rs     # "30 m" ↔ Time
│   │   │   │   └── energy.rs   # "1 kJ" ↔ Energy
│   │   │   ├── loader.rs       # Two-pass JSON loader
│   │   │   ├── resolve.rs      # copy-from: extend/delete/relative/proportional
│   │   │   ├── translate.rs    # THE ACL: RawItemDef → ItemTemplate, etc.
│   │   │   ├── registry_builder.rs  # Build DefRegistry with numeric IDs
│   │   │   ├── mod_shard.rs    # Sharded ID space for mods
│   │   │   ├── mod_layer.rs    # Mod loading and layering
│   │   │   ├── hot_reload.rs   # File watcher + T1 data hot reload
│   │   │   └── save.rs         # Save/load serialization (numeric→string)
│   │   └── tests/
│   │       ├── loading.rs
│   │       ├── copy_from.rs
│   │       ├── translation.rs  # ACL correctness tests
│   │       ├── fuzz/           # cargo-fuzz targets
│   │       └── fixtures/
│   │
│   ├── cdda_sim/           # Simulation on bevy_ecs
│   │   ├── src/
│   │   │   ├── components/     # ECS component structs
│   │   │   ├── systems/        # ECS systems (thin orchestrators)
│   │   │   ├── events.rs       # DamageEvent, DeathEvent, SoundEvent, ...
│   │   │   ├── tick.rs         # Deterministic tick loop
│   │   │   ├── logic/          # Pure functions — NO ECS
│   │   │   │   ├── combat/
│   │   │   │   ├── ai/
│   │   │   │   ├── crafting/
│   │   │   │   ├── inventory/
│   │   │   │   ├── fsm/        # Pure-Rust state machine
│   │   │   │   └── ...
│   │   │   └── world_setup.rs  # Component registration
│   │   └── tests/
│   │
│   ├── cdda_map/           # Spatial layer — NO bevy
│   │   ├── src/
│   │   │   ├── tile_grid.rs    # BubbleGrid (SoA)
│   │   │   ├── item_store.rs   # Dense item store
│   │   │   ├── spatial.rs      # EntitySpatialIndex
│   │   │   ├── submap.rs       # Submap: 12×12 storage unit
│   │   │   ├── overmap.rs      # OvermapIndex (rstar)
│   │   │   ├── fov.rs
│   │   │   ├── pathfind.rs
│   │   │   ├── lighting.rs
│   │   │   ├── mapgen/
│   │   │   │   ├── canvas.rs
│   │   │   │   ├── executor.rs
│   │   │   │   ├── palette.rs
│   │   │   │   ├── split.rs
│   │   │   │   └── overmap_gen.rs
│   │   │   └── vehicle.rs
│   │   └── tests/
│   │
│   ├── cdda_render/        # Bevy rendering plugin
│   ├── cdda_input/         # Bevy input plugin
│   ├── cdda_audio/         # Bevy audio plugin
│   └── cdda_app/           # Binary + #[hot] boundaries
│       ├── src/
│       │   ├── main.rs
│       │   ├── state.rs
│       │   └── hot_boundaries.rs  # All #[hot] functions live here
│       └── Cargo.toml
│
├── data/
│   ├── core/               # Core game JSON (unchanged)
│   └── mods/               # Bundled mods
│
├── docs/
│   ├── CURRENT_ARCHITECTURE.md   # What's implemented now
│   ├── TARGET_ARCHITECTURE.md    # THIS FILE — where we're going
│   ├── coordinate-systems.md
│   ├── data-format.md
│   ├── save-format.md
│   └── contributing.md
│
└── tests/
    └── integration/
```

**Crate dependency graph:**

```
cdda_app  (binary + #[hot] boundaries)
    ├── cdda_render        [bevy full]
    ├── cdda_input         [bevy full]
    ├── cdda_audio         [bevy full]
    └── cdda_sim           [bevy_ecs, bevy_reflect]
            ├── cdda_map   [no Bevy]
            │       └── cdda_core  [ZERO deps]
            ├── cdda_data  [serde, serde_json, schemars]
            │       └── cdda_core
            └── cdda_core
```

---

## 10. Tick Architecture with Phase-Level Parallelism

### 10.1 Current: Fully Serial

```rust
pub fn tick(world: &mut World, rng: &mut SeededRng) {
    turn_order::system.run(world);
    ai::system.run(world);
    movement::system.run(world);
    combat::system.run(world);
    crafting::system.run(world);
    needs::system.run(world);
    effects::system.run(world);
    vehicles::system.run(world);
    spawning::system.run(world);
}
```

### 10.2 Target: Phase-Parallel with Deterministic Merging

```rust
pub fn tick(world: &mut World, rng: &mut SeededRng) {
    // Phase 1: Read-only AI evaluation — all entities, parallel
    // AI systems only READ world state, write to intent components
    schedule_ai.run(world);

    // Phase 2: Serial movement (prevents collisions)
    movement::sequential_system.run(world);

    // Phase 3: Data-parallel combat
    // Each combat pair is independent — resolve in parallel
    schedule_combat.run(world);

    // Phase 4: Serial effect application (order matters)
    effects::sequential_system.run(world);

    // Phase 5: Parallel spawning (each spawn is independent)
    schedule_spawning.run(world);
}
```

Bevy's change detection ensures no two systems in a schedule write to the same
entity simultaneously — it panics in debug if they do. Determinism is preserved
because phase boundaries are sequential.

**Deferred to a future stage.** Get the serial tick working and profiled first.
Only add parallelism if the single-threaded tick is too slow for large hordes.

For the deterministic serial tick, two API approaches are available:

1. **`system.run(&mut world)`** — raw `IntoSystem` calls. Simplest, no
   abstraction. Used in the current code.
2. **`Schedule` with `SingleThreadedExecutor`** — provides the same
   deterministic ordering with better ergonomics: systems are registered
   once and the schedule handles execution order. Better for maintainability
   as systems are added or reordered.

Choose based on ergonomics preference. Both produce identical runtime
behavior.

---

## 11. Profiling from Day One

```rust
// cdda_app/src/tick.rs

use tracing::{info_span, debug_span};

/// The tick function with tracing spans. Zero runtime cost in release
/// when compiled without tracing features.
#[hot]
fn game_tick(world: &mut World, rng: ResMut<SeededRng>, /* ... */) {
    let tick_span = info_span!("tick");
    let _tick_guard = tick_span.enter();

    {
        let _ai = debug_span!("ai").enter();
        run_ai_phase(world, &mut *rng);
    }
    {
        let _mv = debug_span!("movement").enter();
        run_movement_phase(world);
    }
    {
        let _com = debug_span!("combat").enter();
        run_combat_phase(world);
    }
    {
        let _eff = debug_span!("effects").enter();
        run_effects_phase(world);
    }
    {
        let _spw = debug_span!("spawning").enter();
        run_spawning_phase(world, &mut *rng);
    }
}
```

Hook to `tracy` or `chrome://tracing` for flame graphs. Every engineer can see
exactly where tick time goes.

---

## 12. Fuzzing the Data Pipeline

```rust
// cdda_data/tests/fuzz/ (cargo-fuzz targets)

/// Fuzz the copy-from resolver: generate random CDDA-style JSON with
/// arbitrary copy-from chains. Verify: never panics, terminates, produces
/// deterministic output.
#[fuzz]
fn fuzz_copy_from(data: &[u8]) {
    // Parse random bytes → JSON objects with copy-from relationships
    // Resolve → must not panic, must terminate
}

/// Fuzz the ACL translation: valid resolved raw defs must always
/// translate to valid domain types without panicking.
#[fuzz]
fn fuzz_translation(data: &[u8]) {
    // Parse → RawItemDef → translate → ItemTemplate must succeed
    // Unknown fields handled gracefully (RawValue)
    // Known fields with wrong types produce clear errors
}

/// Fuzz item group resolution: random item group JSON must
/// produce a probability distribution that sums correctly.
#[fuzz]
fn fuzz_item_groups(data: &[u8]) { /* ... */ }
```

---

## 13. Migration Path from Current to Target

The migration is ordered by dependency — each step unlocks the next:

| Step | What | Impact | Approx. Effort |
|------|------|--------|----------------|
| 1 | Strip serde from cdda_core units | Move serde impls to cdda_data::cdda_serde | ~200 lines |
| 2 | Move `defs/` from cdda_core to cdda_data::raw_defs | Rename module, fix imports | ~100 lines |
| 3 | Create numeric ID types in cdda_core | DefIdx, per-category newtypes | ~150 lines |
| 4 | Build string↔numeric maps in cdda_data::registry_builder | Populated during load | ~200 lines |
| 5 | Create component templates in cdda_core::templates | ItemTemplate, MonsterTemplate, etc. | ~500 lines |
| 6 | Build ACL in cdda_data::translate | RawItemDef → ItemTemplate translation | ~600 lines |
| 7 | Switch DefRegistry to Vec<T> + numeric IDs | Replace HashMap<String, T> with Vec<T> | ~300 lines |
| 8 | Update cdda_sim systems to use numeric IDs | Replace string lookups with index access | ~400 lines |
| 9 | Implement event-driven system communication | DamageEvent, DeathEvent, etc. | ~400 lines |
| 10 | Implement BubbleGrid (SoA tile storage) | Replace Vec<Tile> with SoA arrays | ~500 lines |
| 11 | Implement ModShard ID allocation | Sharded ID space for mods | ~300 lines |
| 12 | Implement three-tier hot reload | T1 (data), T2 (mapgen), T3 (subsecond) | ~500 lines |
| 13 | Implement incremental save via change detection | Changed<T> queries + WAL | ~400 lines |
| 14 | Add tracing spans to tick phases | info_span!/debug_span! | ~50 lines |
| 15 | Add fuzzing targets for data pipeline | cargo-fuzz harnesses | ~300 lines |
| 16 | Profile and apply Morton ordering if needed | Swap index function, benchmark | ~100 lines |
| 17 | Add phase-level parallelism | Bevy schedule per phase | ~400 lines (deferred) |

Steps 1–8 are the critical path and should be completed before Stage 2 simulation
work begins. Steps 9–12 can be concurrent with simulation development.
Steps 13–17 are optimization and hardening.

---

## 14. Design Principles (Extended)

These are added to the principles in `CURRENT_ARCHITECTURE.md`:

16. **The ACL is the only bridge.** CDDA JSON knowledge lives exclusively in
    `cdda_data`. No other crate depends on `serde_json`, `schemars`, or knows
    CDDA field names.

17. **Numeric IDs, not strings.** All cross-references at runtime are numeric.
    Strings exist only at the I/O boundary. Lookups are O(1) via Vec indexing.

18. **Components over subtyping.** An item isn't a type with optional fields —
    it's a set of ECS components. Bevy archetypes make sparse behavior efficient.

19. **Events over direct mutation.** Systems communicate through events. No
    system knows which other systems consume its outputs.

20. **Hot reload is a design constraint, not an afterthought.** The tick loop
    is structured for subsecond's stack-unwinding model. Data changes go through
    the ACL's fast path. Code changes respect component modification discipline.

21. **Cache-conscious data layout.** SoA for tiles. Morton ordering when
    profiling says so. Dense storage for items on ground. No per-tile heap
    allocations.

22. **Determinism enables safe retry.** The seeded RNG makes hot-patch retries
    converge to the same state. Replays are verifiable.

23. **Profile before optimizing.** `tracing` spans on every tick phase from
    day one. Decisions like Morton ordering or parallelism are data-driven.

---

## 15. Decision Table: Current → Target

| Current Architecture | Target Architecture |
|---|---|
| `DefId<T> { id: String }` | `ItemId(DefIdx(u32))`, concrete per-category types |
| `HashMap<String, Arc<T>>` in DefRegistry | `Vec<T>` indexed by `DefIdx.0` |
| `cdda_core::defs/` has serde, schemars, CDDA JSON shapes | `cdda_data::raw_defs/` owns those; `cdda_core` is pure |
| `cdda_types.rs` in cdda_core resolving CDDA JSON quirks | `cdda_data::raw_defs::cdda_types.rs` — isolated |
| Monolithic `ItemDef` with 60+ optional fields | `ItemTemplate` with composable `Option<Behavior>` |
| Copy-from machinery in `cdda_core::types` | Copy-from in `cdda_data::resolve` where it belongs |
| Unit types have custom CDDA serde in cdda_core | Custom serde in `cdda_data::cdda_serde`; pure newtypes in cdda_core |
| Tiles as `Vec<Tile>` (AoS, per-tile allocations) | `BubbleGrid` (SoA, dense arrays, zero per-tile allocs) |
| Direct `system.run()` serial tick (only option) | Phase-parallel with Bevy schedules (future) |
| No hot reload strategy | Three-tier: T1 data ACL, T2 mapgen replay, T3 subsecond |
| No save strategy beyond "serialize everything" | Incremental saves via `Changed<T>`, WAL, atomic writes |
| No spatial index for entities | `EntitySpatialIndex` (4×4 cell grid) |
| Mods merge into core namespace | Sharded ID space: core 0x0000, mods 0x8000 |
| No fuzzing | cargo-fuzz targets for copy-from, translation, item groups |
| No profiling infrastructure | tracing spans on every tick phase |
| cdda_mod is separate crate | cdda_mod integrated into cdda_data (mod_shard, mod_layer) |
