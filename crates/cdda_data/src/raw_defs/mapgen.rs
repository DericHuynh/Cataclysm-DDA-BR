use crate::raw_defs::cdda_types::RawValue;
use crate::raw_types::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A mapgen definition from JSON type `"mapgen"`.
///
/// Defines how an overmap terrain (OMT) is procedurally generated.
/// Multiple mapgen defs can exist for a single OMT, chosen at random.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MapgenDef {
    /// If present, must match an `OvermapTerrainDef` ID.
    #[serde(default)]
    pub om_terrain: Option<MapgenTarget>,

    /// Weight for weighted selection when multiple mapgen exist for the same OMT.
    #[serde(default = "default_weight")]
    pub weight: u32,

    /// Method of mapgen execution.
    #[serde(default)]
    pub method: Option<String>,

    /// Whether to use JSON objects format.
    #[serde(default)]
    pub object: Option<MapgenObject>,

    /// Fill terrain (background tile for the entire OMT).
    #[serde(default)]
    pub fill_ter: Option<DefId<crate::raw_defs::terrain::TerrainDef>>,

    /// Palettes referenced by this mapgen.
    #[serde(default)]
    pub palettes: Vec<PaletteRef>,

    /// Terrain placement data (char → terrain ID).
    #[serde(default)]
    pub terrain: Option<HashMap<String, String>>,

    /// Furniture placement data (char → furniture ID).
    #[serde(default)]
    pub furniture: Option<HashMap<String, String>>,

    /// Place result (field or type-specific data).
    #[serde(default)]
    pub place_result: Option<HashMap<String, RawValue>>,

    /// Set of place_* directives.
    #[serde(default)]
    pub place_terrain: Option<Vec<PlaceTerrain>>,

    /// Place furniture directives.
    #[serde(default)]
    pub place_furniture: Option<Vec<PlaceFurniture>>,

    /// Place items directives.
    #[serde(default)]
    pub place_items: Option<Vec<PlaceItems>>,

    /// Place monster directives.
    #[serde(default)]
    pub place_monster: Option<Vec<PlaceMonster>>,

    /// Place NPC directives.
    #[serde(default)]
    pub place_npc: Option<Vec<PlaceNpc>>,

    /// Place field directives.
    #[serde(default)]
    pub place_fields: Option<Vec<PlaceField>>,

    /// Place trapping directives.
    #[serde(default)]
    pub place_traps: Option<Vec<PlaceTrap>>,

    /// Add terrain transformation (mapping char → terrain mapping).
    #[serde(default)]
    pub mapping: Option<HashMap<String, RawValue>>,

    /// Set of "set" directives.
    #[serde(default)]
    pub set: Option<Vec<SetDirective>>,

    /// Nested mapgen references.
    #[serde(default)]
    pub nested: Option<HashMap<String, RawValue>>,

    /// Row data (old-style character-based mapgen).
    #[serde(default)]
    pub rows: Option<Vec<String>>,
}

fn default_weight() -> u32 {
    100
}

/// What OMT this mapgen applies to.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum MapgenTarget {
    /// Single OMT ID.
    Single(String),
    /// List of OMT IDs.
    Multi(Vec<String>),
}

/// A palette reference in mapgen, which can be a string or a parameterized reference.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum PaletteRef {
    /// Simple: palette ID string.
    Id(DefId<MapgenPaletteDef>),
    /// Parameterized: {"param": "palette_id_param"}.
    Param { param: String },
    /// Object: {"palette": "id"}.
    Obj { palette: DefId<MapgenPaletteDef> },
    /// Distribution: {"distribution": [["id", weight], ...]}.
    Distribution { distribution: Vec<RawValue> },
}

/// Map generation object (the JSON `object` field of a mapgen def).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MapgenObject {
    /// Pre-fill terrain.
    #[serde(default)]
    pub fill_ter: Option<DefId<crate::raw_defs::terrain::TerrainDef>>,

    /// Rows of characters representing the map.
    #[serde(default)]
    pub rows: Option<Vec<String>>,

    /// Character → terrain mapping.
    #[serde(default)]
    pub terrain: Option<HashMap<String, String>>,

    /// Character → furniture mapping.
    #[serde(default)]
    pub furniture: Option<HashMap<String, String>>,

    /// Palettes.
    #[serde(default)]
    pub palettes: Vec<PaletteRef>,

    /// Nested mapgen.
    #[serde(default)]
    pub nested: Option<HashMap<String, RawValue>>,

    /// Place items.
    #[serde(default)]
    pub place_items: Option<Vec<PlaceItems>>,

    /// Place monster.
    #[serde(default)]
    pub place_monster: Option<Vec<PlaceMonster>>,

    /// Place NPC.
    #[serde(default)]
    pub place_npc: Option<Vec<PlaceNpc>>,

    /// Place fields.
    #[serde(default)]
    pub place_fields: Option<Vec<PlaceField>>,

    /// Place traps.
    #[serde(default)]
    pub place_traps: Option<Vec<PlaceTrap>>,

    /// Set directives.
    #[serde(default)]
    pub set: Option<Vec<SetDirective>>,

    /// Mapping directives.
    #[serde(default)]
    pub mapping: Option<HashMap<String, RawValue>>,
}

