use crate::types::{DefId, LocalizedString};
use serde::{Deserialize, Serialize};

/// Overmap terrain definition from JSON type `"overmap_terrain"`.
///
/// Defines a 24×24 tile region on the overmap (e.g. "house", "forest", "lake").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OvermapTerrainDef {
    /// Unique identifier (e.g. "house_garage", "forest", "lake_surrounding").
    pub id: DefId<OvermapTerrainDef>,

    /// Display name.
    pub name: LocalizedString,

    /// Symbol used on the overmap.
    #[serde(default = "default_symbol")]
    pub symbol: String,

    /// Color on the overmap.
    #[serde(default)]
    pub color: Option<String>,

    /// Look-alike for display.
    #[serde(default)]
    pub looks_like: Option<String>,

    /// Flags for overmap generation.
    #[serde(default)]
    pub flags: Vec<String>,

    /// Spawn chance for this OMT.
    #[serde(default)]
    pub spawn_chance: Option<u32>,

    /// Map extras that can appear on this OMT.
    #[serde(default)]
    pub extras: Option<String>,

    /// Whether this is a city-building OMT.
    #[serde(default)]
    pub is_urban: Option<bool>,

    /// Whether this OMT has a basement.
    #[serde(default)]
    pub has_basement: Option<bool>,

    /// Mapgen ID(s) (references to mapgen definitions).
    #[serde(default)]
    pub mapgen: Option<Vec<String>>,

    /// Tiles with lighting for night display.
    #[serde(default)]
    pub light: Option<Vec<String>>,

    /// Side (for OMTs that are building sides).
    #[serde(default)]
    pub side: Option<OvermapSide>,

    /// Connection to other OMTs.
    #[serde(default)]
    pub connect_to: Option<String>,

    /// Which overmap connections are valid.
    #[serde(default)]
    pub connections: Option<Vec<OmtConnection>>,

    /// Catch-all
    #[serde(default)]
    pub extra: Option<serde_json::Value>,

    /// Land use code
    #[serde(default)]
    pub land_use_code: Option<String>,

    /// Spawns
    #[serde(default)]
    pub spawns: Option<serde_json::Value>,

    /// Monster density
    #[serde(default)]
    pub mondensity: Option<u32>,

    /// Travel cost type
    #[serde(default)]
    pub travel_cost_type: Option<String>,

    /// See cost (visibility) — CDDA uses string values like "high", "none", "low"
    #[serde(default)]
    pub see_cost: Option<serde_json::Value>,

    /// Vision levels
    #[serde(default)]
    pub vision_levels: Option<serde_json::Value>,

    /// Symbol (short form)
    #[serde(default)]
    pub sym: Option<String>,

    /// Delete operations
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delete: Option<serde_json::Value>,

    /// Extend operations
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extend: Option<serde_json::Value>,

    /// Abstract flag
    #[serde(default)]
    pub abstract_: Option<bool>,

    /// copy-from parent
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_from: Option<String>,
}

fn default_symbol() -> String {
    ".".to_string()
}

/// Side of a building for multi-tile structures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmtConnection {
    /// Connection type.
    pub connection: Option<String>,
    /// Which sides have the connection.
    pub sides: Option<Vec<String>>,
}

/// Overmap special definition from JSON type `"overmap_special"`.
///
/// Defines a special placement of OMTs (e.g. a "mansion" that spans multiple tiles).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OvermapSpecialDef {
    /// Unique identifier.
    pub id: DefId<OvermapSpecialDef>,

    /// Overmap terrains that make up this special.
    pub overmaps: Vec<SpecialOmt>,

    /// Locations this special can spawn in.
    #[serde(default)]
    pub locations: Vec<String>,

    /// City distance range [min, max].
    #[serde(default)]
    pub city_distance: Option<[u32; 2]>,

    /// City size range [min, max].
    #[serde(default)]
    pub city_sizes: Option<[u32; 2]>,

    /// Chance of occurrence.
    #[serde(default)]
    pub occurrences: Option<[u32; 2]>,

    /// Flags.
    #[serde(default)]
    pub flags: Vec<String>,

    /// Rotations allowed.
    #[serde(default)]
    pub rotations: Vec<String>,
}

/// A single OMT within an overmap special.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialOmt {
    /// OMT ID.
    pub overmap: String,
    /// Position offset from special origin [dx, dy, dz].
    pub point: [i32; 3],
    /// Required locations for each position.
    pub locations: Option<Vec<String>>,
}

/// Overmap connection definition from JSON type `"overmap_connection"`.
///
/// Defines how different OMTs connect (e.g. "road" connecting "forest" and "house").
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionTerrain {
    /// OMT ID or list.
    pub terrain: serde_json::Value,
    /// Whether connection is blocked.
    #[serde(default)]
    pub is_connector: Option<bool>,
    /// Chance of connecting.
    #[serde(default)]
    pub connect_chance: Option<u32>,
}

/// Subtype of a connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OvermapLocationDef {
    /// Unique identifier.
    pub id: DefId<OvermapLocationDef>,

    /// Terrain flags that define this location.
    pub terrains: Vec<String>,

    /// Flags.
    #[serde(default)]
    pub flags: Vec<String>,
}

/// Overmap land use code from JSON type `"overmap_land_use_code"`.
///
/// Defines how land is used on the overmap (e.g. "residential", "commercial", "rural").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OvermapLandUseCodeDef {
    /// Unique identifier.
    pub id: DefId<OvermapLandUseCodeDef>,

    /// Display name.
    pub name: LocalizedString,

    /// Symbol.
    #[serde(default)]
    pub symbol: Option<String>,

    /// Color.
    #[serde(default)]
    pub color: Option<String>,
}
