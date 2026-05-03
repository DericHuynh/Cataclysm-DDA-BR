use crate::raw_defs::cdda_types::{CddaColor, RawValue};
use crate::raw_types::{DefId, LocalizedString};
use cdda_core::units::{Volume, Weight};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
/// A vehicle part definition from JSON type "vehicle_part".
///
/// Defines a component that can be installed on a vehicle (e.g. engine, wheel, seat, battery).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VehiclePartDef {
    /// Unique identifier (e.g. "diesel_engine", "wheel_wide").
    pub id: DefId<VehiclePartDef>,

    /// Display name.
    #[serde(default)]
    pub name: Option<LocalizedString>,

    /// Description text.
    #[serde(default)]
    pub description: Option<LocalizedString>,

    /// Symbol on the map.
    #[serde(default = "default_symbol")]
    pub symbol: String,

    /// Color for display.
    #[serde(default)]
    pub color: Option<CddaColor>,

    /// Symbol when part is broken.
    #[serde(default)]
    pub symbol_broken: Option<String>,

    /// Color when part is broken.
    #[serde(default)]
    pub color_broken: Option<CddaColor>,

    /// Aesthetics category.
    #[serde(default)]
    pub looks_like: Option<DefId<VehiclePartDef>>,

    /// Categories this part belongs to.
    #[serde(default)]
    pub categories: Vec<DefId<VehiclePartCategoryDef>>,

    /// Location on the vehicle (e.g. "on_roof", "under").
    #[serde(default)]
    pub location: Option<DefId<VehiclePartLocationDef>>,

    /// Part durability / hit points.
    #[serde(default = "default_durability")]
    pub durability: u32,

    /// Damage modifier (armor).
    #[serde(default)]
    pub damage_modifier: Option<u32>,

    /// Damage reduction (e.g. {"all": 20} or {"bash": 5, "cut": 3}).
    #[serde(default)]
    pub damage_reduction: Option<HashMap<String, u32>>,

    /// Width of the part.
    #[serde(default)]
    pub width: Option<u32>,

    /// Required strength to install.
    #[serde(default)]
    pub install_skills: Option<Vec<VehiclePartSkillReq>>,

    /// Time to install.
    #[serde(default)]
    pub install_time: Option<u32>,

    /// Requirements for install/removal/repair (object with nested operation requirements).
    #[serde(default)]
    pub requirements: Option<VehiclePartRequirements>,

    /// Foldable: folded volume.
    #[serde(default)]
    pub folded_volume: Option<Volume>,

    /// Size / volume of the part.
    #[serde(default)]
    pub size: Option<Volume>,

    /// Mass / weight of the part.
    #[serde(default)]
    pub mass: Option<Weight>,

    /// Fuel type this part uses.
    #[serde(default)]
    pub fuel_type: Option<String>,

    /// Engine power (for engine parts).
    #[serde(default)]
    pub power: Option<cdda_core::units::Energy>,

    /// Energy consumption rate.
    #[serde(default)]
    pub energy_consumption: Option<cdda_core::units::Energy>,

    /// Description of what this part does (fuel consumption, power generation, etc).
    #[serde(default)]
    pub description_extra: Option<String>,

    /// Whether part breaks first. Can be a string ID or array of break entries.
    #[serde(default)]
    pub breaks_into: Option<BreaksInto>,

    /// Flags.
    #[serde(default)]
    pub flags: Vec<String>,

    /// Items used to repair this part.
    #[serde(default)]
    pub repair_item: Option<String>,

    /// Cannot be removed if true.
    #[serde(default)]
    pub prohibited: Option<bool>,

    /// Removed when folded.
    #[serde(default)]
    pub remove_folded: Option<bool>,

    /// Whether part is a standard variant.
    #[serde(default)]
    pub standard: Option<bool>,

    /// Whether part is a military variant.
    #[serde(default)]
    pub military: Option<bool>,

    /// Fuel capacity.
    #[serde(default)]
    pub fuel_capacity: Option<u64>,

    /// Cargo capacity.
    #[serde(default)]
    pub cargo_capacity: Option<Volume>,

    /// Coverage percentage.
    #[serde(default)]
    pub coverage: Option<u32>,

    /// Wheel diameter (for wheels).
    #[serde(default)]
    pub wheel_diameter: Option<u32>,

    /// Wheel width (for wheels).
    #[serde(default)]
    pub wheel_width: Option<u32>,

    /// Roller bearing type.
    #[serde(default)]
    pub rolling_resistance: Option<u32>,

    /// Seat belt type.
    #[serde(default)]
    pub belt: Option<String>,

    /// Seat type.
    #[serde(default)]
    pub seat: Option<String>,

    /// Engine type.
    #[serde(default)]
    pub engine_type: Option<String>,

    /// Backup camera.
    #[serde(default)]
    pub has_backup_camera: Option<bool>,

    /// Backup camera cover.
    #[serde(default)]
    pub backup_camera_cover: Option<u32>,

    /// Whether this part emits light.
    #[serde(default)]
    pub emission: Option<Vec<VehiclePartEmission>>,

    /// Whether this part generates exhaust.
    #[serde(default)]
    pub exhaust: Option<Vec<String>>,

    /// Whether this part can be used as a seat.
    #[serde(default)]
    pub seat_type: Option<String>,

    /// Contact area (for wheels)
    #[serde(default)]
    pub contact_area: Option<u32>,

    /// Pseudo tools
    #[serde(default)]
    pub pseudo_tools: Option<Vec<String>>,

    /// Bonus stats
    #[serde(default)]
    pub bonus: Option<crate::raw_defs::cdda_types::VehiclePartBonus>,

    /// Electrical power consumption/generation
    #[serde(default)]
    pub epower: Option<i32>,

    /// Broken color
    #[serde(default)]
    pub broken_color: Option<CddaColor>,

    /// Variants (array of variant objects).
    #[serde(default)]
    pub variants: Option<Vec<VehiclePartVariant>>,

    /// Item from which this part is made
    #[serde(default)]
    pub item: Option<String>,

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

