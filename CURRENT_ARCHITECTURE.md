# CDDA — Rust/Bevy Rewrite: Architecture & Project Structure

> A clean-room reimplementation of Cataclysm: Dark Days Ahead in Rust + Bevy ECS.
> Designed for maintainability, onboarding clarity, and comprehensive testability.
>
> This document is the canonical architecture reference. It lives in the repo and
> should be updated whenever a structural decision changes.
>
> **Target Bevy version: 0.18** (released 2026-01-13).

---

## What We Learned from the Original CDDA

Before designing anything, we did a thorough analysis of the original C++ codebase —
its genuine strengths, and the specific technical debts that have caused pain for
contributors over years. Everything in this architecture is a deliberate response
to one of those findings.

### Genuine Strengths to Preserve

**The JSON data system is exceptional.** CDDA's decision to put nearly all game
content — items, monsters, terrain, buildings, recipes, factions — in JSON files
has proven enormously successful. Over 2,000 contributors have added content without
touching C++. The modding system, built on the same JSON format as the core game,
is a direct consequence of this. Our architecture preserves and deepens this commitment.

**`copy-from` inheritance is powerful.** The ability for a definition to inherit
from a base and override only specific fields keeps the item database compact and
consistent. The pain is in debugging when it fails — our two-pass loader with
fixture-based tests and clear error messages addresses exactly this.

> **Important:** `copy-from` in CDDA is not just field merging. It interacts with
> `extend`/`delete` (for arrays and flag sets), `relative` modifications (e.g. a
> weight bump expressed as a delta), and `proportional` modifications. It also has
> "abstract" base types that are not valid game objects on their own. The loader
> must model *operations-on-fields*, not just inheritance. See `cdda_data/src/resolve.rs`
> and the full details in the CDDA modding docs.

**The mapgen palette system is clever.** CDDA's mapgen uses shared palettes
(symbol → terrain/furniture mappings) that multiple buildings reference. The
`standard_domestic_palette` alone covers most house furniture placement, saving
enormous duplication. Our `cdda_map` crate replicates this pattern faithfully.

**Item groups for loot spawning are well-designed.** The layered item group system
(nested groups, probability tables, count ranges) is flexible enough to express
everything from "trash in a bin" to "pharmacist's personal stash". We preserve
this model directly in `ItemGroupDef`.

**The units system.** CDDA's C++ `units.h` provides type-safe `units::mass`,
`units::volume`, etc. with literal syntax (`3_kilograms`). This catches entire
classes of bugs at compile time. Our `cdda_core` gives us equivalent newtypes
from day one.

**The coordinate type refactor is the right direction.** CDDA has been adding
`tripoint_bub_ms`, `tripoint_abs_omt`, etc. as distinct types to prevent mixing
local and global coordinate systems. We start with this discipline built in rather
than bolting it on. The ongoing C++ migration has confirmed that untyped coordinates
are one of the deepest sources of bugs in the codebase — the fix has been in
progress since 2014.

> **Note on CDDA's coordinate system:** The refactor has two independent axes that
> both matter — *scale* (ms/sm/omt/om) and *origin* (abs/bub/rel/veh). CDDA
> issue #71852 documents how conflating these causes bugs just as bad as the original
> untyped `tripoint`. Our coordinate types encode both axes. See the Coordinate
> System Reference section below.

**The modding system works.** Mods use the exact same JSON format as the core
game. Any content contribution could equivalently be a mod. This unification is
worth preserving architecturally, not just philosophically.

### Pain Points We Are Explicitly Solving

**`game.h` / `game_t` / `g->` global singleton.** The original codebase
acknowledges this in its own contributor guide: *"Global data access through
`g->` is deprecated to reduce overinclusion of the `game.h` header, which is
slowing compile times."* The `game::` class became a kitchen-sink mixing the main
loop, UI rendering, creature queries, player movement, and faction checks in one
file. Senior developers explicitly want it gone. Our architecture has no global
singleton — all state flows through ECS `Component`s and `Resource`s.

**`character.cpp` is roughly 13,000 lines.** The `Character` class accreted every
feature touching a creature over a decade. ECS components replace deep inheritance
with composable data.

**`player::` → `avatar::` migration debt.** We have no `Player` class — the player
is an entity with a `PlayerTag` marker component, treated identically to any other
character-type entity in the simulation layer.

**`is_player()` / `is_npc()` conditional branches everywhere.** ECS components
eliminate it entirely.

**Adding a monster special attack requires editing 4 separate files.** Our
architecture makes monster attacks fully data-driven from JSON — no code changes
needed for attacks that fit existing patterns.

**The item `charges` duality.** Our `CountMode` enum makes this explicit rather
than implicit. (Verified: CDDA has open issues documenting incorrect weight/volume
calculations stemming from charges-count confusion.)

**~29% test coverage (as of 2019 analysis), most tests requiring full game startup.**
Our architecture tests simulation logic as pure functions with `cargo test` in
milliseconds.

**All source files in a single flat `src/` directory.** Our 9-crate workspace
gives every domain a home obvious from its name.

**Inconsistent documentation with no reliable architectural overview.** This
document is our commitment to accuracy — it gets reviewed on every structural PR.

**Coordinate system confusion — scale AND origin.** CDDA's `point`/`tripoint` types
were untyped, causing subtle bugs where local coordinates were stored across turns
or passed to functions expecting global coordinates. The C++ fix encodes scale in
type names (`_ms_`, `_sm_`, `_omt_`) but also requires an origin marker (`_abs_`,
`_bub_`, `_rel_`) — both axes matter. Issue #71852 documents bugs that survived the
scale-only phase. We encode both from day one. We also use
`div_euclid`/`rem_euclid` everywhere coordinate arithmetic happens — Rust's `/`
truncates toward zero, which silently mis-assigns negative coordinates to the wrong
submap.

**Z-levels were retrofitted onto a flat 2D engine over many years.** CDDA started
as a flat game. Z-levels were added as an experimental opt-in in 2015, made default
in 2019, made mandatory in 2020, and 3D FOV was made default only in January 2024.
Each transition introduced bugs that persisted for years: creatures targeting through
floors ("z-level view violation"), NPCs stopping activities when on a different
z-level, vehicles being split across z-levels by diagonal ramp collisions, and
rendering failures at z-level boundaries. The entire class of bugs traces to systems
that were designed 2D and had z-awareness grafted on later. Our simulation layer
treats z as a first-class dimension in every system from day one — combat range
checks, FOV, pathfinding, and the renderer all speak full tripoint coordinates.

**Deferred JSON loading resolves forward references by re-scanning files
repeatedly.** Our two-pass loader is explicit: pass 1 ingests all raw defs, pass 2
topologically resolves `copy-from` chains with clear error messages on cycles.

**Non-humanoid body configurations retrofitted mid-development.** We start with a
dynamic `BodyParts` component rather than hardcoding a humanoid layout.

**Save/load was never designed in — it is bolted on.** CDDA's save system is
per-submap tile data plus per-entity state, with notorious mod compatibility issues.
We treat save/load as a first-class architectural concern from the start. See the
Save/Load Architecture section for the full approach.

**The save unit in CDDA is the submap (12×12 tiles), not the OMT (24×24 tiles).**
This distinction is critical. Mapgen runs on 24×24 OMT canvases, but load/unload
and serialization happen at 12×12 submap granularity. Our architecture preserves
this split with separate `Submap` (storage/simulation) and `MapgenCanvas`
(generation) types. Conflating them would make save-format compatibility
impossible and recreate the same scale-confusion that has cost CDDA a decade.

**The vehicle coordinate system is its own beast.** CDDA vehicles have their own
origin point, mount coordinates (relative to origin, facing due east), and map
square coordinates (accounting for current facing via rotation + shearing). Parts
are mounted at `(dx, dy)` relative to vehicle origin; multiple parts can occupy one
mount point (one external, others internal). The vehicle keeps precalculated arrays
(`precalc_*[0]` for current facing, `precalc_*[1]` for next move) to avoid
recomputing rotations per tick. Our `Rel` origin type accommodates this; vehicle
positions use `Pos<Ms, Rel>` with the vehicle entity as reference frame.

