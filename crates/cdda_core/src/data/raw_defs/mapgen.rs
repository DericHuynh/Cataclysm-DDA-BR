//! Mapgen and palette definition types.
//!
//! CDDA's mapgen system uses two JSON types:
//! - `"mapgen"` — a mapgen definition (with `om_terrain` or `nested_mapgen_id`)
//! - `"palette"` — shared symbol→terrain/furniture/item mappings

use crate::data::raw_defs::cdda_types::RawValue;
use crate::data::raw_types::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ===========================================================================
// MapgenDef — the "mapgen" type
// ===========================================================================

/// A mapgen definition from JSON type `"mapgen"`.
///
/// CDDA structure:
/// ```json
/// {
///   "type": "mapgen",
///   "om_terrain": "house_01",
///   "weight": 100,
///   "method": "json",
///   "object": { ... }
/// }
/// ```
///
/// For nested (sub-mapgen) entries, `nested_mapgen_id` replaces `om_terrain`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MapgenDef {
    /// OMT this mapgen applies to.  For nested mapgen, this is absent and
    /// `nested_mapgen_id` is used instead.
    #[serde(default)]
    pub om_terrain: Option<MapgenTarget>,

    /// For nested sub-mapgen: the nested mapgen ID string.
    #[serde(default)]
    pub nested_mapgen_id: Option<String>,

    /// Weight for random selection among multiple mapgen for the same OMT.
    #[serde(default = "default_weight")]
    pub weight: u32,

    /// Mapgen method: "json" (default), "builtin", etc.
    #[serde(default)]
    pub method: Option<String>,

    /// The actual mapgen data — rows, palettes, terrain, furniture, etc.
    #[serde(default)]
    pub object: Option<MapgenObject>,
}

fn default_weight() -> u32 {
    100
}

/// What OMT this mapgen applies to — a single string or an array of strings.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum MapgenTarget {
    Single(String),
    Multi(Vec<String>),
}

// ===========================================================================
// MapgenObject — the "object" field
// ===========================================================================

/// The `object` field of a mapgen definition, containing all placement data.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MapgenObject {
    /// Default terrain for every cell not otherwise specified.
    #[serde(default)]
    pub fill_ter: Option<String>,

    /// Size of the mapgen area.  Defaults to [24, 24] (one OMT).
    /// For nested mapgen, this can be smaller (e.g. [6, 6]).
    #[serde(default)]
    pub mapgensize: Option<Vec<u32>>,

    /// Grid of characters — each string is a row.
    #[serde(default)]
    pub rows: Option<Vec<String>>,

    /// Palettes referenced by this mapgen.
    #[serde(default)]
    pub palettes: Vec<PaletteRef>,

    /// Character → terrain ID mappings (overrides palette).
    #[serde(default)]
    pub terrain: Option<HashMap<String, RawValue>>,

    /// Character → furniture ID mappings (overrides palette).
    #[serde(default)]
    pub furniture: Option<HashMap<String, RawValue>>,

    /// Character → item group / spawn data (overrides palette).
    #[serde(default)]
    pub items: Option<HashMap<String, RawValue>>,

    /// Character → toilet marker.
    #[serde(default)]
    pub toilets: Option<HashMap<String, RawValue>>,

    /// Character → monster spawn.
    #[serde(default)]
    pub monster: Option<HashMap<String, RawValue>>,

    /// Character → liquid placement.
    #[serde(default)]
    pub liquids: Option<HashMap<String, RawValue>>,

    /// Character → vending machine config.
    #[serde(default)]
    pub vendingmachines: Option<HashMap<String, RawValue>>,

    /// Character → nested mapgen chunks.
    #[serde(default)]
    pub nested: Option<HashMap<String, RawValue>>,

    /// Remove all items/monsters/etc. of the given chars from nested mapgen.
    #[serde(default)]
    pub remove_all: Option<HashMap<String, Vec<String>>>,

    /// Position-based item spawn directives.
    #[serde(default)]
    pub place_items: Option<Vec<PlaceItem>>,

    /// Position-based monster spawn directives.
    #[serde(default)]
    pub place_monster: Option<Vec<PlaceMonster>>,

    /// Position-based monster group spawn directives.
    #[serde(default)]
    pub place_monsters: Option<Vec<PlaceMonster>>,

    /// Position-based NPC placement.
    #[serde(default)]
    pub place_npc: Option<Vec<PlaceNpc>>,

    /// Position-based NPCs placement.
    #[serde(default)]
    pub place_npcs: Option<Vec<PlaceNpc>>,

    /// Position-based field placement.
    #[serde(default)]
    pub place_fields: Option<Vec<PlaceField>>,

    /// Position-based trap placement.
    #[serde(default)]
    pub place_traps: Option<Vec<PlaceTrap>>,

    /// Position-based terrain placement.
    #[serde(default)]
    pub place_terrain: Option<Vec<PlaceTerrain>>,

    /// Position-based furniture placement.
    #[serde(default)]
    pub place_furniture: Option<Vec<PlaceFurniture>>,

    /// Loot (specific item) placement.
    #[serde(default)]
    pub place_loot: Option<Vec<PlaceLoot>>,

    /// Nested mapgen chunk placement.
    #[serde(default)]
    pub place_nested: Option<Vec<PlaceNested>>,

    /// Zone placement.
    #[serde(default)]
    pub place_zones: Option<Vec<PlaceZone>>,

    /// Vehicle placement.
    #[serde(default)]
    pub place_vehicles: Option<Vec<PlaceVehicle>>,

    /// Terrain transformation mapping.
    #[serde(default)]
    pub mapping: Option<HashMap<String, RawValue>>,

    /// Set directives for mapgen parameters.
    #[serde(default)]
    pub set: Option<Vec<SetDirective>>,

    /// General flags.
    #[serde(default)]
    pub flags: Vec<String>,
}