fn default_durability() -> u32 {
    100
}

/// Breaks into can be a simple item group ID string or an array of break entries.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum BreaksInto {
    /// Simple item group ID: `"ig_vp_frame"`
    Group(String),
    /// Array of break entries: `[{ "item": "chunk", "count": [0, 2] }]`
    Items(Vec<VehiclePartBreak>),
}

/// Requirements for installing/removing/repairing a vehicle part.
/// CDDA format: `{"install": {...}, "removal": {...}, "repair": {...}}`
/// Uses RawValue for the whole struct since CDDA formats are complex.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VehiclePartRequirements {
    /// Install operation requirements.
    #[serde(default)]
    pub install: Option<RawValue>,

    /// Removal operation requirements.
    #[serde(default)]
    pub removal: Option<RawValue>,

    /// Repair operation requirements.
    #[serde(default)]
    pub repair: Option<RawValue>,
}

/// A skill requirement for vehicle part installation.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VehiclePartSkillReq {
    pub skill: String,
    pub level: u32,
}

/// Items dropped when a vehicle part breaks.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VehiclePartBreak {
    pub item: String,
    /// Item count range [min, max].
    #[serde(default)]
    pub count: Option<[u32; 2]>,
    /// Item charges range [min, max].
    #[serde(default)]
    pub charges: Option<[u32; 2]>,
    /// Probability (percentage).
    #[serde(default)]
    pub prob: Option<u32>,
    /// Container item.
    #[serde(default, rename = "container-item")]
    pub container_item: Option<String>,
}

/// A vehicle part variant for different visual styles.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VehiclePartVariant {
    /// Variant ID (optional).
    #[serde(default)]
    pub id: Option<String>,
    /// Display label.
    #[serde(default)]
    pub label: Option<String>,
    /// Symbol string (can be a single char or multi-character string).
    #[serde(default)]
    pub symbols: Option<RawValue>,
    /// Symbol when broken.
    #[serde(default)]
    pub symbols_broken: Option<RawValue>,
}

/// Emission from a vehicle part.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VehiclePartEmission {
    pub id: String,
    #[serde(default)]
    pub rate: Option<u32>,
}

/// Vehicle part location definition from JSON type `"vehicle_part_location"`.
///
/// Defines where on a vehicle a part can be mounted (e.g. "on_mount", "under", "on_roof").
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VehiclePartLocationDef {
    /// Unique identifier.
    pub id: DefId<VehiclePartLocationDef>,

    /// Display name.
    pub name: LocalizedString,

    /// Format strings for display.
    #[serde(default)]
    pub hotplate_temperature: Option<i32>,

    /// Flags.
    #[serde(default)]
    pub flags: Vec<String>,
}

/// Vehicle part category definition from JSON type `"vehicle_part_category"`.
///
/// Defines a category of vehicle parts (e.g. "engine", "wheel", "cargo").
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VehiclePartCategoryDef {
    /// Unique identifier.
    pub id: DefId<VehiclePartCategoryDef>,

    /// Display name.
    pub name: LocalizedString,

    /// Short name for UI.
    #[serde(default)]
    pub short_name: Option<crate::raw_types::LocalizedString>,

    /// Priority for UI sorting.
    #[serde(default)]
    pub priority: Option<i32>,
}