**The inventory pocket system is deeply nested.** CDDA's inventory overhaul replaced
the flat "amalgam inventory" with per-item pockets: every container has pockets with
independent volume/weight limits, flags (`CONTAINER`, `MAGAZINE`, `HOLSTER`), and
type-specific constraints. Items can be nested arbitrarily (a pot inside a backpack
inside a duffel bag). The UI must traverse this tree. Our `Inventory` component
models this as a tree of `Pocket` structs from day one; the rendering layer provides
recursive browsing.

---

## Design Principles

These are direct responses to the research above, not abstract ideals.

1. **Crates over files.** Each domain lives in its own crate. You should understand
   a crate's responsibility from its name and `lib.rs` exports alone.

2. **No global singleton. Ever.** The `g->` pattern is CDDA's worst architectural
   debt. All state flows through ECS `Component`s and `Resource`s.

3. **`bevy_ecs` as the simulation data model; no Bevy scheduler.** `cdda_core`,
   `cdda_data`, `cdda_mod`, and `cdda_map` have zero Bevy dependency. `cdda_sim`
   depends on `bevy_ecs` and `bevy_reflect` as pure data libraries — it uses them
   for component storage, archetypal queries, change detection, and reflection.
   Full Bevy ECS types (`Query`, `Commands`, `Entity`, `Component`, `Resource`,
   `MessageReader`, `MessageWriter`, `Observer`, `Relationship`, etc.) are used
   freely within `cdda_sim`. However, `cdda_sim` **never** uses the Bevy `App`,
   `Schedule`, or any system that depends on `winit`, windowing, or rendering.
   All simulation systems run in a fixed, deterministic order via manual
   `system.run(&mut world)` calls. The full `bevy` crate (with default features,
   scheduler, and rendering) is a dependency only of `cdda_render`,
   `cdda_input`, `cdda_audio`, and `cdda_app`.

4. **Composition over inheritance.** No `Character` god-class, no `is_player()`
   checks. Behavior comes from components an entity carries.

5. **Systems are thin orchestrators; logic is pure.** Bevy ECS systems in `cdda_sim`
   handle world queries, entity spawning, and component mutation. They delegate all
   computation to pure functions in `logic/` that accept plain Rust types
   (`ActorState`, `CombatIntent`, etc.). These pure functions are unit-tested with
   `cargo test` and never touch a `World`. This preserves fast, isolated testing
   for all game rules while letting the ECS handle entity management efficiently.

6. **Data is not logic.** JSON defs are read-only reference data. ECS components
   hold instance state.

7. **New content touches one place.** Adding a monster attack, an item flag, or a
   terrain type should require editing exactly one file.

8. **Typed coordinates from day one: both scale and origin, including z.** Coordinate
   types are parameterized on both scale (`Ms`, `Sm`, `Omt`, `Om`) and origin
   (`Abs`, `Bubble`, `Rel`). Types with different scale or origin do not coerce.
   The z-axis is a first-class member of every coordinate type. All horizontal
   coordinate arithmetic uses `div_euclid`/`rem_euclid` to correctly handle negative
   values.

9. **Z is always absolute.** The z-coordinate does not scale with horizontal unit
   changes. Converting `WorldPos` → `SubmapPos` strips z from the local offset and
   carries it unchanged. Z is stored as `ZLevel(i8)` — a newtype with checked
   arithmetic to prevent silent overflow.

10. **Modding is first-class.** Mods use identical JSON format to core data.

11. **Every simulation Component derives Reflect.** This is required for save/load
    and for `bevy-inspector-egui` debugging. It is not optional. Adding a component
    without `Reflect` causes it to silently drop out of saves. The 7-step mechanic
    checklist enforces this. For Bevy 0.18, use `#[reflect(Component)]` syntax
    (parentheses only, not brackets).