// ===========================================================================
// PaletteRef
// ===========================================================================

/// A reference to a palette — can be a plain string ID, a parameter, or a
/// distribution of weighted options.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum PaletteRef {
    /// Simple: `"domestic_general_palette"`.
    Id(String),
    /// Parameterized: `{"param": "food_type"}`.
    Param { param: String },
    /// Object with palette key: `{"palette": "roof_palette"}`.
    Obj { palette: String },
    /// Weighted distribution: `{"distribution": [["a", 2], ["b", 1]]}`.
    Distribution { distribution: Vec<RawValue> },
}

// ===========================================================================
// Position helper — accepts both single values and [min, max] ranges
// ===========================================================================

/// A coordinate that can be a single integer or a `[min, max]` range.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum PosOrRange {
    Single(i32),
    Range([i32; 2]),
}

// ===========================================================================
// PlaceXxx directive types
// ===========================================================================

/// `place_items` directive.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PlaceItem {
    /// Item or item group ID.
    #[serde(default)]
    pub item: Option<String>,
    /// Item group ID (alternative).
    #[serde(default)]
    pub group: Option<String>,
    /// Chance 0–100.
    #[serde(default = "default_chance")]
    pub chance: u32,
    /// X position or range.
    #[serde(default)]
    pub x: Option<PosOrRange>,
    /// Y position or range.
    #[serde(default)]
    pub y: Option<PosOrRange>,
    /// Repeat count or [min, max].
    #[serde(default)]
    pub repeat: Option<PosOrRange>,
}

/// `place_monster` / `place_monsters` directive.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PlaceMonster {
    /// Monster group or specific monster ID.
    #[serde(default)]
    pub monster: Option<String>,
    /// Monster group ID.
    #[serde(default)]
    pub group: Option<String>,
    /// X position or range.
    #[serde(default)]
    pub x: Option<PosOrRange>,
    /// Y position or range.
    #[serde(default)]
    pub y: Option<PosOrRange>,
    /// Chance 0–100.
    #[serde(default = "default_chance")]
    pub chance: u32,
    /// Repeat count or [min, max].
    #[serde(default)]
    pub repeat: Option<PosOrRange>,
    /// Density multiplier.
    #[serde(default)]
    pub density: Option<f64>,
}

/// `place_npc` / `place_npcs` directive.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PlaceNpc {
    /// NPC class ID.
    pub class: String,
    /// X position or range.
    #[serde(default)]
    pub x: Option<PosOrRange>,
    /// Y position or range.
    #[serde(default)]
    pub y: Option<PosOrRange>,
    /// Whether to place as a target (unique).
    #[serde(default)]
    pub target: Option<bool>,
}

/// `place_field` directive.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PlaceField {
    /// Field type ID.
    pub field: String,
    /// X position or range.
    #[serde(default)]
    pub x: Option<PosOrRange>,
    /// Y position or range.
    #[serde(default)]
    pub y: Option<PosOrRange>,
    /// Field intensity.
    #[serde(default)]
    pub density: Option<u32>,
    /// Age of field in turns.
    #[serde(default)]
    pub age: Option<i32>,
}

/// `place_trap` directive.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PlaceTrap {
    /// Trap type ID.
    pub trap: String,
    /// X position or range.
    #[serde(default)]
    pub x: Option<PosOrRange>,
    /// Y position or range.
    #[serde(default)]
    pub y: Option<PosOrRange>,
    /// Repeat count.
    #[serde(default)]
    pub repeat: Option<PosOrRange>,
}

/// `place_terrain` directive.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PlaceTerrain {
    /// Terrain type ID.
    pub terrain: String,
    /// X position or range.
    #[serde(default)]
    pub x: Option<PosOrRange>,
    /// Y position or range.
    #[serde(default)]
    pub y: Option<PosOrRange>,
    /// Repeat count.
    #[serde(default)]
    pub repeat: Option<PosOrRange>,
}

/// `place_furniture` directive.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PlaceFurniture {
    /// Furniture type ID.
    pub furn: String,
    /// X position or range.
    #[serde(default)]
    pub x: Option<PosOrRange>,
    /// Y position or range.
    #[serde(default)]
    pub y: Option<PosOrRange>,
    /// Repeat count.
    #[serde(default)]
    pub repeat: Option<PosOrRange>,
}

