use crate::data::raw_defs::cdda_types::{
    BodyPartArmor, CddaColor, CddaDuration, CddaPrice, ComestibleType, ItemVariant, MaterialList,
    MeleeDamage, RawValue, SnippetCategory, StringOrArray, ToHit, UseAction, VitaminContents,
};
use crate::data::raw_types::{DefId, LocalizedString};
use crate::core::units::{Length, Volume, Weight};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// How charges work for this item.
///
/// CDDA has long-standing bugs from confusing charges-count meaning;
/// `CountMode` makes this explicit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ItemCategory {
    pub id: String,
    pub name: LocalizedString,
}

/// A single item definition from JSON type `"ITEM"`.
///
/// CDDA has many subtypes of ITEM (GUN, AMMO, COMESTIBLE, TOOL, etc.)
/// that all share this same struct with different fields populated.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ItemDef {
    /// Unique string identifier.
    pub id: DefId<ItemDef>,

    /// Display name.
    #[serde(default)]
    pub name: Option<LocalizedString>,

    /// Description text.
    #[serde(default)]
    pub description: Option<LocalizedString>,

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
    pub material: MaterialList,

    /// ASCII symbol for map display.
    #[serde(default = "default_symbol")]
    pub symbol: String,

    /// Color for map display (can be string, map, or array in CDDA data).
    #[serde(default)]
    pub color: Option<CddaColor>,

    /// Item price before the cataclysm.
    #[serde(default)]
    pub price: Option<CddaPrice>,

    /// Item price after the cataclysm.
    #[serde(default)]
    pub price_postapoc: Option<CddaPrice>,

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
    pub melee_damage: Option<MeleeDamage>,

    /// If set, this item is a container with the given pocket data.
    #[serde(default)]
    pub pocket_data: Option<Vec<PocketDef>>,

    /// Tool qualities (e.g. `[["CUT", 2], ["BUTCHER", -18]]`).
    #[serde(default, deserialize_with = "deserialize_qualities")]
    pub qualities: Option<Vec<ToolQuality>>,

    /// Rechargeable / energy capacity if applicable.
    #[serde(default)]
    pub capacity: Option<crate::core::units::Energy>,

    /// Catch-all for unknown fields
    #[serde(default)]
    pub extra: Option<HashMap<String, RawValue>>,

    /// Snippet category (for random text)
    #[serde(default)]
    pub snippet_category: Option<SnippetCategory>,

    /// Tool requirement (tool ID string or map of tool IDs to counts).
    #[serde(default)]
    pub tool: Option<ToolRequirement>,

    /// Variants (different appearances)
    #[serde(default)]
    pub variants: Option<Vec<ItemVariant>>,

    /// Techniques for melee
    #[serde(default)]
    pub techniques: Option<Vec<String>>,

    /// Category override
    #[serde(default)]
    pub subtypes: Option<Vec<String>>,

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

    /// Ammo type (string or array of strings for magazines).
    #[serde(default)]
    pub ammo_type: Option<StringOrArray>,

    /// Tool ammo (ammunition type(s) this tool accepts).
    #[serde(default)]
    pub tool_ammo: Option<StringOrArray>,

    /// Spoils in (time string)
    #[serde(default)]
    pub spoils_in: Option<CddaDuration>,

    /// Warmth value (for clothing)
    #[serde(default)]
    pub warmth: Option<i32>,

    /// Comestible type (food/drink/med)
    #[serde(default)]
    pub comestible_type: Option<ComestibleType>,

    /// Vitamins
    #[serde(default)]
    pub vitamins: Option<VitaminContents>,

    /// Calories (for food)
    #[serde(default)]
    pub calories: Option<u32>,

    /// Fun rating (for reading)
    #[serde(default)]
    pub fun: Option<i32>,

    /// Material thickness (for armor)
    #[serde(default)]
    pub material_thickness: Option<f64>,

    /// To-hit modifier
    #[serde(default)]
    pub to_hit: Option<ToHit>,

    /// Armor values — array of body part armor data (CDDA format).
    #[serde(default)]
    pub armor: Option<Vec<BodyPartArmor>>,

    /// Use action (interaction behavior)
    #[serde(default)]
    pub use_action: Option<UseAction>,

    /// Charges per use (for tools).
    #[serde(default)]
    pub charges_per_use: Option<u32>,

    /// Power draw (for electric tools).
    #[serde(default)]
    pub power_draw: Option<String>,

    /// Revert to item ID (for activated tools).
    #[serde(default)]
    pub revert_to: Option<String>,

    /// Item looks like another item for display
    #[serde(default)]
    pub looks_like: Option<String>,

    /// Abstract flag (not a real game object)
    #[serde(default)]
    pub abstract_: Option<bool>,

    /// copy-from parent ID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_from: Option<String>,

    // ── Ammo-specific fields ───────────────────────────────────────
    /// Ammo damage — number, structured object `{"damage_type": "bullet",
    /// "amount": 25}`, or typed array.  Uses `RawValue` because CDDA ammo
    /// damage objects mix string and numeric values (unlike `MeleeDamage`).
    #[serde(default)]
    pub damage: Option<RawValue>,

    /// Armor penetration for ammunition.
    #[serde(default)]
    pub pierce: Option<i32>,

    /// Effective range of the ammunition (in tiles).
    #[serde(default)]
    pub range: Option<i32>,

    /// Inherent dispersion (accuracy penalty) of the ammunition.
    #[serde(default)]
    pub dispersion: Option<i32>,

    /// Recoil contributed by firing this ammunition.
    #[serde(default)]
    pub recoil: Option<i32>,
}