12. **The simulation tick is deterministic and single-threaded.** `cdda_sim::tick()`
    runs simulation systems in a fixed, explicit order via
    `system.run(&mut world)` (Bevy 0.18's `IntoSystem` trait). Bevy's parallel
    scheduler is never used for simulation. An alternative worth considering is
    `Schedule` with `SingleThreadedExecutor` — it gives the same deterministic
    ordering with better API ergonomics for registering and reordering systems.
    No systems touch `Res<Time>`, `std::time`, or unseeded randomness. This
    makes replays, crash reproduction, and potential future network
    synchronization tractable.

13. **Save atomicity is a first-class concern.** Chunk writes use
    write-to-temp-then-rename. An interrupted save never leaves the world in a
    partially-written state. See the Save/Load Architecture section.

14. **Model/View separation for save/load.** Rendering components (sprites, tilemap
    data, camera settings, UI state) are never saved. They are reconstructed from
    simulation state on load. Simulation components are the singular source of
    truth.

15. **State-scoped entity cleanup.** Entities that belong to a specific game state
    (e.g. UI entities for the main menu, world-gen preview entities) carry Bevy's
    `StateScoped<S>` component so they are automatically despawned on state exit.
    This prevents stale-entity bugs and eliminates manual cleanup systems.

---

## Workspace Layout

```
cdda-rs/
├── Cargo.toml                     # workspace root — all shared dep versions here
├── Cargo.lock
├── .cargo/
│   └── config.toml                # dev: dynamic_linking (Linux/macOS only), fast linker (mold/zld)
│
├── docs/
│   ├── architecture.md            # THIS FILE — reviewed on every structural PR
│   ├── contributing.md            # PR process, commit format, review expectations
│   ├── onboarding.md              # "first PR in 30 minutes" guide
│   ├── data-format.md             # copy-from (incl. extend/delete/relative/proportional),
│   │                             # item groups, mapgen
│   ├── coordinate-systems.md     # Pos<Scale, Origin> design with diagrams.
│   │                             # Covers ZLevel newtype, div_euclid requirement,
│   │                             # the submap vs. OMT distinction, and vehicle coords.
│   └── save-format.md            # versioned save wire format, migration strategy,
│                                 # atomicity guarantees
│
├── crates/
│   ├── cdda_core/                 # Pure types: coords, units, damage, calendar. No Bevy.
│   ├── cdda_data/                 # JSON loading, def registry, copy-from resolver. No Bevy.
│   ├── cdda_mod/                  # Mod loading, mod layering, conflict detection. No Bevy.
│   ├── cdda_sim/                  # Simulation: components, systems, logic, tick loop
│   │                             # Depends: bevy_ecs, bevy_reflect. NOT bevy (full).
│   ├── cdda_map/                  # Map storage, mapgen, FOV, pathfinding. No Bevy.
│   ├── cdda_render/               # Bevy rendering plugin (tiles, UI, ASCII mode)
│   ├── cdda_input/                # Bevy input plugin (keybinds, input contexts)
│   ├── cdda_audio/                # Bevy audio plugin
│   └── cdda_app/                  # Binary: App::new() + plugin registration only
│
├── data/
│   ├── core/                      # Core game JSON (mirrors CDDA data/json layout)
│   │   ├── items/
│   │   ├── monsters/
│   │   ├── terrain/
│   │   ├── furniture/
│   │   ├── recipes/
│   │   ├── item_groups/
│   │   ├── mapgen/
│   │   ├── mapgen_palettes/
│   │   ├── overmap/
│   │   ├── fields/
│   │   ├── vehicle_parts/
│   │   ├── mutations/
│   │   ├── bionics/
│   │   ├── effects/
│   │   ├── factions/
│   │   └── scenarios/
│   └── mods/                      # bundled optional mods (same format as core/)
│
└── tests/
    └── integration/
        ├── data_loading.rs
        ├── copy_from.rs
        ├── extend_delete.rs        # copy-from with extend/delete/relative operations
        ├── item_groups.rs
        ├── crafting.rs
        ├── combat_round.rs
        ├── combat_z_level.rs
        ├── map_gen.rs
        ├── mapgen_z_levels.rs
        └── mod_loading.rs
```

---

## Crate Dependency Graph

```
cdda_app  (binary)
    ├── cdda_render        [bevy = "0.18", full]
    ├── cdda_input         [bevy = "0.18", full]
    ├── cdda_audio         [bevy = "0.18", full]
    └── cdda_sim           [bevy_ecs = "0.18", bevy_reflect = "0.18"]
            ├── cdda_map   [no Bevy]
            ├── cdda_mod   [no Bevy]
            └── cdda_data  [no Bevy]
                    └── cdda_core  [no Bevy]
```

`cdda_core` has no game-domain dependencies. Everything flows upward.

**The dependency firewall:**
- **`cdda_core`, `cdda_data`, `cdda_mod`, `cdda_map`** — zero Bevy dependency of any kind.
- **`cdda_sim`** — depends on `bevy_ecs` and `bevy_reflect` as standalone data libraries.
  Uses `World`, `Entity`, `Component`, `Query`, `Commands`, `Resource`,
  `MessageReader`, `MessageWriter`, `Observer`, `Relationship`, change detection,
  component hooks, and all other `bevy_ecs` types and features.
  Does **NOT** depend on `bevy` (the full crate), `App`, `Schedule`, `winit`,
  rendering, windowing, asset loading, or the parallel executor.
- **`cdda_render`, `cdda_input`, `cdda_audio`, `cdda_app`** — full `bevy` dependency,
  including scheduler, rendering, windowing, input, and audio.

**Why this split works:**
`bevy_ecs` is a pure data library — it provides component storage, archetypal queries,
change detection, and entity management with no dependency on `winit`, rendering,
windowing, or threading. It compiles to a library that does data storage and querying
and nothing more. The full `bevy` crate adds the scheduler (`App`, `Schedule`),
parallel executor, and render-dependency — those are what make systems non-deterministic
and entangled with the frame lifecycle. By using `bevy_ecs` directly and running
systems manually, `cdda_sim` gets the full power of archetypal ECS while staying
frame-independent, deterministic, and headless-testable.

---

## Crate Details

---

### `cdda_core` — Pure types and units

**Job:** Types and pure functions shared by all other crates. No IO, no Bevy,
no game-domain logic.

```
crates/cdda_core/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   │
│   ├── coords/
│   │   ├── mod.rs
│   │   ├── scales.rs       # Scale marker types: Ms, Sm, Omt, Om
│   │   ├── origins.rs      # Origin marker types: Abs, Bubble, Rel
│   │   ├── pos.rs          # Pos<Scale, Origin> { x: i32, y: i32, z: ZLevel }
│   │   │                   # Type aliases:
│   │   │                   #   WorldPos       = Pos<Ms, Abs>
│   │   │                   #   SubmapPos      = Pos<Sm, Abs>
│   │   │                   #   SubmapLocal    = Pos<Ms, Rel>   (offset 0..=11)
│   │   │                   #   BubblePos      = Pos<Ms, Bubble>
│   │   │                   #   OmtPos         = Pos<Omt, Abs>
│   │   │                   #   OmPos          = Pos<Om, Abs>
│   │   │                   #   VehicleMountPos = Pos<Ms, Rel>
│   │   │                   #   VehicleMapPos   = Pos<Ms, Rel>
│   │   ├── z_level.rs      # ZLevel(i8) newtype with checked_add/sub
│   │   │                   # Range [-10, 10] enforced on construction.
│   │   │                   # CDDA docs confirm: "z-coordinates do not scale
│   │   │                   # along with the horizontal dimensions."
│   │   └── direction.rs    # Direction (N/S/E/W/NE/…), rotation helpers
│   │
│   ├── units/
│   │   ├── mod.rs
│   │   ├── volume.rs
│   │   ├── weight.rs
│   │   ├── time.rs
│   │   └── energy.rs
│   │
│   ├── damage.rs
│   ├── flags.rs
│   ├── stats.rs
│   ├── rng.rs              # Seeded deterministic RNG — reproducible tests and replays
│   └── error.rs
│
└── tests/
    ├── coords.rs           # round-trips, distance, direction math, negative-coord cases,
    │                       # ZLevel checked arithmetic, scale/origin type mismatch (compile-fail)
    ├── units.rs
    ├── calendar.rs
    └── damage.rs
```

> **Known issue:** The current `cdda_core` also contains `defs/` (game definition
> structs with `serde` and `schemars` derives) and `types/` (including `copy-from`
> machinery). This violates the "pure types, no IO" principle. The
> `TARGET_ARCHITECTURE.md` addresses this by moving `defs/` to
> `cdda_data::raw_defs/` and stripping all IO dependencies from `cdda_core`.

#### Coordinate design: two axes of typing

Every coordinate has two independent type parameters:

- **Scale** — what one unit represents: `Ms` (map square), `Sm` (submap = 12×12 tiles),
  `Omt` (overmap terrain = 2×2 submaps = 24×24 tiles), `Om` (overmap = 180×180 omts).
- **Origin** — what position (0, 0) means: `Abs` (global, never shifts),
  `Bubble` (relative to reality-bubble top-left corner), `Rel` (generic relative —
  used for vehicle-relative coordinates, submap-local offsets, and delta calculations).

CDDA's own coordinate documentation defines the same axes. Issue #71852 documents
surviving bugs where two coordinates with the same scale but different semantic
origins were mixed. We encode both axes from day one. Functions that accept a
position declare exactly which `Pos<Scale, Origin>` they need; the types do not
coerce to each other.

```rust
// Type aliases defined in cdda_core::coords
pub type WorldPos        = Pos<Ms,  Abs>;     // absolute map-square position
pub type SubmapPos       = Pos<Sm,  Abs>;     // which 12×12 submap
pub type SubmapLocal     = Pos<Ms,  Rel>;     // offset within submap, 0..=11 on x and y
pub type BubblePos       = Pos<Ms,  Bubble>;  // position within the reality bubble
pub type OmtPos          = Pos<Omt, Abs>;     // overmap terrain position (1 unit = 24×24 tiles)
pub type OmPos           = Pos<Om,  Abs>;     // overmap position (1 unit = 180×180 omts)

// Vehicle coordinates
pub type VehicleMountPos = Pos<Ms, Rel>;      // mount coords (facing east, relative to origin)
pub type VehicleMapPos   = Pos<Ms, Rel>;      // map square coords (accounting for facing)
```

#### Coordinate arithmetic rules

All horizontal coordinate division uses **`div_euclid` and `rem_euclid`**, never
`/` and `%`. Rust's `/` truncates toward zero: `-1 / 12 = 0`, which would assign
world position (-1, 0) to submap (0, 0) — the same submap as (0, 0). Euclidean
division gives `-1 / 12 = -1` (submap -1), which is correct. This is a real bug
class in CDDA and we must never introduce it.

The z-coordinate **does not participate in horizontal scale conversions.**
CDDA's POINTS_COORDINATES.md confirms: "z-coordinates do not scale along with the
horizontal dimensions, so z values are unchanged by project_to and project_remain."

> **Note on `SubmapLocal` and z:** `SubmapLocal` stores `z: ZLevel` alongside `x`
> and `y`. Since z does not participate in horizontal scaling, this is a convenience
> copy of the same absolute z-value already present in the `SubmapPos`/`WorldPos`.
> It is stored here so that conversion back to `WorldPos` does not require fetching
> z from an external source.

```rust
impl WorldPos {
    /// Returns the containing submap and the offset within it.
    pub fn to_submap(self) -> (SubmapPos, SubmapLocal) {
        let sx = self.x.div_euclid(12);
        let sy = self.y.div_euclid(12);
        let lx = self.x.rem_euclid(12);
        let ly = self.y.rem_euclid(12);
        (
            SubmapPos { x: sx, y: sy, z: self.z },
            SubmapLocal { x: lx, y: ly, z: self.z },
        )
    }

    pub fn from_submap(sm: SubmapPos, local: SubmapLocal) -> Self {
        WorldPos {
            x: sm.x * 12 + local.x,
            y: sm.y * 12 + local.y,
            z: sm.z,  // z passes through unchanged
        }
    }

    /// 1 OmtPos = 2×2 submaps = 24×24 world tiles.
    pub fn to_omt(self) -> OmtPos {
        OmtPos {
            x: self.x.div_euclid(24),
            y: self.y.div_euclid(24),
            z: self.z,  // z unchanged
        }
    }
}
```

---

### `cdda_data` — JSON loading and def registry

**Job:** Parse CDDA JSON files into typed Rust structs. Resolve `copy-from`
inheritance (including `extend`/`delete`/`relative`/`proportional` operations).
Expose `DefRegistry` as the single authoritative read-only store of all game
definitions.

Two-pass explicit loader instead of CDDA's repeated-scan strategy. Pass 1 ingests
all raw `serde_json::Value`s keyed by `"type"` field. Pass 2 topologically sorts by
`copy-from` dependency and resolves in order, applying field operations in sequence.
Circular references are caught in pass 2 and reported with the full chain. Abstract
base types (those with `"abstract": true`) are resolved but not inserted into the
final registry.

`DefId<T>` typed IDs instead of bare `String`. `DefId<ItemDef>` and
`DefId<MonsterDef>` are incompatible at compile time.

`CountMode` enum addresses the `charges` duality explicitly. (CDDA has open issues
documenting incorrect weight/volume calculations from charges-count confusion.)

```
crates/cdda_data/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── registry.rs
│   ├── loader.rs
│   ├── resolve.rs          # Two-pass copy-from: handles extend/delete/relative/proportional
│   ├── mod_layer.rs
│   │
│   ├── defs/
│   │   ├── mod.rs
│   │   ├── item.rs
│   │   ├── monster.rs
│   │   ├── terrain.rs
│   │   ├── furniture.rs
│   │   ├── recipe.rs
│   │   ├── item_group.rs
│   │   ├── mapgen.rs       # MapgenDef, MapgenPalette — see mapgen pipeline below
│   │   ├── overmap_terrain.rs
│   │   ├── field.rs
│   │   ├── vehicle_part.rs
│   │   ├── mutation.rs
│   │   ├── bionic.rs
│   │   ├── effect.rs
│   │   ├── faction.rs
│   │   └── scenario.rs
│   │
│   └── types/
│       ├── mod.rs
│       ├── localized.rs
│       ├── id.rs
│       └── flags.rs
│
└── tests/
    ├── item_parsing.rs
    ├── copy_from.rs
    ├── extend_delete.rs    # extend/delete/relative/proportional field operations
    ├── abstract_types.rs   # abstract bases not in final registry
    ├── item_groups.rs
    ├── charge_duality.rs
    ├── mapgen_palette.rs
    └── fixtures/
```

**Key types:**

```rust
pub struct DefRegistry {
    pub items:       HashMap<DefId<ItemDef>,          Arc<ItemDef>>,
    pub monsters:    HashMap<DefId<MonsterDef>,        Arc<MonsterDef>>,
    pub terrain:     HashMap<DefId<TerrainDef>,        Arc<TerrainDef>>,
    pub furniture:   HashMap<DefId<FurnitureDef>,      Arc<FurnitureDef>>,
    pub recipes:     HashMap<DefId<RecipeDef>,         Arc<RecipeDef>>,
    pub item_groups: HashMap<DefId<ItemGroupDef>,      Arc<ItemGroupDef>>,
    pub mapgen:      HashMap<DefId<OmtDef>,            Vec<Arc<MapgenDef>>>,
    pub palettes:    HashMap<DefId<MapgenPaletteDef>,  Arc<MapgenPaletteDef>>,
}

pub enum CountMode {
    Single,
    ByCount  { default: u32, max: Option<u32> },
    Charges  { default: u32, max: Option<u32> },
}

pub struct ItemDef {
    pub id:         DefId<ItemDef>,
    pub name:       LocalizedString,
    pub volume:     Volume,
    pub weight:     Weight,
    pub count_mode: CountMode,
    pub flags:      FlagSet,
    // ...
}
```

---

### `cdda_mod` — Mod loading and layering

**Job:** Load mods, resolve their dependency order, apply their definitions on
top of the core registry. Detect conflicts. Expose a fully-resolved `DefRegistry`.

```
crates/cdda_mod/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── manifest.rs
│   ├── resolver.rs
│   ├── loader.rs
│   ├── merge.rs
│   └── conflict.rs
│
└── tests/
    ├── mod_loading.rs
    ├── mod_dependency_sort.rs
    ├── mod_conflict.rs
    └── fixtures/
```

---

### `cdda_sim` — Simulation logic

**Job:** Turn engine, combat, crafting, AI, needs, status effects, mutations,
bionics, vehicles, inventory. Three sub-layers: `components` (EC-entity data structs),
`systems` (ECS systems that query and mutate the `World`), `logic` (pure functions —
the testable core that never touches ECS types).

> **This crate depends on `bevy_ecs` and `bevy_reflect`.** It uses `World`, `Entity`,
> `Component`, `Query`, `Commands`, `Resource`, `MessageReader`, `MessageWriter`,
> `Observer`, `Relationship`, component hooks, change detection, and all other
> `bevy_ecs` features. It does **NOT** depend on `bevy` (the full crate), `App`,
> `Schedule`, `winit`, or rendering. Systems are run manually in a fixed, explicit
> order. This gives us the full power of archetypal ECS while preserving
> determinism, decoupling from the frame lifecycle, and keeping the simulation
> headless-testable.

#### The ECS design rationale

CDDA's simulation is an archetypal ECS problem: thousands of entities (creatures,
items, vehicles, fields) with heterogeneous components that need to be efficiently
queried. Building a hand-rolled component storage with HashMaps would be slower,
harder to maintain, and would require implementing change detection, hooks, and
relationships from scratch. `bevy_ecs` provides all of this as a pure data library
with no rendering or threading dependency.

