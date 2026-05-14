# Target Architecture — CDDA-BR

> **Last updated:** Planned migration roadmap from current state to target.
> See [CURRENT_ARCHITECTURE.md](CURRENT_ARCHITECTURE.md) for what's implemented now.

## Vision

CDDA-BR aims for:
1. **Maintainability** — small focused crates with clear dependency direction
2. **Performance** — SoA tile storage, deterministic tick, efficient spatial queries
3. **First-class modding** — hot-reloadable definitions, mod layering via ACL

## Decomposition Plan

### Phase 1 — Fix the God Functions (short-term)

**1. Decompose `build_def_world`**
- Current: ~900-line monolith spawning all definition types in one function
- Target: Per-category builder functions in separate modules:
  - `build_item_world()`, `build_monster_world()`, `build_terrain_world()`,
    `build_furniture_world()`, `build_recipe_world()`, `build_body_part_world()`
- Each builder takes `&DefRegistry` and returns `Vec<(String, Entity)>`

**2. DefRegistry decomposition**
- Current: single struct with ~100+ `HashMap` fields
- Target: Trait-based registry with per-category `Registry<T>` + global
  `RegistrySet` for combined queries

### Phase 2 — ID System Unification (medium-term)

**3. Unified ID system**
- Current: Three concurrent ID patterns
  - `DefStrId(String)` — stored as ECS component
  - `DefId<T>(String)` — type-safe generic wrapper
  - Interned tokens (`SkillId(u16)`, `BodyPartId(u16)`) — numeric
- Target: Single interned `DefIdx(u32)` backed by `Arc<str>` string table
  - Per-category concrete types via `def_id_type!` macro wrapping `DefIdx`
  - Zero-cost comparison, small copy size, string resolution on demand

**4. From `DefStrId` component to `DefOrigin` + `DefIdx`**
- Current: def entities use `DefStrId(String)` component
- Target: `DefOrigin(DefIdx)` — numeric index into DefRegistry
- Reduces string comparison to integer comparison for chunk/stack logic

### Phase 3 — Data Pipeline Simplification (medium-term)

**5. Clean separation of parsing vs entity spawning**
- Current: `cdda_data` crate handles both JSON deserialization AND
  Bevy entity construction (in `build_def_world`)
- Target: `cdda_parse` (pure JSON → typed AST, no Bevy dep) + `cdda_def_world`
  (AST → Bevy entities, Bevy ECS dep)

**6. Loader refactor**
- Current: `Loader` struct handles file I/O + type resolution + copy-from in
  a single ~1100-line file
- Target: Separate `FileScanner`, `TypeResolver`, `CopyFromResolver` structs

### Phase 4 — Runtime Architecture (long-term)

**7. SoA tile storage**
- Current: each tile is an ECS entity with components
- Target: chunk-based `Vec<T>` storage for terrain data, ECS entities only
  for dynamic objects (items, creatures)

**8. Event-driven simulation**
- Current: direct system calls in ordered sets
- Target: fully event-driven with observer-based triggers for all
  entity-to-entity interactions

**9. Three-tier hot reload**
- Current: no hot reload path exists
- Target: mod subsecond reload via three tiers:
  1. Tier 1: Asset file change → invalidate + reload changed files
  2. Tier 2: In-memory def tree patching (no new entities)
  3. Tier 3: Full def world rebuild (spawn/despawn entities)

## Design Principles

### Dependency Direction
```
cdda_core_types ← cdda_components ← game logic crates ← data/world crates ← app crates
```
No crate should depend on a crate above it in this hierarchy.

### No Circular Dependencies
If a circular dependency would form (e.g. `cdda_inventory` needs `cdda_actor`),
extract the shared types into a new crate (e.g. `cdda_inventory_types`).

### Registry Pattern
All dynamic registries implement `Registry<T>`:
```rust
pub trait Registry<T> {
    fn intern(&mut self, value: &str) -> T;       // returns a numeric ID
    fn resolve(&self, id: T) -> Option<&str>;      // resolves ID back to string
}
```
Where `T` is a newtype wrapper like `SkillId(2)`.

### No Vec<T> in components
If T has independent lifecycle (status effects, skills, mutations),
each instance is its own entity related via `#[relationship]`.
This enables granular change detection.

## Migration Status

| Item | Status |
|---|---|
| `build_def_world` decomposition | ❌ Not started |
| `DefRegistry` trait-ification | ❌ Not started |
| `DefId<T>` → `DefIdx` | ❌ Not started |
| `DefStrId` → `DefOrigin` | ❌ Not started |
| Bionic `active: bool` → `Active` tag | ✅ Done |
| MutationEntry `visible: bool` → `Visible` tag | ✅ Done |
| `StackCount::new` non-panicking | ✅ Done |
| `ParentPart` immutable | ✅ Done |
| Architecture docs created | ✅ Done |
