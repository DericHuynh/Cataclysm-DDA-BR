use crate::raw_defs::cdda_types::{RawValue, SeeCost, StringOrArray};
use crate::raw_types::{DefId, LocalizedString};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Overmap terrain definition from JSON type `"overmap_terrain"`.
///
/// Defines a 24×24 tile region on the overmap (e.g. "house", "forest", "lake").
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OvermapTerrainDef {
    /// Unique identifier (e.g. "house_garage", "forest", "lake_surrounding").
    /// CDDA can use a single string or an array of strings.
    #[serde(default)]
    pub id: StringOrArray,

    /// Display name.
    #[serde(default)]
    pub name: Option<LocalizedString>,

    /// Symbol used on the overmap.
    #[serde(default = "default_symbol")]
    pub symbol: String,

    /// Color on the overmap.
    /// Can be a string name or a structured color object.
    #[serde(default)]
    pub color: Option<RawValue>,

    /// Look-alike for display.
    /// Can be a string ID or an object.
    #[serde(default)]
    pub looks_like: Option<RawValue>,

    /// Flags for overmap generation.
    /// CDDA can use a single string or an array of strings.
    #[serde(default)]
    pub flags: StringOrArray,

    /// Spawn chance for this OMT.
    #[serde(default)]
    pub spawn_chance: Option<u32>,

    /// Map extras that can appear on this OMT.
    /// Can be a string or an object.
    #[serde(default)]
    pub extras: Option<RawValue>,

    /// Whether this is a city-building OMT.
    #[serde(default)]
    pub is_urban: Option<bool>,

    /// Whether this OMT has a basement.
    #[serde(default)]
    pub has_basement: Option<bool>,

    /// Mapgen ID(s) (references to mapgen definitions).
    /// Can be an array of strings or an array of objects like {"builtin": "forest"}.
    #[serde(default)]
    pub mapgen: Option<Vec<RawValue>>,

    /// Tiles with lighting for night display.
    #[serde(default)]
    pub light: Option<Vec<String>>,

    /// Side (for OMTs that are building sides).
    #[serde(default)]
    pub side: Option<OvermapSide>,

    /// Connection to other OMTs.
    /// Can be a string or an object.
    #[serde(default)]
    pub connect_to: Option<RawValue>,

    /// Which overmap connections are valid.
    #[serde(default)]
    pub connections: Option<Vec<OmtConnection>>,

    /// Land use code — can be a string or an object.
    #[serde(default)]
    pub land_use_code: Option<RawValue>,

    /// Spawns — can be a string ID or an object with group/population/chance.
    #[serde(default)]
    pub spawns: Option<RawValue>,

    /// Monster density
    #[serde(default)]
    pub mondensity: Option<u32>,

    /// Travel cost type — can be a string or an object.
    #[serde(default)]
    pub travel_cost_type: Option<RawValue>,

    /// See cost (visibility) — CDDA uses string values like "high", "none", "low"
    #[serde(default)]
    pub see_cost: Option<SeeCost>,

    /// Vision levels — can be a string like `"always_full"` or a structured object.
    #[serde(default)]
    pub vision_levels: Option<VisionLevelsOrString>,

    /// Symbol (short form) — can be a string or an object.
    #[serde(default)]
    pub sym: Option<RawValue>,

    /// Abstract flag
    #[serde(default)]
    pub abstract_: Option<bool>,

    /// copy-from parent
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_from: Option<String>,
}

/// Vision levels — can be a named string (e.g. `"always_full"`) or a structured object
/// with `low`, `normal`, and `max` fields.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum VisionLevelsOrString {
    /// A named vision level string (e.g. `"always_full"`, `"none"`).
    Named(String),
    /// Structured vision level object with low/normal/max fields.
    Structured(crate::raw_defs::cdda_types::VisionLevels),
}

fn default_symbol() -> String {
    ".".to_string()
}

/// Side of a building for multi-tile structures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum OvermapSide {
    #[serde(rename = "north")]
    North,
    #[serde(rename = "south")]
    South,
    #[serde(rename = "east")]
    East,
    #[serde(rename = "west")]
    West,
}

/// Connection to adjacent OMTs.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OmtConnection {
    /// Connection type.
    pub connection: Option<String>,
    /// Which sides have the connection.
    pub sides: Option<Vec<String>>,
}

/// Overmap special definition from JSON type `"overmap_special"`.
///
/// Defines a special placement of OMTs (e.g. a "mansion" that spans multiple tiles).
/// A connection defined within an overmap special (e.g. road connection).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SpecialConnection {
    /// Position offset from special origin [dx, dy, dz].
    pub point: [i32; 3],
    /// Terrain to place for the connection (e.g. "road").
    #[serde(default)]
    pub terrain: Option<String>,
    /// Connection type ID (e.g. "local_road").
    #[serde(default)]
    pub connection: Option<String>,
    /// Hint direction from [dx, dy, dz].
    #[serde(default)]
    pub from: Option<[i32; 3]>,
    /// Whether the connection already exists at this location.
    #[serde(default)]
    pub existing: bool,
}

