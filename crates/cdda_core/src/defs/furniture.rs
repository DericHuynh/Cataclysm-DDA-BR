use crate::types::{DefId, LocalizedString};
use crate::units::Weight;
use serde::{Deserialize, Serialize};

/// A furniture definition from JSON type `"furniture"`.
///
/// Furniture is placed on top of terrain (e.g. chairs, tables, counters).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FurnitureDef {
    /// Unique identifier (e.g. "f_chair", "f_table").
    pub id: DefId<FurnitureDef>,

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

    /// Another furniture this one "looks like".
    #[serde(default)]
    pub looks_like: Option<DefId<FurnitureDef>>,

    /// Movement cost modifier (added to terrain move_cost).
    #[serde(default)]
    pub move_cost_mod: Option<i32>,

    /// Coverage percentage (for cover in combat).
    #[serde(default)]
    pub coverage: Option<u32>,

    /// Required strength to move / interact.
    #[serde(default)]
    pub required_str: Option<i32>,

    /// Mass of the furniture object.
    #[serde(default)]
    pub mass: Option<Weight>,

    /// A pseudo item used for crafting at this furniture.
    #[serde(default)]
    pub crafting_pseudo_item: Option<DefId<crate::defs::item::ItemDef>>,

    /// Examine action triggered when examining this furniture.
    #[serde(default)]
    pub examine_action: Option<String>,

    /// Flags.
    #[serde(default)]
    pub flags: Vec<String>,

    /// Amount of light emitted.
    #[serde(default)]
    pub light_emitted: Option<u32>,

    /// Maximum brightness this furniture can emit.
    #[serde(default)]
    pub max_light_emitted: Option<u32>,

    /// Bash result.
    #[serde(default)]
    pub bash: Option<FurnitureBash>,

    /// Deconstruction result.
    #[serde(default)]
    pub deconstruct: Option<FurnitureDeconstruct>,

    /// What this furniture becomes when burned.
    #[serde(default)]
    pub burn_into: Option<DefId<FurnitureDef>>,

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

    /// Floor bedding warmth
    #[serde(default)]
    pub floor_bedding_warmth: Option<u32>,

    /// Deployed item
    #[serde(default)]
    pub deployed_item: Option<String>,

    /// Rotates to furniture
    #[serde(default)]
    pub rotates_to: Option<Vec<String>>,

    /// Spawned item
    #[serde(default)]
    pub item: Option<String>,

    /// Comfort level
    #[serde(default)]
    pub comfort: Option<u32>,

    /// Maximum volume (for containers)
    #[serde(default)]
    pub max_volume: Option<crate::units::Volume>,

    /// Connects to group
    #[serde(default)]
    pub connects_to: Option<String>,

    /// Connection groups
    #[serde(default)]
    pub connect_groups: Option<Vec<String>>,

    /// Background color
    #[serde(default)]
    pub bgcolor: Option<String>,

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
    "#".to_string()
}

/// Bash result for furniture.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FurnitureBash {
    /// Sound made when bashing.
    #[serde(default)]
    pub sound: Option<String>,

    /// Volume of the sound.
    #[serde(default)]
    pub sound_vol: Option<u32>,

    /// Sound made when bash fails.
    #[serde(default)]
    pub sound_fail: Option<String>,

    /// Volume of the fail sound.
    #[serde(default)]
    pub sound_fail_vol: Option<u32>,

    /// Terrain this becomes after bash.
    #[serde(default)]
    pub ter_set: Option<DefId<crate::defs::terrain::TerrainDef>>,

    /// Furniture this becomes after bash.
    #[serde(default)]
    pub furn_set: Option<DefId<FurnitureDef>>,

    /// Minimum strength to bash.
    #[serde(default)]
    pub str_min: u32,

    /// Maximum strength for randomization.
    #[serde(default)]
    pub str_max: u32,

    /// Items dropped.
    #[serde(default)]
    pub items: Option<Vec<FurnitureBashItem>>,
}

/// Item dropped from bashing furniture.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FurnitureBashItem {
    pub item: String,
    #[serde(default)]
    pub count: Option<[u32; 2]>,
    #[serde(default)]
    pub prob: Option<u32>,
}

/// Deconstruction result for furniture.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FurnitureDeconstruct {
    /// Items returned.
    pub items: Vec<FurnitureDeconItem>,
    /// Furniture set after deconstruction.
    #[serde(default)]
    pub furn_set: Option<DefId<FurnitureDef>>,
}

/// An item from deconstructing furniture.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FurnitureDeconItem {
    pub item: String,
    #[serde(default)]
    pub count: Option<u32>,
}