The key insight is that **determinism comes from how you schedule execution, not
from what data structure you use.** `bevy_ecs` queries are deterministic when you
own the `World` exclusively and iterate in order. Bevy's parallel scheduler is what
introduces non-determinism — and `cdda_sim` does not use it.

All simulation components use `#[derive(Component, Reflect, Serialize, Deserialize)]`
with `#[reflect(Component)]`. They are registered in the world during tick setup.

#### Turn scheduling — deterministic, single-threaded

The simulation tick runs inside a single Bevy system in `cdda_app` that calls
`cdda_sim::tick()` once per game turn. Inside `tick()`, each simulation system
is executed sequentially via `system.run(&mut world)` — a plain Rust call with
no Bevy `Schedule` involved. An alternative worth considering is `Schedule` with
`SingleThreadedExecutor` for better API ergonomics. The order is fixed and documented.

```rust
// cdda_sim/src/tick.rs
pub fn tick(world: &mut World, rng: &mut Rng) {
    // Systems run in this exact order, every tick. No parallelism.
    turn_order::system.run(world);
    ai::system.run(world);
    movement::system.run(world);
    combat::system.run(world);
    crafting::system.run(world);
    needs::system.run(world);
    effects::system.run(world);
    vehicles::system.run(world);
    spawning::system.run(world);
    // ... any additional systems
}
```

Systems are thin orchestrators:

```rust
// cdda_sim/src/systems/combat.rs
pub fn system(world: &mut World) {
    let mut combat_pairs: Vec<(Entity, Entity, MeleeIntent)> = Vec::new();
    
    // Archetypal ECS query — iterates contiguous archetype memory efficiently
    let mut query = world.query::<(Entity, &MeleeIntent, &Position)>();
    for (attacker, intent, pos) in query.iter(world) {
        if let Some(target) = find_target(world, pos, intent.target_pos) {
            combat_pairs.push((attacker, target, intent.clone()));
        }
    }
    
    // All computation delegated to pure functions — no World access
    for (attacker, target, intent) in combat_pairs {
        let attacker_stats = build_actor_state(world, attacker);
        let defender_stats = build_actor_state(world, target);
        let result = logic::combat::melee::resolve(&attacker_stats, &defender_stats, &intent);
        apply_combat_result(world, attacker, target, &result);
    }
}
```

The logic functions are pure, tested without any ECS:

```rust
// cdda_sim/src/logic/combat/melee.rs
/// Pure function — unit-testable with cargo test, no World or Query needed.
pub fn resolve(attacker: &ActorState, defender: &ActorState, intent: &MeleeIntent) -> CombatResult {
    let hit_chance = hit_chance(attacker.melee_skill, defender.dodge_skill);
    let damage = if roll_hit(hit_chance, intent.rng_roll) {
        let base = base_damage(attacker.strength, intent.weapon_damage);
        let mitigated = armor::mitigate(base, defender.armor);
        critical_roll(mitigated, attacker.melee_skill, intent.rng_crit)
    } else {
        0
    };
    CombatResult { damage, hit: damage > 0 }
}
```

#### AI architecture

Monster and NPC behavior uses a **utility AI** module implemented in
`cdda_sim/logic/ai/`. Rather than depending on an external crate, the AI layer
is a small, purpose-built module: a `Scorer` trait, an `Action` enum, and a
`Thinker` struct that selects actions based on numeric scores. This is approximately
200 lines of pure Rust and has no Bevy dependency, making it independently testable.