/// A place_terrain directive.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PlaceTerrain {
    /// Position as character reference.
    #[serde(default)]
    pub c: Option<String>,

    /// Position as x, y coords.
    #[serde(default)]
    pub x: Option<i32>,
    #[serde(default)]
    pub y: Option<i32>,

    /// Target terrain.
    pub terrain: DefId<crate::raw_defs::terrain::TerrainDef>,

    /// Repeat count.
    #[serde(default)]
    pub repeat: Option<u32>,
}

/// A place_furniture directive.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PlaceFurniture {
    #[serde(default)]
    pub c: Option<String>,
    #[serde(default)]
    pub x: Option<i32>,
    #[serde(default)]
    pub y: Option<i32>,
    pub furn: DefId<crate::raw_defs::furniture::FurnitureDef>,
    #[serde(default)]
    pub repeat: Option<u32>,
}

/// A place_items directive.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PlaceItems {
    /// Item group ID to spawn.
    pub item: DefId<crate::raw_defs::item_group::ItemGroupDef>,
    /// Chance (as percentage).
    #[serde(default = "default_chance")]
    pub chance: u32,
    /// Position.
    #[serde(default)]
    pub x: Option<i32>,
    #[serde(default)]
    pub y: Option<i32>,
    /// Repeat count.
    #[serde(default)]
    pub repeat: Option<u32>,
}

fn default_chance() -> u32 {
    100
}

/// A place_monster directive.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PlaceMonster {
    /// Monster group or specific monster ID.
    pub monster: String,
    /// Position.
    #[serde(default)]
    pub x: Option<i32>,
    #[serde(default)]
    pub y: Option<i32>,
    /// Chance.
    #[serde(default = "default_chance")]
    pub chance: u32,
    /// Whether to spawn as a group.
    #[serde(default)]
    pub group: Option<bool>,
    /// Whether to pack together.
    #[serde(default)]
    pub pack: Option<bool>,
    /// Repeat count.
    #[serde(default)]
    pub repeat: Option<u32>,
    /// Density multiplier.
    #[serde(default)]
    pub density: Option<f64>,
}

/// A place_npc directive.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PlaceNpc {
    /// NPC class ID.
    pub class: String,
    /// Position.
    #[serde(default)]
    pub x: Option<i32>,
    #[serde(default)]
    pub y: Option<i32>,
    /// Whether to place as a target.
    #[serde(default)]
    pub target: Option<bool>,
}

/// A place_field directive.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PlaceField {
    /// Field type ID.
    pub field: String,
    /// Position.
    #[serde(default)]
    pub x: Option<i32>,
    #[serde(default)]
    pub y: Option<i32>,
    /// Field intensity/density.
    #[serde(default)]
    pub density: Option<u32>,
    /// Age of field in turns.
    #[serde(default)]
    pub age: Option<i32>,
}

/// A place_trap directive.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PlaceTrap {
    /// Trap type ID.
    pub trap: String,
    /// Position.
    #[serde(default)]
    pub x: Option<i32>,
    #[serde(default)]
    pub y: Option<i32>,
    #[serde(default)]
    pub repeat: Option<u32>,
}

/// A "set" directive for mapgen parameters.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SetDirective {
    /// What to set.
    #[serde(default)]
    pub line: Option<String>,
    /// Value.
    pub point: Option<String>,
    /// Position.
    #[serde(default)]
    pub x: Option<i32>,
    #[serde(default)]
    pub y: Option<i32>,
    /// Id of vehicle to place.
    #[serde(default)]
    pub id: Option<String>,
    /// Whether to set.
    #[serde(default)]
    pub set: Option<String>,
    /// Amount.
    #[serde(default)]
    pub amount: Option<i32>,
    /// Rotation.
    #[serde(default)]
    pub rotation: Option<i32>,
    /// Status.
    #[serde(default)]
    pub status: Option<String>,
}

/// A mapgen palette definition from JSON type `"palette"`.
///
/// Palettes are shared mappings from map characters to terrain/furniture/items/etc.
/// Multiple mapgen defs can reference the same palette.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MapgenPaletteDef {
    /// Unique identifier.
    pub id: DefId<MapgenPaletteDef>,

    /// Character → terrain mappings.
    #[serde(default)]
    pub terrain: Option<HashMap<String, RawValue>>,

    /// Character → furniture mappings.
    #[serde(default)]
    pub furniture: Option<HashMap<String, RawValue>>,

    /// Character → item group mappings.
    #[serde(default)]
    pub items: Option<HashMap<String, RawValue>>,

    /// Nested / referenced palettes.
    #[serde(default)]
    pub palettes: Vec<PaletteRef>,

    /// Parameters for parameterized palettes.
    #[serde(default)]
    pub parameters: Option<HashMap<String, RawValue>>,

    /// Toilet mappings.
    #[serde(default)]
    pub toilets: Option<HashMap<String, RawValue>>,

    /// Vending machine mappings.
    #[serde(default)]
    pub vendingmachines: Option<HashMap<String, RawValue>>,

    /// Flags.
    #[serde(default)]
    pub flags: Vec<String>,
}
