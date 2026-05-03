use crate::raw_defs::cdda_types::{CddaColor, ExamineAction, RawValue, StringOrArray};
use crate::raw_types::{DefId, LocalizedString};
use cdda_core::units::Weight;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A furniture definition from JSON type `"furniture"`.
///
/// Furniture is placed on top of terrain (e.g. chairs, tables, counters).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FurnitureDef {
    /// Unique identifier (e.g. "f_chair", "f_table").
    pub id: DefId<FurnitureDef>,

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
    #[serde(default)]
    pub color: Option<CddaColor>,

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
    /// Can be a string ID or an object.
    #[serde(default)]
    pub crafting_pseudo_item: Option<RawValue>,

    /// Examine action triggered when examining this furniture.
    #[serde(default)]
    pub examine_action: Option<ExamineAction>,

    /// Flags.
    /// CDDA can use a single string or an array of strings.
    #[serde(default)]
    pub flags: StringOrArray,

    /// Amount of light emitted.
    #[serde(default)]
    pub light_emitted: Option<u32>,

    /// Maximum brightness this furniture can emit.
    #[serde(default)]
    pub max_light_emitted: Option<u32>,

    /// Bash result.
    /// Can be a string (reference) or an object.
    #[serde(default)]
    pub bash: Option<RawValue>,

    /// Deconstruction result.
    /// Can be an object or an array of objects.
    #[serde(default)]
    pub deconstruct: Option<RawValue>,

    /// What this furniture becomes when burned.
    /// Can be a string ID or an object.
    #[serde(default)]
    pub burn_into: Option<RawValue>,

    /// Shoot action — can be a string, number, array, or object.
    #[serde(default)]
    pub shoot: Option<RawValue>,

    /// Close action — can be a string, number, array, or object.
    #[serde(default)]
    pub close: Option<RawValue>,

    /// Open action — can be a string, number, array, or object.
    #[serde(default)]
    pub open: Option<RawValue>,

    /// Floor bedding warmth (can be negative for cooling furniture).
    #[serde(default)]
    pub floor_bedding_warmth: Option<i32>,

    /// Deployed item
    /// Can be a string ID or an object.
    #[serde(default)]
    pub deployed_item: Option<RawValue>,

    /// Rotates to terrain/furniture
    #[serde(default)]
    pub rotates_to: StringOrArray,

    /// Spawned item
    /// Can be a string ID or an object.
    #[serde(default)]
    pub item: Option<RawValue>,

    /// Comfort level (can be negative for uncomfortable furniture).
    #[serde(default)]
    pub comfort: Option<i32>,

    /// Maximum volume (for containers)
    #[serde(default)]
    pub max_volume: Option<cdda_core::units::Volume>,

    /// Connection group(s) this furniture connects to.
    /// Can be a single string or an array of strings.
    #[serde(default)]
    pub connects_to: Option<StringOrArray>,

    /// Connection groups this furniture belongs to.
    /// Can be a single string or an array of strings.
    #[serde(default)]
    pub connect_groups: Option<StringOrArray>,

    /// Background color
    /// Can be a single color string or an array like ["white"].
    #[serde(default)]
    pub bgcolor: Option<StringOrArray>,

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