> **Why not `big-brain`?** `big-brain` was archived by its maintainer in October 2025
> and is no longer maintained. Additionally, its design runs Scorer/Action evaluation
> every frame — a real-time pattern mismatched with a turn-based simulation where
> off-screen creatures should evaluate AI on a turn tick, not at 60 Hz.

The same AI systems run over all entity types without `is_player()` branching — a
monster, an NPC companion, and a wild animal all use the same Thinker mechanism.

#### Finite State Machine — pure-Rust implementation

Player and NPC **state** (climbing, swimming, driving, crafting, stunned) uses a
**pure-Rust finite state machine** implemented in `cdda_sim/logic/fsm/`. This
provides states, transitions, entry/exit callbacks in ~150 lines of pure Rust.

> **Why a custom FSM instead of `seldom_state`?** `seldom_state` is a Bevy plugin
> with a full Bevy dependency (it operates on Bevy entities with Bevy components
> and Bevy system schedules). Its state machine logic would work inside `cdda_sim`
> only if we used it as a component-storage library without the scheduler — which
> is essentially what the custom FSM does with less code and no dependency.
> `seldom_state` remains available for UI-level state management inside
> `cdda_render` if needed.

```rust
// cdda_sim/src/logic/fsm/mod.rs (sketch)
pub struct StateMachine<S: State> {
    current: S,
    transitions: Vec<Transition<S>>,
}

pub trait State: Clone + PartialEq {
    fn on_enter(&self, entity: &mut EntityState) {}
    fn on_exit(&self, entity: &mut EntityState) {}
}

impl<S: State> StateMachine<S> {
    pub fn tick(&mut self, entity: &mut EntityState) {
        for transition in &self.transitions {
            if transition.trigger.evaluate(entity) && transition.from == self.current {
                self.current.on_exit(entity);
                self.current = transition.to.clone();
                self.current.on_enter(entity);
                break;
            }
        }
    }
}
```

```
crates/cdda_sim/
├── Cargo.toml              # bevy_ecs = "0.18", bevy_reflect = "0.18"
│                           # serde, thiserror, tracing
│                           # DOES NOT DEPEND ON bevy (full)
├── src/
│   ├── lib.rs
│   │
│   ├── components/             # ECS component structs — #[derive(Component, Reflect)]
│   │   ├── mod.rs
│   │   ├── actor.rs            # Health, Stamina, ActionPoints, Speed
│   │   ├── body.rs             # BodyParts — dynamic, not hardcoded humanoid
│   │   ├── position.rs         # Position(WorldPos)
│   │   ├── wounds.rs
│   │   ├── inventory.rs        # Inventory tree: pockets, containers, nesting
│   │   ├── skills.rs
│   │   ├── needs.rs
│   │   ├── effects.rs
│   │   ├── mutations.rs
│   │   ├── bionics.rs
│   │   ├── faction.rs
│   │   ├── vehicle.rs          # Vehicle state, part health, fuel, velocity
│   │   ├── fsm.rs              # StateMachine<S> component (current state, transitions)
│   │   └── tags.rs             # PlayerTag, NpcTag, MonsterTag — no is_player()
│   │
│   ├── systems/                # ECS systems — thin orchestrators, run via system.run(&mut world)
│   │   ├── mod.rs
│   │   ├── turn_order.rs
│   │   ├── ai.rs               # calls logic::ai
│   │   ├── movement.rs
│   │   ├── combat.rs           # Query → logic::combat → write results. Z-aware.
│   │   ├── crafting.rs
│   │   ├── needs.rs
│   │   ├── effects.rs
│   │   ├── mutations.rs
│   │   ├── vehicles.rs         # Vehicle movement, collisions, part damage
│   │   ├── inventory.rs        # Pickup, drop, container insert/remove
│   │   └── spawning.rs
│   │
│   ├── tick.rs                 # THE DETERMINISTIC LOOP:
│   │                           # pub fn tick(world: &mut World, rng: &mut Rng) {
│   │                           #     turn_order::system.run(world);
│   │                           #     ai::system.run(world);
│   │                           #     movement::system.run(world);
│   │                           #     combat::system.run(world);
│   │                           #     // ... all systems in explicit order
│   │                           # }
│   │
│   ├── logic/                  # Pure functions — NO World, NO Query, NO ECS
│   │   ├── mod.rs
│   │   ├── combat/
│   │   │   ├── mod.rs
│   │   │   ├── melee.rs        # hit_chance(), melee_damage(), critical_roll()
│   │   │   │                   # range check: same z OR explicit floor opening
│   │   │   │                   # (TFLAG_NO_FLOOR on intervening tile)
│   │   │   ├── ranged.rs
│   │   │   └── armor.rs
│   │   ├── crafting.rs
│   │   ├── needs.rs
│   │   ├── effects.rs
│   │   ├── mutations.rs
│   │   ├── inventory.rs        # pocket insertion, weight/volume validation
│   │   ├── skills.rs
│   │   ├── fsm/
│   │   │   ├── mod.rs          # Pure-Rust state machine, ~150 lines
│   │   │   ├── types.rs        # State trait, Transition, Trigger
│   │   │   └── machine.rs      # StateMachine<S> implementation
│   │   └── ai/
│   │       ├── mod.rs
│   │       ├── scorer.rs       # Scorer trait + built-ins: ThirstScore, ThreatScore…
│   │       ├── action.rs       # Action enum + built-ins: Flee, Attack, Wander…
│   │       ├── thinker.rs      # Thinker: selects highest-scoring Action each turn
│   │       └── pathfind.rs     # A* adapter over cdda_map, z-aware neighbors
│   │
│   └── world_setup.rs          # Registration: world.register_component::<T>() for all components
│
└── tests/
    ├── tick_ordering.rs        # verify systems run in declared order
    ├── combat_melee.rs         # pure logic tests — no World
    ├── combat_ranged.rs
    ├── combat_armor.rs
    ├── combat_z_level.rs       # cross-z attacks blocked by solid floor
    ├── combat_integration.rs   # ECS integration test — spawn world, run combat system
    ├── crafting.rs
    ├── needs_tick.rs
    ├── effects.rs
    ├── inventory_weight.rs
    ├── inventory_nesting.rs    # nested container insertion/extraction
    ├── fsm_transition.rs       # state machine transitions
    └── ai_behaviour.rs
```

#### Inventory architecture

CDDA's inventory is a tree of pockets, not a flat list. Each worn or carried
container (backpack, holster, pot, magazine) has one or more pockets with
independent constraints: maximum volume, maximum weight, minimum/maximum item
volume, pocket type (`CONTAINER`, `MAGAZINE`, `HOLSTER`), and flag-based
restrictions.

Our `Inventory` component models this directly:

```rust
// cdda_sim/src/components/inventory.rs

/// A pocket within a container or worn item.
pub struct Pocket {
    pub pocket_type: PocketType,
    pub max_volume: Volume,
    pub max_weight: Weight,
    pub min_item_volume: Option<Volume>,
    pub max_item_length: Option<Millimeters>,
    pub contents: Vec<ItemInstance>,
    pub sealed: bool,
}

pub enum PocketType {
    Container,     // general-purpose storage
    Magazine,      // ammunition magazine
    Holster,       // sized for specific weapon types
    Special,       // special-purpose (e.g. canteen)
}

/// The root inventory component attached to every actor.
pub struct Inventory {
    pub worn: HashMap<BodyPartSlot, Vec<WornContainer>>,
    pub wielded: Option<ItemInstance>,
    pub carried: Option<ItemInstance>,
}
```

The inventory logic in `cdda_sim/logic/inventory.rs` handles insertion into
compatible pockets (by volume, weight, type constraints), extraction from nested
containers with cascade updates, and wield/unwield operations. This is purely
computational — no ECS, no UI — and fully unit-testable.

#### Z-level combat rule

CDDA's most persistent z-level bug class is "z-level view violation" — entities
attacking across z-levels when no floor opening exists. Our combat logic enforces
this as a single, centralized runtime check:

```rust
// logic/combat/melee.rs
pub fn can_reach(map: &WorldMap, attacker: WorldPos, target: WorldPos) -> bool {
    if attacker.z == target.z {
        return horizontal_dist(attacker, target) <= MELEE_REACH;
    }
    let floor_pos = WorldPos { z: attacker.z.min(target.z), ..attacker };
    map.tile(floor_pos)
       .map(|t| t.terrain.has_flag(TerFlag::NO_FLOOR))
       .unwrap_or(false)
}
```

