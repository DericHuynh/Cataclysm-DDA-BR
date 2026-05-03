use crate::types::{DefId, LocalizedString};
use serde::{Deserialize, Serialize};

/// A terrain definition from JSON type `"terrain"`.
///
/// Terrain is the base ground / wall / floor that makes up the map.
/// Every tile must have exactly one terrain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainDef {
    /// Unique identifier (e.g. "t_floor", "t_wall_brick").
    pub id: DefId<TerrainDef>,

    /// Display name.
    pub name: LocalizedString,

    /// Description text.
    pub description: LocalizedString,

    /// ASCII symbol on the map.
    #[serde(default = "default_symbol")]
    pub symbol: String,

    /// Color for map display.
    #[serde(default)]
    pub color: Option<String>,

    /// Another terrain this one "looks like" for display purposes.
    #[serde(default)]
    pub looks_like: Option<DefId<TerrainDef>>,

    /// Movement cost to enter this tile (0 = impassable).
    #[serde(default)]
    pub move_cost: i32,

    /// Speed penalty / multiplier when moving through.
    #[serde(default)]
    pub movecost: Option<i32>,

    /// Roof terrain above this tile.
    #[serde(default)]
    pub roof: Option<DefId<TerrainDef>>,

    /// Light emitted by this tile (for glowing terrain).
    #[serde(default)]
    pub light_emitted: Option<u32>,

    /// Light color emitted.
    #[serde(default)]
    pub light_color: Option<[u8; 3]>,

    /// Flags.
    #[serde(default)]
    pub flags: Vec<String>,

    /// Connection groups for automatic terrain transitions.
    #[serde(default)]
    pub connect_groups: Vec<String>,

    /// Which connection group this terrain connects to.
    #[serde(default)]
    pub connects_to: Option<String>,

    /// Whether this tile has a ceiling (for z-level checks).
    #[serde(default)]
    pub has_ceiling: Option<bool>,

    /// Exterior tile (what it looks like from outside).
    #[serde(default)]
    pub exterior: Option<DefId<TerrainDef>>,

    /// Trap that appears when this terrain is disturbed.
    #[serde(default)]
    pub trap: Option<DefId<crate::defs::trap::TrapDef>>,

    /// Bash result data.
    #[serde(default)]
    pub bash: Option<TerrainBash>,

    /// Deconstruction result data.
    #[serde(default)]
    pub deconstruct: Option<DeconstructResult>,

    /// Harvest result data.
    #[serde(default)]
    pub harvest: Option<HarvestResult>,

    /// Catch-all
    #[serde(default)]
    pub extra: Option<serde_json::Value>,

    /// Shoot action
    #[serde(default)]
    pub shoot: Option<serde_json::Value>,

    /// Close action
    #[serde(default)]
    pub close: Option<serde_json::Value>,

    /// Open action
    #[serde(default)]
    pub open: Option<serde_json::Value>,

    /// Transforms into terrain
    #[serde(default)]
    pub transforms_into: Option<String>,

    /// Rotates to terrain
    #[serde(default)]
    pub rotates_to: Option<Vec<String>>,

    /// Examine action
    #[serde(default)]
    pub examine_action: Option<String>,

    /// Coverage percentage
    #[serde(default)]
    pub coverage: Option<u32>,

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

/// Result of bashing this terrain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainBash {
    /// Sound made when bashing.
    #[serde(default)]
    pub sound: Option<String>,

    /// Volume of the sound.
    #[serde(default)]
    pub sound_vol: Option<u32>,

    /// Sound made when bashing fails.
    #[serde(default)]
    pub sound_fail: Option<String>,

    /// Terrain this becomes after successful bash.
    #[serde(default)]
    pub ter_set: Option<DefId<TerrainDef>>,

    /// Furniture that spawns after bash.
    #[serde(default)]
    pub furn_set: Option<DefId<crate::defs::furniture::FurnitureDef>>,

    /// Minimum strength required to bash.
    #[serde(default)]
    pub str_min: u32,

    /// Maximum strength required (for randomization).
    #[serde(default)]
    pub str_max: u32,

    /// Minimum strength if supported.
    #[serde(default)]
    pub str_min_supported: Option<u32>,

    /// Items dropped when bashed.
    #[serde(default)]
    pub items: Option<Vec<BashItemDrop>>,
}

/// An item dropped when bashing a tile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BashItemDrop {
    /// Item type to drop.
    pub item: DefId<crate::defs::item::ItemDef>,
    /// Count range [min, max].
    #[serde(default)]
    pub count: Option<[u32; 2]>,
    /// Chance (as percentage).
    #[serde(default)]
    pub prob: Option<u32>,
}

/// Deconstruction result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeconstructResult {
    /// Items returned on deconstruction.
    #[serde(default)]
    pub items: Vec<DeconstructItem>,
    /// Terrain set after deconstruction.
    #[serde(default)]
    pub ter_set: Option<DefId<TerrainDef>>,
    /// Furniture set after deconstruction.
    #[serde(default)]
    pub furn_set: Option<DefId<crate::defs::furniture::FurnitureDef>>,
}

/// An item from deconstruction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeconstructItem {
    pub item: String,
    #[serde(default)]
    pub count: Option<u32>,
}

/// Harvest result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarvestResult {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
}