fn default_volume() -> Volume {
    Volume::from_milliliters(250)
}

fn default_symbol() -> String {
    "#".to_string()
}

/// A pocket inside a container item.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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

    /// Whether the pocket can be used to store ammo (maps ammo type to capacity).
    #[serde(default)]
    pub ammo_restriction: Option<HashMap<String, u32>>,

    /// Whether this pocket acts as a holster.
    #[serde(default)]
    pub holster: Option<bool>,

    /// Whether this pocket is rigid (volume doesn't change when empty).
    #[serde(default)]
    pub rigid: Option<bool>,

    /// Whether this pocket is ablative.
    #[serde(default)]
    pub ablative: Option<bool>,

    /// Volume encumber modifier (float, e.g. 0.3).
    #[serde(default)]
    pub volume_encumber_modifier: Option<f64>,

    /// Max contains volume (string like "2 L").
    #[serde(default)]
    pub max_contains_volume: Option<String>,

    /// Max contains weight (string like "2 kg").
    #[serde(default)]
    pub max_contains_weight: Option<String>,

    /// Move cost to interact.
    #[serde(default)]
    pub moves: Option<u32>,

    /// Description of the pocket.
    #[serde(default)]
    pub description: Option<String>,

    /// Extra flags.
    #[serde(default)]
    pub flag: Option<String>,
}

/// Pocket type enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
    /// File storage (e.g. USB drives).
    #[serde(rename = "E_FILE_STORAGE")]
    EFileStorage,
}

impl Default for PocketType {
    fn default() -> Self {
        PocketType::Container
    }
}

/// Tool requirement — a single tool ID string, or a map of tool IDs to counts.
///
/// CDDA uses `"tool": "syringe"` for single-tool requirements and
/// `"tool": { "syringe": 1 }` for tool-with-count requirements.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ToolRequirement {
    /// Single tool ID: `"syringe"`.
    Named(String),
    /// Map of tool IDs to counts: `{ "syringe": 1 }`.
    Map(HashMap<String, RawValue>),
}

/// A tool quality entry (e.g. "CUT" quality level 2).
///
/// CDDA format is `[["CUT", 2], ["BUTCHER", -18], ...]` — an array of 2-element arrays.
/// This can also be objects `{"id": "CUT", "level": 2}`.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ToolQuality {
    /// Quality type identifier (e.g. "CUT", "BOIL", "HAMMER").
    pub id: String,
    /// Level of the quality (can be negative for "anti-qualities").
    pub level: i32,
}

/// Custom deserializer for `Vec<ToolQuality>` that handles both
/// `[["CUT", 2], ...]` (CDDA tuple format) and
/// `[{"id": "CUT", "level": 2}, ...]` (object format).
fn deserialize_qualities<'de, D>(deserializer: D) -> Result<Option<Vec<ToolQuality>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum QualEntry {
        /// Object format: `{"id": "CUT", "level": 2}`
        Obj { id: String, level: i32 },
        /// Tuple format: `["CUT", 2]`
        Tuple(String, i32),
    }

    let opt: Option<Vec<QualEntry>> = Option::deserialize(deserializer)?;
    Ok(opt.map(|entries| {
        entries
            .into_iter()
            .map(|e| match e {
                QualEntry::Obj { id, level } => ToolQuality { id, level },
                QualEntry::Tuple(id, level) => ToolQuality { id, level },
            })
            .collect()
    }))
}
