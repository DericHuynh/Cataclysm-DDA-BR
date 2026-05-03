use crate::types::{DefId, LocalizedString};
use crate::units::{Length, Volume, Weight};
use serde::{Deserialize, Serialize};

/// How charges work for this item.
///
/// CDDA has long-standing bugs from confusing charges-count meaning;
/// `CountMode` makes this explicit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CountMode {
    /// Item is not a stackable/charged item.
    Single,
    /// Item that stacks by count (e.g. rocks, nails).
    ByCount {
        #[serde(default = "default_count")]
        default: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        max: Option<u32>,
    },
    /// Item that stores charges (e.g. battery, fuel, liquid).
    Charges {
        #[serde(default = "default_count")]
        default: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        max: Option<u32>,
    },
}

impl Default for CountMode {
    fn default() -> Self {
        CountMode::Single
    }
}

fn default_count() -> u32 {
    1
}

/// Phase of matter for an item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    #[serde(rename = "solid")]
    Solid,
    #[serde(rename = "liquid")]
    Liquid,
    #[serde(rename = "gas")]
    Gas,
    #[serde(rename = "plasma")]
    Plasma,
}

impl Default for Phase {
    fn default() -> Self {
        Phase::Solid
    }
}

/// Category of an item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemCategory {
    pub id: String,
    pub name: LocalizedString,
}

/// A single item definition from JSON type `"ITEM"`.
///
/// CDDA has many subtypes of ITEM (GUN, AMMO, COMESTIBLE, TOOL, etc.)
/// that all share this same struct with different fields populated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemDef {
    /// Unique string identifier.
    pub id: DefId<ItemDef>,

    /// Display name.
    pub name: LocalizedString,

    /// Description text.
    #[serde(default)]
    pub description: Option<serde_json::Value>,

    /// Volume of the item (in mL).
    #[serde(default = "default_volume")]
    pub volume: Volume,

    /// Weight of the item (in grams).
    #[serde(default)]
    pub weight: Option<Weight>,

    /// How charges work for this item.
    #[serde(default)]
    pub count_mode: CountMode,

    /// Category (e.g. "weapons", "ammo", "food").
    #[serde(default)]
    pub category: Option<String>,

    /// Materials this item is made from.
    #[serde(default)]
    pub material: Vec<DefId<crate::defs::material::MaterialDef>>,

    /// ASCII symbol for map display.
    #[serde(default = "default_symbol")]
    pub symbol: String,

    /// Color for map display (can be string, map, or array in CDDA data).
    #[serde(default)]
    pub color: Option<serde_json::Value>,

    /// Item price before the cataclysm.
    #[serde(default)]
    pub price: Option<serde_json::Value>,

    /// Item price after the cataclysm.
    #[serde(default)]
    pub price_postapoc: Option<serde_json::Value>,

    /// Flags applied to this item.
    #[serde(default)]
    pub flags: Vec<String>,

    /// If true, item is a pseudo item (used internally, not a real object).
    #[serde(default)]
    pub stackable: Option<bool>,

    /// Phase of matter.
    #[serde(default)]
    pub phase: Phase,

    /// Longest dimension of the item (for pocket fitting).
    #[serde(default)]
    pub longest_side: Option<Length>,

    /// Whether the item is rigid (volume doesn't change when empty).
    #[serde(default)]
    pub rigid: Option<bool>,

    /// Whether item is conductive (electric damage passes through).
    #[serde(default)]
    pub conductive: Option<bool>,

    /// Whether item covers the wearer's head (for helmets).
    #[serde(default)]
    pub covers_head: Option<bool>,

    /// Damage data for melee (can be object or number).
    #[serde(default)]
    pub melee_damage: Option<serde_json::Value>,

    /// If set, this item is a container with the given pocket data.
    #[serde(default)]
    pub pocket_data: Option<Vec<PocketDef>>,

    /// Materials this item can be crafted from (for recipes).
    #[serde(default)]
    pub qualities: Option<Vec<ToolQuality>>,

    /// Rechargeable / energy capacity if applicable.
    #[serde(default)]
    pub capacity: Option<crate::units::Energy>,

    /// Catch-all for unknown fields
    #[serde(default)]
    pub extra: Option<serde_json::Value>,

    /// Snippet category (for random text)
    #[serde(default)]
    pub snippet_category: Option<serde_json::Value>,

    /// Tool quality (for tools)
    #[serde(default)]
    pub tool: Option<serde_json::Value>,

    /// Variants (different appearances)
    #[serde(default)]
    pub variants: Option<serde_json::Value>,

    /// Techniques for melee
    #[serde(default)]
    pub techniques: Option<serde_json::Value>,

    /// Category override
    #[serde(default)]
    pub subtypes: Option<serde_json::Value>,

    /// Default ammo
    #[serde(default)]
    pub default_ammo: Option<String>,

    /// Max charges
    #[serde(default)]
    pub max_charges: Option<u32>,

    /// Initial charges
    #[serde(default)]
    pub initial_charges: Option<u32>,

    /// Charges
    #[serde(default)]
    pub charges: Option<u32>,

    /// Stack size
    #[serde(default)]
    pub stack_size: Option<u32>,

    /// Container (the container item this comes in)
    #[serde(default)]
    pub container: Option<String>,

    /// Quench value (for drinks)
    #[serde(default)]
    pub quench: Option<i32>,

    /// Ammo type
    #[serde(default)]
    pub ammo_type: Option<String>,

    /// Spoils in (time string)
    #[serde(default)]
    pub spoils_in: Option<serde_json::Value>,

    /// Warmth value (for clothing)
    #[serde(default)]
    pub warmth: Option<u32>,

    /// Comestible type (food/drink/med)
    #[serde(default)]
    pub comestible_type: Option<serde_json::Value>,

    /// Vitamins
    #[serde(default)]
    pub vitamins: Option<serde_json::Value>,

    /// Calories (for food)
    #[serde(default)]
    pub calories: Option<u32>,

    /// Fun rating (for reading)
    #[serde(default)]
    pub fun: Option<i32>,

    /// Material thickness (for armor)
    #[serde(default)]
    pub material_thickness: Option<u32>,

    /// To-hit modifier
    #[serde(default)]
    pub to_hit: Option<serde_json::Value>,

    /// Armor values
    #[serde(default)]
    pub armor: Option<serde_json::Value>,

    /// Use action (interaction behavior)
    #[serde(default)]
    pub use_action: Option<serde_json::Value>,

    /// Item looks like another item for display
    #[serde(default)]
    pub looks_like: Option<String>,

    /// Proportional modifications
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proportional: Option<serde_json::Value>,

    /// Relative modifications
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relative: Option<serde_json::Value>,

    /// Delete operations
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delete: Option<serde_json::Value>,

    /// Extend operations
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extend: Option<serde_json::Value>,

    /// Abstract flag (not a real game object)
    #[serde(default)]
    pub abstract_: Option<bool>,

    /// copy-from parent ID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_from: Option<String>,
}