The improvement over CDDA: exactly **one** `can_reach` function in the codebase,
called from every attack path. In CDDA this logic is scattered, allowing paths
to bypass it.

---

### `cdda_map` — Map storage, coordinates, generation

**Job:** Spatial layer. Stores tiles in typed submap structs. Manages the reality
bubble. Implements FOV, pathfinding, lighting, and mapgen including the palette
system. **No Bevy dependency.**

#### Key distinction: Submap vs. MapgenCanvas

- **`Submap`** (12×12 tiles): the unit of storage, load/unload, and serialization.
  This matches CDDA's save format.
- **`MapgenCanvas`** (24×24 tiles = 2×2 submaps): the unit mapgen writes into.
  After mapgen completes, the canvas is split across 4 submaps for storage.

#### Tile and submap storage

```rust
// tile.rs
pub struct Tile {
    pub terrain:   DefId<TerrainDef>,
    pub furniture: Option<DefId<FurnitureDef>>,
    pub trap:      Option<DefId<TrapDef>>,
    pub field:     FieldSet,
    pub items:     TileItems,
}

pub enum TileItems {
    Empty,
    Some(Box<[ItemInstance]>),   // most tiles empty; Box keeps Tile small
}

// submap.rs — the unit of storage and load/unload
pub struct Submap {
    pub tiles: Box<[Tile; 144]>,  // flat: tiles[y * 12 + x], ~3.5 KB per submap
    pub dirty: bool,               // set on any tile mutation; cleared after render sync
}

impl Submap {
    #[inline]
    pub fn get(&self, local: SubmapLocal) -> &Tile {
        &self.tiles[local.y as usize * 12 + local.x as usize]
    }

    #[inline]
    pub fn get_mut(&mut self, local: SubmapLocal) -> &mut Tile {
        self.dirty = true;
        &mut self.tiles[local.y as usize * 12 + local.x as usize]
    }
}
```

> **Morton ordering note.** Row-major (`y * 12 + x`) is the default. If profiling
> shows FOV/pathfinding is cache-bound (`perf stat -e cache-misses`), Morton
> (Z-order curve) ordering can improve cache-line reuse for spatially adjacent
> tiles. Do not switch preemptively — measure first.

#### World map

```rust
// map.rs
pub struct WorldMap {
    pub submaps: HashMap<SubmapPos, Box<Submap>>,
    pub overmap: OvermapIndex,
}
```

Public API always takes `WorldPos`; submap decomposition is internal:

```rust
impl WorldMap {
    pub fn tile(&self, pos: WorldPos) -> Option<&Tile> {
        let (sm_pos, local) = pos.to_submap();
        self.submaps.get(&sm_pos).map(|s| s.get(local))
    }

    pub fn tile_mut(&mut self, pos: WorldPos) -> Option<&mut Tile> {
        let (sm_pos, local) = pos.to_submap();
        self.submaps.get_mut(&sm_pos).map(|s| s.get_mut(local))
    }
}
```

#### Why not an octree for tile storage

The loaded region is **dense** (every tile has terrain), the dominant access
pattern is **random single-tile lookup** (O(1) on flat array, O(log N) on tree),
and all hot paths (FOV, pathfinding, mapgen, turn processing) benefit from flat
arrays. The R-tree (`rstar`) is used at the overmap layer where data is genuinely
sparse and range queries dominate.

#### Overmap spatial index

The overmap uses `rstar` (R\*-tree) for range and nearest-neighbor queries:

```rust
// overmap.rs
pub struct OvermapIndex {
    tiles: HashMap<OmtPos, OmtTile>,
    spatial: RTree<OvermapEntry>,     // rstar R*-tree
}

impl OvermapIndex {
    pub fn nearest_city(&self, from: OmtPos) -> Option<OmtPos> { /* ... */ }
    pub fn within_range(&self, center: OmtPos, radius: i32) -> impl Iterator<...> { /* ... */ }
}
```

This is a 2D spatial index over overmap x/y. Z-levels at overmap scale are few
and fixed (surface, underground), represented as an enum on `OmtTile`.

#### Pathfinding

The `pathfinding` crate (pure Rust, no Bevy, callback-based) handles local
pathfinding. Z-aware neighbor generation includes stairs, ladders, and ramps
via `GOES_UP`/`GOES_DOWN` terrain flags with `ZLevel::checked_add`/`checked_sub`
to prevent overflow. Long-distance NPC routing uses `bevy_northstar` in a crate
with full Bevy dependency. **Verify Bevy 0.18 compatibility before pinning** —
ecosystem crates often lag major Bevy releases.

#### Mapgen pipeline

Three phases plus multi-level assembly:

- **Phase 1 — Palette resolution** (`mapgen/palette.rs`): Merge all referenced
  palettes. Last-write wins for terrain/furniture; items additive.
- **Phase 2 — Canvas execution** (`mapgen/executor.rs`): Write a `MapgenCanvas`
  (24×24 tile buffer). `fill_ter` as background; `place_*` directives override.
- **Phase 3 — Canvas-to-submap split** (`mapgen/split.rs`): Split the completed
  `MapgenCanvas` into 2×2 submaps and write to `WorldMap.submaps`.
- **Phase 4 — Multi-level assembly** (overmap generator): Each entry in an
  `overmap_special`'s `overmaps` array carries a `[dx, dy, dz]` offset.
  Phases 1–3 run once per entry at a different z-level for basements (`dz: -1`)
  and roofs (`dz: +1`).

#### Vehicle coordinate handling

Vehicles use their own coordinate system:

```rust
pub struct VehicleComponent {
    pub parts: Vec<VehiclePart>,
    pub origin: WorldPos,
    pub facing: Facing,
    pub velocity: Option<(i32, i32)>,
    pub precalc_current: HashMap<usize, WorldPos>,  // mount index → current map pos
    pub precalc_next: HashMap<usize, WorldPos>,     // mount index → next tick map pos
}
```

Mount coordinates (`VehicleMountPos`) are relative to origin, assuming facing due
east. Map square coordinates (`VehicleMapPos`) account for current facing via
rotation and shearing, precalculated each tick.

```
crates/cdda_map/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── tile.rs
│   ├── submap.rs           # Submap: 12×12 tile array — storage and load/unload unit
│   ├── map.rs              # WorldMap: HashMap<SubmapPos, Box<Submap>> + OvermapIndex
│   ├── overmap.rs          # OmtTile, OvermapIndex (HashMap + rstar R-tree)
│   ├── coords.rs           # conversion functions between coordinate types
│   ├── fov.rs              # recursive shadowcasting — z-aware via NO_FLOOR flag
│   ├── lighting.rs
│   ├── pathfind.rs         # uses `pathfinding` crate — z-aware neighbor generation
│   ├── vehicle.rs          # vehicle coordinate transforms, precalculated arrays
│   │
│   ├── mapgen/
│   │   ├── mod.rs
│   │   ├── canvas.rs       # MapgenCanvas: 24×24 tile buffer (2×2 submaps)
│   │   ├── executor.rs     # Phase 2: interpret MapgenDef → fill MapgenCanvas
│   │   ├── palette.rs      # Phase 1: resolve palette chain → ResolvedPalette
│   │   ├── split.rs        # Phase 3: split MapgenCanvas → 4 Submaps
│   │   ├── nested.rs
│   │   ├── overmap_gen.rs  # Phase 4: place specials, calls executor per z-level
│   │   └── city.rs
│   │
│   └── plugin.rs
│
└── tests/
    ├── coords.rs
    ├── fov.rs
    ├── fov_z_level.rs
    ├── pathfind.rs
    ├── pathfind_z_level.rs
    ├── mapgen_exec.rs
    ├── mapgen_canvas_split.rs
    └── palette_resolve.rs
```

---

### `cdda_render` — Bevy rendering plugin

**Job:** Everything visual. Reads simulation state from the `World` and renders it.
Never writes simulation state — only communicates via Bevy messages
(`MessageWriter`/`MessageReader` in Bevy 0.18 naming).

ASCII mode is a first-class rendering path.