/// Monster spawn configuration within an overmap special.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SpecialSpawns {
    /// Monster group to spawn.
    #[serde(default)]
    pub group: Option<String>,
    /// Population range [min, max].
    #[serde(default)]
    pub population: Option<[i32; 2]>,
    /// Radius range [min, max].
    #[serde(default)]
    pub radius: Option<[i32; 2]>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OvermapSpecialDef {
    /// Unique identifier.
    pub id: DefId<OvermapSpecialDef>,

    /// Display name.
    #[serde(default)]
    pub name: Option<LocalizedString>,

    /// Overmap terrains that make up this special.
    /// For standard specials: an array of SpecialOmt objects or strings.
    /// For mutable specials: an object/map with named overmap entries.
    #[serde(default)]
    pub overmaps: Option<RawValue>,

    /// Locations this special can spawn in.
    /// Can be a single string (e.g. `"forest"`) or an array of strings.
    #[serde(default)]
    pub locations: StringOrArray,

    /// City distance range [min, max].
    #[serde(default)]
    pub city_distance: Option<[i32; 2]>,

    /// City size range [min, max].
    #[serde(default)]
    pub city_sizes: Option<[i32; 2]>,

    /// Chance of occurrence.
    #[serde(default)]
    pub occurrences: Option<[i32; 2]>,

    /// Flags.
    /// CDDA can use a single string or an array of strings.
    #[serde(default)]
    pub flags: StringOrArray,

    /// Rotations allowed.
    /// CDDA can use a single string or an array of strings.
    #[serde(default)]
    pub rotations: StringOrArray,

    /// Connections to roads/railroads from this special.
    /// Each connection specifies a point, terrain, and connection type.
    #[serde(default)]
    pub connections: Option<Vec<SpecialConnection>>,

    /// Subtype: "fixed" (default) or "mutable" for procedural placement.
    #[serde(default)]
    pub subtype: Option<String>,

    /// Mutable special: join definitions (array of join objects/strings).
    #[serde(default)]
    pub joins: Option<RawValue>,

    /// Mutable special: name of the root overmap entry in `overmaps`.
    #[serde(default)]
    pub root: Option<String>,

    /// Mutable special: phases (array of phase objects, each with `rules`).
    #[serde(default)]
    pub phases: Option<RawValue>,

    /// Priority for placement ordering (lower = earlier).
    #[serde(default)]
    pub priority: Option<i32>,

    /// Whether rotation is allowed.
    #[serde(default = "default_rotate")]
    pub rotate: bool,

    /// Monster spawns associated with this special.
    #[serde(default)]
    pub spawns: Option<SpecialSpawns>,

    /// Effect-on-condition triggered on placement.
    #[serde(default)]
    pub eoc: Option<RawValue>,
}

fn default_rotate() -> bool { true }

/// A single OMT reference within an overmap special.
/// CDDA can use a plain string (OMT ID) or an object with `overmap` and `point`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum SpecialOmtOrString {
    /// Just the OMT ID as a string.
    Id(String),
    /// Full object with overmap ID and point.
    Obj(SpecialOmt),
}

/// A single OMT within an overmap special.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SpecialOmt {
    /// OMT ID.
    pub overmap: String,
    /// Position offset from special origin [dx, dy, dz].
    pub point: [i32; 3],
    /// Required locations for each position.
    /// Can be a single string (e.g. `"land"`) or an array of strings.
    #[serde(default)]
    pub locations: Option<StringOrArray>,
}

/// Overmap connection definition from JSON type `"overmap_connection"`.
///
/// Defines how different OMTs connect (e.g. "road" connecting "forest" and "house").
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OvermapConnectionDef {
    /// Unique identifier.
    pub id: DefId<OvermapConnectionDef>,

    /// Terrain types this connection can travel through.
    #[serde(default)]
    pub terrains: Vec<ConnectionTerrain>,

    /// Default terrain subtype.
    #[serde(default)]
    pub default_terrain: Option<String>,

    /// Subtype connections.
    #[serde(default)]
    pub subtypes: Option<Vec<ConnectionSubtype>>,
}

/// Terrain type in a connection.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ConnectionTerrain {
    /// OMT ID or list.
    pub terrain: StringOrArray,
    /// Whether connection is blocked.
    #[serde(default)]
    pub is_connector: Option<bool>,
    /// Chance of connecting.
    #[serde(default)]
    pub connect_chance: Option<u32>,
}

/// Subtype of a connection.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ConnectionSubtype {
    /// Terrain ID.
    pub terrain: String,
    /// Valid locations for this connection subtype.
    #[serde(default)]
    pub locations: Vec<String>,
    /// Basic cost for travel.
    #[serde(default)]
    pub basic_cost: Option<u32>,
    /// Flags for this connection subtype.
    #[serde(default)]
    pub flags: Vec<String>,
}

/// Overmap location definition from JSON type `"overmap_location"`.
///
/// Defines a named location type for overmap generation (e.g. "land", "water", "forest").
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OvermapLocationDef {
    /// Unique identifier.
    pub id: DefId<OvermapLocationDef>,

    /// Terrain flags that define this location.
    /// CDDA can use a single string or an array of strings.
    #[serde(default)]
    pub terrains: Option<StringOrArray>,

    /// Flags.
    /// CDDA can use a single string or an array of strings.
    #[serde(default)]
    pub flags: StringOrArray,
}

/// Overmap land use code from JSON type `"overmap_land_use_code"`.
///
/// Defines how land is used on the overmap (e.g. "residential", "commercial", "rural").
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OvermapLandUseCodeDef {
    /// Unique identifier.
    pub id: DefId<OvermapLandUseCodeDef>,

    /// Display name.
    #[serde(default)]
    pub name: Option<LocalizedString>,

    /// Symbol.
    #[serde(default)]
    pub symbol: Option<String>,

    /// Color.
    #[serde(default)]
    pub color: Option<String>,
}
