use crate::raw_defs::cdda_types::{CddaColor, CountRange, ExamineAction, RawValue, StringOrArray};
use crate::raw_types::{DefId, LocalizedString};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A terrain definition from JSON type `"terrain"`.
///
/// Terrain is the base ground / wall / floor that makes up the map.
/// Every tile must have exactly one terrain.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TerrainDef {
    /// Unique identifier (e.g. "t_floor", "t_wall_brick").
    pub id: DefId<TerrainDef>,

    /// Display name.
    #[serde(default)]
    pub name: Option<LocalizedString>,

    /// Description text.
    #[serde(default)]
    pub description: Option<LocalizedString>,

    /// ASCII symbol on the map.
    #[serde(default = "default_symbol")]
    pub symbol: String,

    /// Color for map display.
    /// CDDA variations: "red", ["white", "light_gray"], {"fg": "red", "bg": "blue"}
    #[serde(default)]
    pub color: Option<CddaColor>,

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
    /// CDDA can use a single string ("WALL") or an array (["WALL", "NOITEM"]).
    #[serde(default)]
    pub flags: StringOrArray,

    /// Connection groups for automatic terrain transitions.
    /// Can be a string or array of strings.
    #[serde(default)]
    pub connect_groups: StringOrArray,

    /// Which connection group this terrain connects to.
    /// Can be a string or array of strings.
    #[serde(default)]
    pub connects_to: StringOrArray,

    /// Whether this tile has a ceiling (for z-level checks).
    #[serde(default)]
    pub has_ceiling: Option<bool>,

    /// Exterior tile (what it looks like from outside).
    #[serde(default)]
    pub exterior: Option<DefId<TerrainDef>>,

    /// Trap that appears when this terrain is disturbed.
    #[serde(default)]
    pub trap: Option<RawValue>,

    /// Bash result data.
    /// CDDA can use a string ("wall_bash_results") or an object.
    #[serde(default)]
    pub bash: Option<TerrainBashOrString>,

    /// Deconstruction result data.
    /// Can be an object or an array of objects.
    #[serde(default)]
    pub deconstruct: Option<RawValue>,

    /// Harvest result data.
    /// Can be a string ID (e.g. `"harvest_id"`) or an object (e.g. `{"id": "...", "message": "..."}`).
    #[serde(default)]
    pub harvest: Option<RawValue>,

    /// Shoot action — values can be strings, numbers, arrays (e.g. `[15, 30]`), or objects.
    #[serde(default)]
    pub shoot: Option<RawValue>,

    /// Close action — values can be strings, numbers, arrays (e.g. `[15, 30]`), or objects.
    #[serde(default)]
    pub close: Option<RawValue>,

    /// Open action — values can be strings, numbers, arrays (e.g. `[15, 30]`), or objects.
    #[serde(default)]
    pub open: Option<RawValue>,

    /// Transforms into terrain
    #[serde(default)]
    pub transforms_into: Option<String>,

    /// Rotates to terrain
    #[serde(default)]
    pub rotates_to: StringOrArray,

    /// Examine action (can be a string like "cardreader" or an object).
    #[serde(default)]
    pub examine_action: Option<ExamineAction>,

    /// Coverage percentage
    #[serde(default)]
    pub coverage: Option<u32>,

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

/// Bash result data — CDDA can reference a bash group by name (string) or inline (object).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum TerrainBashOrString {
    /// Reference to a named bash result group (e.g. "wall_bash_results").
    Reference(String),
    /// Inline bash result object.
    Inline(TerrainBash),
}

/// Result of bashing this terrain.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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

    /// Volume of the fail sound.
    #[serde(default)]
    pub sound_fail_vol: Option<u32>,

    /// Terrain this becomes after successful bash.
    #[serde(default)]
    pub ter_set: Option<DefId<TerrainDef>>,

    /// Furniture that spawns after bash.
    #[serde(default)]
    pub furn_set: Option<DefId<crate::raw_defs::furniture::FurnitureDef>>,

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
    /// Can be a string (group reference) or an array of BashItemDrop objects.
    #[serde(default)]
    pub items: Option<RawValue>,
}

/// An item dropped when bashing a tile.
/// CDDA format can use `"item"` (item ID) or `"group"` (item group ID).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BashItemDrop {
    /// Item type to drop.
    #[serde(default)]
    pub item: Option<DefId<crate::raw_defs::item::ItemDef>>,
    /// Item group reference (alternative to `item`).
    #[serde(default)]
    pub group: Option<String>,
    /// Count range [min, max] or single value.
    #[serde(default)]
    pub count: CountRange,
    /// Chance (as percentage).
    #[serde(default)]
    pub prob: Option<u32>,
}