Tile rendering uses `bevy_fast_tilemap` (GPU-side buffers, "hundreds of fps
largely independent of map size") rather than `bevy_ecs_tilemap` (per-tile
entities, degrades at CDDA's ~144k visible tile scale). The `Submap::dirty` flag
drives upload batching — only changed submaps are re-uploaded to the GPU.

`bevy-inspector-egui` is a dev-dependency. Because all simulation components
derive `Reflect`, the `WorldInspectorPlugin` works out of the box in debug
builds.

> **Bevy 0.18 rendering note:** `RenderTarget` is now a required component on
> cameras, not a field on `Camera`. When spawning a camera, add
> `RenderTarget::Window(WindowRef::Primary)` as a separate component.

```
crates/cdda_render/
├── Cargo.toml              # bevy = { version = "0.18", features = ["2d"] }
│                           # bevy_fast_tilemap, bevy-inspector-egui (dev)
├── src/
│   ├── lib.rs
│   ├── plugin.rs
│   ├── tilemap.rs          # WorldMap → bevy_fast_tilemap sync (current z only)
│   │                       # uses Submap::dirty flag
│   ├── camera.rs           # Camera setup with RenderTarget component
│   ├── sprites.rs
│   │
│   ├── ui/
│   │   ├── mod.rs
│   │   ├── hud.rs
│   │   ├── sidebar.rs
│   │   ├── inventory.rs    # recursive inventory tree browser
│   │   ├── crafting.rs
│   │   ├── examine.rs
│   │   ├── character.rs
│   │   └── menus/
│   │       ├── mod.rs
│   │       ├── main_menu.rs
│   │       ├── world_gen.rs
│   │       └── char_create.rs
│   │
│   └── ascii/
│       ├── mod.rs
│       └── renderer.rs
```

---

### `cdda_input` — Input handling plugin

```
crates/cdda_input/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── plugin.rs
│   ├── actions.rs
│   ├── keybinds.rs
│   └── context.rs          # InputContextStack: Gameplay | Menu | Examine | …
```

---

### `cdda_audio` — Audio plugin

```
crates/cdda_audio/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── plugin.rs
│   ├── events.rs
│   └── bank.rs
```

---

### `cdda_app` — Binary entry point

The simulation `World` is owned by a resource in the Bevy `App`. One system calls
`cdda_sim::tick()` per game turn. Entity cleanup uses `StateScoped<S>` for
state-bound entities (main menu, world-gen preview, etc.), which are automatically
despawned on state exit.

```
crates/cdda_app/
├── Cargo.toml
└── src/
    ├── main.rs             # ~30 lines: App::new().add_plugins(GamePlugins)
    └── state.rs            # AppState: Loading | MainMenu | WorldGen | InGame | Paused
                            # Uses StateScoped<S> for entity lifecycle management
```

---

## Coordinate System Reference

Four coordinate scales, two origin types. All carry z as a `ZLevel(i8)` newtype
that does not participate in horizontal scaling. CDDA's POINTS_COORDINATES.md
confirms: "z-coordinates do not scale along with the horizontal dimensions."

```
WorldPos    — absolute tile position. NEVER changes as player moves.
              Always use WorldPos for stored coordinates (entity positions, etc.).
              WorldPos.xy = SubmapPos.xy * 12 + SubmapLocal.xy
              z is the same value at every scale.

SubmapPos   — which 12×12 submap. SubmapPos.xy = div_euclid(WorldPos.xy, 12).
              Must use div_euclid, not /. Negative submap coords are valid.
              The unit of serialization (matches CDDA save format).

SubmapLocal — offset within submap, 0..=11 on x and y.
              SubmapLocal.xy = rem_euclid(WorldPos.xy, 12).
              Must use rem_euclid, not %. Internal to cdda_map.
              Stores z as a convenience copy of the value in SubmapPos.

BubblePos   — position relative to the top-left corner of the reality bubble.
              Used inside FOV and rendering code. Never stored.

OmtPos      — overmap terrain scale. 1 unit = 24×24 world tiles (2×2 submaps).
              Used for overmap generation and long-range NPC routing.

OmPos       — overmap scale. 1 unit = 180×180 omts.

VehicleMountPos — part position relative to vehicle origin (facing east). Never changes.
                  Uses Pos<Ms, Rel> with vehicle entity as reference.

VehicleMapPos   — map-square position accounting for vehicle facing (rotation + shear).
                  Precalculated from mount coords each tick.
```

**Two axes of typing:**
- Scale (`Ms`, `Sm`, `Omt`, `Om`) — what one unit represents.
- Origin (`Abs`, `Bubble`, `Rel`) — what (0, 0) means.

Types do not coerce. Compiler errors on type mismatch prevent the coordinate
confusion bugs that CDDA's refactor has been chasing since 2014.

**Save/load rule:** Store `WorldPos` only. Never serialize `SubmapLocal` or a raw
submap-relative offset. `SubmapPos` is the serialization key for submap files.

---

## Save/Load Architecture

Save/load is a first-class concern, not an afterthought.

**Approach:** Custom serde-based serialization over simulation component types.
The design follows the MVC (Model-View-Controller) philosophy: simulation
components form the "model" (singular source of truth), rendering components are
the "view" (reconstructed on load, never saved). We use our own serde
implementation (not `bevy_save` or `moonshine_save` directly) to maintain full
control over the wire format and migration story.

**Atomicity:** Every submap write uses write-to-temp-then-rename (via
`std::fs::rename`, atomic on all supported platforms within the same filesystem).
Player and world state files follow the same pattern.

**Requirements on component types:**
- Every type in `cdda_sim/src/components/` must `#[derive(Component, Reflect, Serialize, Deserialize)]`
- Every such type must use `#[reflect(Component)]` (Bevy 0.18: parentheses only)
- Every such type must be registered: `world.register_component::<T>()`
- Entity IDs are treated as opaque — use a stable `UniqueId` component instead
  (a UUID generated at entity spawn)

**Wire format versioning:** Breaking changes increment the format version.
Migration functions handle older saves before deserialization. See
`docs/save-format.md`.

**What is saved vs. reconstructed:**
- **Saved:** all `cdda_sim` components on entities with the `Save` marker, all
  simulation resources (player position, time of day, active effects), all submap
  tile data
- **Reconstructed on load:** all rendering components, audio state, UI state

---

## Integration Tests

```
tests/integration/
├── data_loading.rs
├── copy_from.rs
├── extend_delete.rs
├── item_groups.rs
├── crafting.rs
├── combat_round.rs
├── combat_z_level.rs
├── map_gen.rs
├── mapgen_canvas_split.rs
├── mapgen_z_levels.rs
└── mod_loading.rs
```

---

## Testing Strategy

| Layer | Test type | Location | How |
|---|---|---|---|
| Units, coords, damage | Unit | `cdda_core/tests/` | `cargo test` — no ECS |
| Deserialization, copy-from | Unit | `cdda_data/tests/` | `cargo test` — no ECS |
| Item groups, mapgen palette | Unit | `cdda_data/tests/` | `cargo test` — no ECS |
| Mod loading, conflicts | Unit | `cdda_mod/tests/` | `cargo test` — no ECS |
| Combat formulas, AI scoring | Unit | `cdda_sim/tests/` | `cargo test` — pure logic functions, no World |
| FSM transitions | Unit | `cdda_sim/tests/` | `cargo test` — no World |
| System integration | Integration | `cdda_sim/tests/` | Spawn `World`, run system, assert components |
| FOV, pathfinding | Unit | `cdda_map/tests/` | `cargo test` — no ECS |
| Mapgen canvas split | Unit | `cdda_map/tests/` | `cargo test` — no ECS |
| Full data pipeline | Integration | `tests/integration/` | `cargo test` — World + data |
| Map generation + z-levels | Integration | `tests/integration/` | `cargo test` — World + map |
| Rendering, UI | Manual / screenshot | `cdda_render/` | Visual inspection |

**CI command:** `cargo test --workspace --exclude cdda_render --exclude cdda_app`

> `cdda_input` and `cdda_audio` depend on full Bevy but may work headless.
> Test before deciding whether to exclude them.

Target: under 15 seconds on a mid-range developer machine.

---

## `Cargo.toml` (workspace root)

```toml
[workspace]
resolver = "2"
members = [
    "crates/cdda_core",
    "crates/cdda_data",
    "crates/cdda_mod",
    "crates/cdda_sim",
    "crates/cdda_map",
    "crates/cdda_render",
    "crates/cdda_input",
    "crates/cdda_audio",
    "crates/cdda_app",
]

[workspace.dependencies]
# Bevy 0.18 — released 2026-01-13.
# Key changes from 0.17: EventReader→MessageReader, RenderTarget is a component,
# SimpleExecutor removed, ron no longer re-exported, FunctionSystem gains In generic,
# #[reflect(...)] now only supports parentheses syntax.
# See contributing.md for the full migration checklist.
# Budget ~1 dev-week per Bevy release.
bevy            = { version = "0.18", default-features = false }
bevy_ecs        = "0.18"          # standalone data library — cdda_sim only
bevy_reflect    = "0.18"          # standalone reflection — cdda_sim only
serde           = { version = "1", features = ["derive"] }
serde_json      = "1"
thiserror       = "2"
indexmap        = "2"
rand            = "0.9"
tracing         = "0.1"
smallvec        = "1"
rstar           = "0.12"
pathfinding     = "4"
bevy-inspector-egui = { version = "0.28", optional = true }  # debug only
bevy_fast_tilemap   = "0.9"       # GPU-side tile rendering, no per-tile entities
# VERIFY: check crates.io for Bevy 0.18-compatible version before pinning

# NOT included:
# - seldom_state: full Bevy dependency; pure-Rust FSM in cdda_sim/logic/fsm/ replaces it
# - zorder: only add after profiling proves cache-bound submap access
# - bevy_save: couples wire format to Bevy internals
# - big-brain: archived October 2025

[profile.dev]
opt-level = 0

[profile.dev.package."*"]
opt-level = 2

[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
```

---

## `.cargo/config.toml` — fast dev builds

```toml
# Dynamic linking speeds up incremental builds on Linux and macOS.
# Does NOT work reliably on Windows.
# Never enable in release builds.
[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold"]

[target.x86_64-apple-darwin]
rustflags = ["-C", "link-arg=-fuse-ld=/usr/local/bin/zld"]
```

Enable `bevy/dynamic_linking` via a `dev` feature flag in `cdda_app/Cargo.toml`
for Linux and macOS only.

---

## Onboarding Path

1. `cargo test -p cdda_data` — JSON parsing tests pass.
2. `cargo test -p cdda_sim` — pure logic tests. Read `logic/combat/melee.rs`.
3. `cargo test --workspace --exclude cdda_render --exclude cdda_app` — full suite.
4. `cargo run -p cdda_app` — see the game. Press `F3` to open the world inspector.

**Adding a new game mechanic (always the same 7 steps):**

```
1. Add def fields  →  cdda_data/src/defs/<type>.rs
2. Add component   →  cdda_sim/src/components/<area>.rs
                      Must: #[derive(Component, Reflect, Serialize, Deserialize)]
                      Must: #[reflect(Component)]  (Bevy 0.18: parentheses only)
                      Must: world.register_component::<MyComponent>() in world_setup.rs
3. Add logic fn    →  cdda_sim/src/logic/<area>.rs        ← write unit tests here
4. Add system      →  cdda_sim/src/systems/<area>.rs      ← Query→logic→write results
5. Register system →  cdda_sim/src/tick.rs (add to tick() in explicit order)
6. Add to save     →  add Save marker to entities that carry this component
7. Add UI          →  cdda_render/src/ui/                 ← if player-visible
```

Step 2 has three sub-requirements (Component derive + Reflect with 0.18 syntax +
registration) because forgetting any one causes silent save failures.

**Adding new content (no code change):**

```
1. Add JSON  →  data/core/<category>/<file>.json
2. Run tests →  cargo test -p cdda_data
```

---

## Ecosystem Crate Summary

| Problem | Crate | Depends on | Notes |
|---|---|---|---|
| Simulation data model | `bevy_ecs` (standalone) | Rust std only | Archetypal storage, queries, change detection, hooks, relationships |
| Reflection for save/debug | `bevy_reflect` (standalone) | Rust std only | Already depended on; mirrored by `cdda_sim` |
| Creature AI | custom `cdda_sim/logic/ai` | Nothing | ~200 lines pure Rust; runs on turn tick |
| Player/NPC state | custom `cdda_sim/logic/fsm` | Nothing | ~150 lines pure Rust |
| Local pathfinding | `pathfinding` | Nothing | Callback A*; lives in `cdda_map` |
| Long-range pathfinding | `bevy_northstar` | Full Bevy | HPA*; lives outside `cdda_map`. Verify 0.18 compat. |
| FOV / shadowcasting | `doryen-fov` or `adam_fov_rs` | Nothing | Pure Rust; lives in `cdda_map` |
| Overmap spatial queries | `rstar` | Nothing | R*-tree; lives in `cdda_map` |
| Tile rendering | `bevy_fast_tilemap` | Full Bevy | GPU-side buffers; verify 0.18 compat on crates.io |
| Debug inspector | `bevy-inspector-egui` | Full Bevy | Dev-only; works via `Reflect` |
| Asset loading | `bevy_asset_loader` + `iyes_progress` | Full Bevy | Loading state management |
| Save/load | custom serde + atomic file writes | `serde` | Full wire-format control |

**Crates explicitly NOT used:**

| Crate | Reason |
|---|---|
| `big-brain` | Archived October 2025; real-time eval mismatches turn-based sim |
| `bevy_save` | Couples wire format to Bevy reflection internals |
| `bevy_ecs_tilemap` | Per-tile entities degrade at CDDA's ~144k visible tile scale |
| `seldom_state` (in sim) | Full Bevy scheduler dependency; pure-Rust FSM serves simulation |
| `moonshine_save` (direct use) | MVC philosophy adopted but implemented with custom serde for control |
| The full `bevy` crate (in sim) | Non-deterministic scheduler, frame coupling, winit, rendering |
| `zorder` | Only add if profiling shows cache-bound submap access |

---

## Decision Table: Original CDDA → This Architecture

| Original CDDA pain | This architecture |
|---|---|
| `g->` global singleton | No singleton — `Resource`s and `Component`s |
| `character.cpp` ~13,000 lines | ECS archetypal components, no god class |
| `is_player()` / `is_npc()` branches | Marker components, query by tag |
| Adding monster attack needs 4 file edits | Fully data-driven from JSON |
| `charges` duality | `CountMode` enum — explicit |
| `player::` → `avatar::` migration half-done | No `Player` class ever created |
| Low test coverage, tests need game startup | Pure logic functions tested with `cargo test`; no World needed |
| Flat `src/` with 400+ files | 9 crates, domain obvious from name |
| Untyped `point`/`tripoint` — scale confusion | `Pos<Scale, Origin>` — both axes typed |
| Scale typed but origin still ambiguous (#71852) | Both scale AND origin in type params |
| `/` and `%` on signed coords → wrong submap | `div_euclid`/`rem_euclid` enforced |
| Z-level retrofitted 2015, bugs still open 2024 | Z first-class in every system from day 1 |
| "Z-level view violation" debug spam | Single `can_reach()` function, all attack paths use it |
| 3D FOV broken in 2023 | Unified FOV; cross-z visibility via NO_FLOOR flag |
| NPCs stopping on different z-level | Simulation systems operate on WorldPos, z-agnostic |
| Deferred loader re-scans until resolved | Explicit two-pass, handles extend/delete/relative |
| Non-humanoid bodies retrofitted mid-dev | Dynamic `BodyParts` from day one |
| Modding bolted on | `cdda_mod` is its own crate |
| Save/load not designed in | Custom serde, atomic writes, versioned format |
| Save unit is submap (12×12), not OMT (24×24) | `Submap` vs. `MapgenCanvas` split preserved |
| Architecture doc "approximate and outdated" | This file, reviewed on structural PRs |
| Hand-rolled component storage, no query engine | `bevy_ecs` archetypal storage, `Query`, change detection |
| Simulation non-deterministic when parallel | Manual `system.run()` in fixed order, single-threaded |
| Flat inventory → per-tile item piles only | Tree-based inventory with pockets, nesting, type constraints |
| Vehicle coords mixed with world coords | Vehicle uses `Pos<Ms, Rel>`; type prevents world-vehicle mixing |
| AI (big-brain) archived; real-time eval | Custom utility AI, runs on turn tick |
| UI entities persist across state transitions | `StateScoped<S>` for automatic cleanup |
| z stored as bare int, silent overflow | `ZLevel(i8)` newtype with checked arithmetic |
| Rendering re-uploads entire map on any change | `Submap::dirty` flag; only changed submaps re-uploaded |