/// `place_loot` directive — places a specific item.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PlaceLoot {
    /// Item ID.
    pub item: String,
    /// X position or range.
    #[serde(default)]
    pub x: Option<PosOrRange>,
    /// Y position or range.
    #[serde(default)]
    pub y: Option<PosOrRange>,
    /// Chance 0–100.
    #[serde(default = "default_chance")]
    pub chance: u32,
    /// Repeat count or [min, max].
    #[serde(default)]
    pub repeat: Option<PosOrRange>,
    /// Item group (alternative to `item`).
    #[serde(default)]
    pub group: Option<String>,
}

/// `place_nested` directive — places a nested mapgen chunk.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PlaceNested {
    /// Chunk(s) to place.  Can be a single string or a list of `[id, weight]` tuples.
    #[serde(default)]
    pub chunks: Option<RawValue>,

    /// Alternative chunks if the primary chunks can't be placed.
    #[serde(default)]
    pub else_chunks: Option<RawValue>,

    /// X position or range.
    #[serde(default)]
    pub x: Option<PosOrRange>,
    /// Y position or range.
    #[serde(default)]
    pub y: Option<PosOrRange>,

    /// Neighbor constraints.
    #[serde(default)]
    pub neighbors: Option<HashMap<String, String>>,
}

/// `place_zones` directive — places a faction zone.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PlaceZone {
    /// Zone type (e.g. "LOOT_UNSORTED").
    #[serde(rename = "type")]
    pub zone_type: String,
    /// Faction that owns the zone.
    #[serde(default)]
    pub faction: Option<String>,
    /// X position or range.
    #[serde(default)]
    pub x: Option<PosOrRange>,
    /// Y position or range.
    #[serde(default)]
    pub y: Option<PosOrRange>,
}

/// `place_vehicles` directive.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PlaceVehicle {
    /// Vehicle type or group.
    pub vehicle: String,
    /// X position or range.
    #[serde(default)]
    pub x: Option<PosOrRange>,
    /// Y position or range.
    #[serde(default)]
    pub y: Option<PosOrRange>,
    /// Chance 0–100.
    #[serde(default = "default_chance")]
    pub chance: u32,
    /// Facing direction in degrees.
    #[serde(default)]
    pub facing: Option<i32>,
    /// Status of the vehicle (-1 = wreck, 0 = default, etc.).
    #[serde(default)]
    pub status: Option<i32>,
}

/// A "set" directive for mapgen parameters.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SetDirective {
    #[serde(default)]
    pub line: Option<String>,
    #[serde(default)]
    pub point: Option<String>,
    #[serde(default)]
    pub x: Option<i32>,
    #[serde(default)]
    pub y: Option<i32>,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub set: Option<String>,
    #[serde(default)]
    pub amount: Option<i32>,
    #[serde(default)]
    pub rotation: Option<i32>,
    #[serde(default)]
    pub status: Option<String>,
}

// ===========================================================================
// MapgenPaletteDef — the "palette" type
// ===========================================================================

/// A mapgen palette from JSON type `"palette"`.
///
/// Palettes are shared symbol→entity mappings referenced by mapgen definitions.
/// Multiple palette IDs can compose (via the `palettes` array).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MapgenPaletteDef {
    /// Unique palette identifier.
    pub id: DefId<MapgenPaletteDef>,

    /// Other palettes to compose into this one (bottom-to-top layering).
    #[serde(default)]
    pub palettes: Vec<PaletteRef>,

    /// Symbol → terrain ID.
    /// Value can be a string (`"t_floor"`) or an array of `[id, weight]` tuples
    /// for weighted random selection.
    #[serde(default)]
    pub terrain: Option<HashMap<String, RawValue>>,

    /// Symbol → furniture ID.  Same format as terrain.
    #[serde(default)]
    pub furniture: Option<HashMap<String, RawValue>>,

    /// Symbol → item group / spawn entry.
    /// Value is typically `{"item": "group_id", "chance": N}` or an array of such objects.
    #[serde(default)]
    pub items: Option<HashMap<String, RawValue>>,

    /// Parameter definitions for parameterized palettes.
    #[serde(default)]
    pub parameters: Option<HashMap<String, RawValue>>,

    /// Symbol → toilet marker (value is typically `{}`).
    #[serde(default)]
    pub toilets: Option<HashMap<String, RawValue>>,

    /// Symbol → vending machine config.
    #[serde(default)]
    pub vendingmachines: Option<HashMap<String, RawValue>>,

    /// Symbol → liquid placement data.
    #[serde(default)]
    pub liquids: Option<HashMap<String, RawValue>>,

    /// Symbol → monster spawn entry.
    #[serde(default)]
    pub monster: Option<HashMap<String, RawValue>>,

    /// General flags.
    #[serde(default)]
    pub flags: Vec<String>,
}

fn default_chance() -> u32 {
    100
}