fn default_volume() -> Volume {
    Volume::from_milliliters(250)
}

fn default_symbol() -> String {
    "#".to_string()
}

/// A pocket inside a container item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PocketDef {
    /// Type of pocket.
    #[serde(rename = "pocket_type", default)]
    pub pocket_type: PocketType,

    /// Maximum volume this pocket can hold.
    #[serde(default)]
    pub max_volume: Option<Volume>,

    /// Maximum weight this pocket can hold.
    #[serde(default)]
    pub max_weight: Option<Weight>,

    /// Minimum item volume to fit in this pocket.
    #[serde(default)]
    pub min_item_volume: Option<Volume>,

    /// Maximum item length to fit in this pocket.
    #[serde(default)]
    pub max_item_length: Option<Length>,

    /// Whether the pocket is sealed (can't be opened).
    #[serde(default)]
    pub sealed: Option<bool>,

    /// Number of items that can be stored.
    #[serde(default)]
    pub max_contains: Option<u32>,

    /// Whether the pocket can be used to store ammo.
    #[serde(default)]
    pub ammo_restriction: Option<Vec<DefId<ItemDef>>>,

    /// Relative morph levels for non-container pockets.
    #[serde(default)]
    pub relative: Option<serde_json::Value>,

    /// Extra flags.
    #[serde(default)]
    pub flag: Option<String>,
}

/// Pocket type enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PocketType {
    /// General-purpose container.
    #[serde(rename = "CONTAINER")]
    Container,
    /// Magazine for ammunition.
    #[serde(rename = "MAGAZINE")]
    Magazine,
    /// Magazine well (for weapons that accept magazines).
    #[serde(rename = "MAGAZINE_WELL")]
    MagazineWell,
    /// Holster sized for specific weapon types.
    #[serde(rename = "HOLSTER")]
    Holster,
    /// Special-purpose pocket (e.g. canteen).
    #[serde(rename = "SPECIAL")]
    Special,
    /// Corrosion-resistant outer layer for things like hip pouches.
    #[serde(rename = "CORROSION")]
    Corrosion,
    /// Migrates items.
    #[serde(rename = "MIGRATING")]
    Migrating,
    /// Pocket that only stores items with certain qualifiers.
    #[serde(rename = "MOD")]
    Mod,
    /// Software storage.
    #[serde(rename = "SOFTWARE")]
    Software,
}

impl Default for PocketType {
    fn default() -> Self {
        PocketType::Container
    }
}

/// A tool quality entry (e.g. "CUTTING" quality level 2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolQuality {
    /// Quality type identifier (e.g. "CUT", "BOIL", "HAMMER").
    pub id: String,
    /// Level of the quality.
    pub level: u32,
}
