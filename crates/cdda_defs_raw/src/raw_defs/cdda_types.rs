//! Shared CDDA field types — proper Rust enums replacing raw `serde_json::Value`.
//!
//! Many CDDA JSON fields accept multiple shapes (string, number, object, array).
//! This module provides typed enums for the most common variant patterns,
//! making deserialization safe and explicit instead of relying on `serde_json::Value`.
//!
//! Each type here corresponds to a CDDA JSON field pattern that appears across
//! multiple definition types (items, monsters, terrain, etc.).
//!
//! Usage: `use crate::raw_defs::cdda_types::*;`

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Colors
// ---------------------------------------------------------------------------

/// A color value in CDDA can be a simple name, a list of names (random),
/// or a structured map with foreground/background.
///
/// Examples:
/// - `"red"`
/// - `["white", "light_gray"]`
/// - `{"fg": "red", "bg": "blue"}`
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum CddaColor {
    /// Single color name: `"red"`, `"light_blue"`, etc.
    Named(String),
    /// Multiple color names (random selection): `["white", "light_gray"]`
    Multi(Vec<String>),
    /// Structured color with foreground (and optional background / season variants).
    Structured(CddaColorStructured),
}

/// A structured color with optional foreground and background.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CddaColorStructured {
    #[serde(default)]
    pub fg: Option<String>,
    #[serde(default)]
    pub bg: Option<String>,
    // CDDA also supports season variants: { "spring": ..., "summer": ... }
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spring: Option<Box<CddaColor>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summer: Option<Box<CddaColor>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub autumn: Option<Box<CddaColor>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub winter: Option<Box<CddaColor>>,
}

// ---------------------------------------------------------------------------
// Text / Localization
// ---------------------------------------------------------------------------

/// A localized text value — either a plain string or a structured object
/// with `str`, `str_pl`, `ctxt`, and/or `str_sp` fields.
///
/// Examples:
/// - `"A rusty pipe."`
/// - `{"str": "zombie", "str_pl": "zombies"}`
/// - `{"ctxt": "verb", "str": "dented"}`
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum LocalizedText {
    /// Plain string (no translation context).
    Plain(String),
    /// Structured text with optional singular, plural, context, and spatial forms.
    Structured(StructuredText),
}

/// A structured text value from CDDA JSON.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct StructuredText {
    /// Singular (or default) form.
    #[serde(default)]
    pub str: Option<String>,
    /// Plural form.
    #[serde(default)]
    pub str_pl: Option<String>,
    /// Context for translation disambiguation (e.g. "verb" vs "noun").
    #[serde(default)]
    pub ctxt: Option<String>,
    /// Spatial form (same for singular and plural).
    #[serde(default)]
    pub str_sp: Option<String>,
}

// ---------------------------------------------------------------------------
// Damage
// ---------------------------------------------------------------------------

/// CDDA melee damage — the `"melee_damage"` field.
///
/// CDDA has evolved through multiple damage formats:
/// 1. Simple bash number: `4`
/// 2. Object with damage-type keys: `{"bash": 4, "cut": 2}`
/// 3. Array of typed entries: `[{"damage_type": "cut", "amount": 2}]`
///
/// The array form is the current CDDA standard, but all three are still in use.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum MeleeDamage {
    /// Simple bash damage number (legacy format).
    BashOnly(i32),
    /// Object with damage-type keys: `{"bash": 4, "cut": 2}`.
    ByType(HashMap<String, i32>),
    /// Array of typed damage: `[{"damage_type": "cut", "amount": 2}]`.
    TypedArray(Vec<TypedDamage>),
}

/// A damage entry with explicit type and amount (current CDDA format).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TypedDamage {
    pub damage_type: String,
    pub amount: i32,
}

/// Thrown damage is always the typed-array format.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ThrownDamage {
    pub damage_type: String,
    pub amount: i32,
}

// ---------------------------------------------------------------------------
// Price
// ---------------------------------------------------------------------------

/// Item price.
///
/// CDDA prices can be plain integers (in cents) or strings with
/// currency units like `"13 cent"`, `"16 USD"`, `"34 USD"`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum CddaPrice {
    /// Numeric price in cents.
    Numeric(i64),
    /// String with currency unit (e.g. "13 cent", "16 USD").
    Text(String),
}

// ---------------------------------------------------------------------------
// Comestible
// ---------------------------------------------------------------------------

/// Comestible type — the `"comestible_type"` field on COMESTIBLE items.
///
/// CDDA uses "INVALID" for items that are not actually comestible but have
/// COMESTIBLE type for other reasons (e.g., chemicals, non-food items).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum ComestibleType {
    #[serde(rename = "FOOD")]
    Food,
    #[serde(rename = "DRINK")]
    Drink,
    #[serde(rename = "MED")]
    Med,
    #[serde(rename = "TOOL")]
    Tool,
    /// "INVALID" — item is not actually a comestible (used for chemicals, etc.)
    #[serde(rename = "INVALID")]
    Invalid,
}

// ---------------------------------------------------------------------------
// Flexible value (replaces serde_json::Value for catch-all variants)
// ---------------------------------------------------------------------------

/// A flexible JSON-like value for catch-all fields where CDDA data can be
/// a string, number, boolean, array, or object. This replaces `serde_json::Value`
/// to avoid pulling in the serde_json dependency for type definitions.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum RawValue {
    /// A string value.
    String(String),
    /// A numeric value (integer or float).
    Number(f64),
    /// A boolean value.
    Bool(bool),
    /// An array of values.
    Array(Vec<RawValue>),
    /// An object/map of string keys to values.
    Object(HashMap<String, RawValue>),
}

// ---------------------------------------------------------------------------
// Use Action
// ---------------------------------------------------------------------------

/// Item use action — the `"use_action"` field.
///
/// Can be a simple string ID referencing a built-in action,
/// or an object with `"type"` and optional extra parameters.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum UseAction {
    /// Simple action name: `"CROWBAR"`, `"PICKAXE"`, etc.
    Named(String),
    /// Tool action with tool ID and charges: `["CROWBAR", 0]`.
    ToolAction(String, u32),
    /// Structured action definition (single object).
    Structured(UseActionDef),
    /// Array of actions (CDDA format where use_action is an array).
    Array(Vec<UseAction>),
    /// Catch-all for any action type not specifically modeled.
    Other(HashMap<String, RawValue>),
}

/// An effect reference in a use action — either a simple effect ID or an object.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum UseActionEffect {
    /// Simple effect ID string.
    Id(String),
    /// Object with effect type and parameters.
    Obj(HashMap<String, RawValue>),
}

/// A tool reference in a use action.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum UseActionTool {
    /// Simple tool ID.
    Id(String),
    /// Tool with quantity: `["tool_id", count]`
    WithCount(String, u32),
    /// Object with type and amount.
    Obj(HashMap<String, RawValue>),
}

/// An item reference in a use action.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum UseActionItem {
    /// Simple item ID.
    Id(String),
    /// Item with quantity: `["item_id", count]`
    WithCount(String, u32),
    /// Object with type and amount.
    Obj(HashMap<String, RawValue>),
}

/// A use action definition object with the most common fields.
/// CDDA has many action types; this covers the majority.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UseActionDef {
    /// Action type identifier.
    pub r#type: String,
    /// Target item ID (for transforms, etc.).
    #[serde(default)]
    pub target: Option<String>,
    /// Message when activating.
    #[serde(default)]
    pub msg: Option<String>,
    /// Activation message.
    #[serde(default)]
    pub activation_message: Option<String>,
    /// Holster prompt text.
    #[serde(default)]
    pub holster_prompt: Option<String>,
    /// Holster completion message.
    #[serde(default)]
    pub holster_msg: Option<String>,
    /// Whether the item needs to be wielded.
    #[serde(default)]
    pub need_wielding: Option<bool>,
    /// Charges needed to activate.
    #[serde(default)]
    pub need_charges: Option<u32>,
    /// Message when not enough charges.
    #[serde(default)]
    pub need_charges_msg: Option<String>,
    /// Charges consumed per use.
    #[serde(default)]
    pub charges_to_use: Option<u32>,
    /// Move cost.
    #[serde(default)]
    pub moves: Option<u32>,
    /// Speed penalty for the action.
    #[serde(default)]
    pub move_speed: Option<u32>,
    /// Slow move speed (alternative to moves for firestarters etc).
    #[serde(default)]
    pub moves_slow: Option<u32>,
    /// Effects triggered by use.
    #[serde(default)]
    pub effects: Option<Vec<UseActionEffect>>,
    /// Tools consumed by use.
    #[serde(default)]
    pub tools_needed: Option<Vec<UseActionTool>>,
    /// Items consumed by use.
    #[serde(default)]
    pub items_needed: Option<Vec<UseActionItem>>,
    /// Cooldown after use.
    #[serde(default)]
    pub cooldown: Option<u32>,
    /// Final item after transformation.
    #[serde(default)]
    pub resulting_item: Option<String>,
    /// Sealed status for containers.
    #[serde(default)]
    pub seal: Option<bool>,
    /// Whether it unseals containers.
    #[serde(default)]
    pub unseal: Option<bool>,
}

// ---------------------------------------------------------------------------
// Armor
// ---------------------------------------------------------------------------

/// Armor values by damage type — used on materials/monsters as flat damage reduction.
///
/// Example: `{"bash": 6, "cut": 8, "bullet": 6, "electric": 1}`
///
/// NOTE: Items in CDDA use `"armor"` as an array of body-part-specific armor pieces
/// (see `BodyPartArmor`), NOT this type. This type is for materials and monsters.
#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
pub struct ArmorValues {
    #[serde(default)]
    pub bash: Option<i32>,
    #[serde(default)]
    pub cut: Option<i32>,
    #[serde(default)]
    pub stab: Option<i32>,
    #[serde(default)]
    pub bullet: Option<i32>,
    #[serde(default)]
    pub heat: Option<i32>,
    #[serde(default)]
    pub cold: Option<i32>,
    #[serde(default)]
    pub electric: Option<i32>,
    #[serde(default)]
    pub acid: Option<i32>,
    #[serde(default)]
    pub biological: Option<i32>,
}

/// A body part armor data — the individual elements in the `"armor"` array on items.
///
/// CDDA items store armor as an array of these objects, each describing how
/// a piece of armor covers a specific body part.
///
/// Example:
/// ```json
/// {
///   "encumbrance_modifiers": ["NONE"],
///   "coverage": 100,
///   "covers": ["head"],
///   "specifically_covers": ["head_crown", "head_forehead"]
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BodyPartArmor {
    /// Clothing layers this armor piece occupies (e.g. "NORMAL", "OUTER").
    #[serde(default)]
    pub layers: Option<Vec<String>>,
    /// Coverages body parts (single string or array of strings).
    #[serde(default)]
    pub covers: Option<StringOrArray>,
    /// Specific sub-parts covered.
    #[serde(default)]
    pub specifically_covers: Option<Vec<String>>,
    /// Coverage percentage (0-100).
    #[serde(default)]
    pub coverage: Option<u32>,
    /// Encumbrance value (single value or range `[min, max]`).
    #[serde(default)]
    pub encumbrance: Option<EncumbranceOrRange>,
    /// Encumbrance modifiers.
    #[serde(default)]
    pub encumbrance_modifiers: Option<Vec<String>>,
    /// Materials layered in this armor piece.
    #[serde(default)]
    pub material: Option<Vec<ArmorMaterialLayer>>,
    /// Volume encumber modifier (float, e.g. 0.3).
    #[serde(default)]
    pub volume_encumber_modifier: Option<f64>,
    /// Max volume the pocket/armor can hold.
    #[serde(default)]
    pub max_contains_volume: Option<String>,
    /// Max weight.
    #[serde(default)]
    pub max_contains_weight: Option<String>,
    /// Move cost to interact.
    #[serde(default)]
    pub moves: Option<u32>,
    /// Description.
    #[serde(default)]
    pub description: Option<String>,
    /// Whether this is ablative armor.
    #[serde(default)]
    pub ablative: Option<bool>,
}

/// Encumbrance value — single u32 or a range `[min, max]`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum EncumbranceOrRange {
    /// Single encumbrance value.
    Single(u32),
    /// Encumbrance range `[min, max]`.
    Range(Vec<u32>),
}

/// A material layer in an armor piece.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ArmorMaterialLayer {
    /// Material ID.
    pub r#type: String,
    /// Percentage of the armor piece covered by this material.
    #[serde(default)]
    pub covered_by_mat: Option<u32>,
    /// Material thickness.
    #[serde(default)]
    pub thickness: Option<f64>,
}

// ---------------------------------------------------------------------------
// To-Hit
// ---------------------------------------------------------------------------

/// To-hit modifier — the `"to_hit"` field on items.
///
/// CDDA can have to_hit as a simple number (legacy) or as an object with
/// grip/length/surface/balance keys.
///
/// Examples:
/// - `100` — simple numeric bonus
/// - `{"grip": "weapon", "length": "long", "surface": "any", "balance": "good"}` — structured
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ToHit {
    /// Simple numeric to-hit value (legacy format).
    Number(i32),
    /// Structured to-hit with grip/length/surface/balance.
    Struct {
        #[serde(default)]
        grip: Option<String>,
        #[serde(default)]
        length: Option<String>,
        #[serde(default)]
        surface: Option<String>,
        #[serde(default)]
        balance: Option<String>,
    },
}

// ---------------------------------------------------------------------------
// Duration / Time
// ---------------------------------------------------------------------------

/// A time duration value from CDDA.
///
/// CDDA uses human-readable time strings in many places.
/// Examples: `"1 day"`, `"14 days"`, `"30 minutes"`, `"6 h"`.
///
/// Also accepts raw numeric second values.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum CddaDuration {
    /// Raw number of seconds/minutes/turns.
    Number(u32),
    /// Human-readable time string.
    Text(String),
}

// ---------------------------------------------------------------------------
// Monster fields
// ---------------------------------------------------------------------------

/// Monster death drops — the `"death_drops"` field.
///
/// Can be a simple item group ID string or an inline item group definition.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum DeathDrops {
    /// Reference to a named item group.
    GroupId(String),
    /// Inline item group definition (object with subtype, entries, etc.).
    Inline(HashMap<String, RawValue>),
    /// Array of inline item group definitions (each with group, count, etc.).
    Array(Vec<HashMap<String, RawValue>>),
}

/// Monster death function — the `"death_function"` field.
///
/// Can be a single function name, a list, or an object.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum DeathFunction {
    /// Single death function name: `"NORMAL"`, `"SMOKEBURST"`, etc.
    Named(String),
    /// Multiple death functions: `["SPLATTER", "SMOKEBURST"]`.
    Multi(Vec<String>),
    /// Object form: `{"corpse_type": "NO_CORPSE", "message": "..."}`
    Object(HashMap<String, RawValue>),
    /// Fallback for any unrecognized death function format.
    Other(String),
}

/// Monster upgrade info — the `"upgrades"` field.
///
/// Can be `false` (no upgrades), or an object with upgrade parameters.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum UpgradeInfo {
    /// Explicitly disabled (`"upgrades": false`).
    Disabled(bool),
    /// Active upgrade definition.
    Active(UpgradeDef),
}

/// Monster upgrade parameters.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UpgradeDef {
    /// Half-life in days (evolution speed).
    #[serde(default)]
    pub half_life: Option<u32>,
    /// Age to grow (in days).
    #[serde(default)]
    pub age_grow: Option<u32>,
    /// Evolves into a monster group.
    #[serde(default)]
    pub into_group: Option<String>,
    /// Evolves into a specific monster type.
    #[serde(default)]
    pub into: Option<String>,
    /// Whether to use multi-level upgrades.
    #[serde(default)]
    pub multi_level: Option<bool>,
}

/// The type of baby produced by a monster.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BabyType {
    /// Egg item ID.
    #[serde(default)]
    pub baby_egg: Option<String>,
    /// Monster ID(s) produced. Can be a single string or array of strings.
    #[serde(default)]
    pub baby_monster: Option<StringOrArray>,
    /// Monster group ID(s) produced. Can be a single string or array of strings.
    #[serde(default)]
    pub baby_monster_group: Option<StringOrArray>,
}

/// Monster reproduction data — the `"reproduction"` field.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Reproduction {
    /// Type of baby produced (wraps baby_egg or baby_monster).
    #[serde(default)]
    pub baby_type: Option<BabyType>,
    /// Chance per reproduction attempt.
    #[serde(default)]
    pub baby_chance: Option<u32>,
    /// Time between reproduction attempts.
    #[serde(default)]
    pub baby_timer: Option<u32>,
    /// Count of babies produced.
    #[serde(default)]
    pub baby_count: Option<u32>,
    /// Intensity of reproduction trigger.
    #[serde(default)]
    pub intensity: Option<u32>,
    /// Maximum population limit.
    #[serde(default)]
    pub max_population: Option<u32>,
}

// ---------------------------------------------------------------------------
// Faction fields
// ---------------------------------------------------------------------------

/// Faction size — the `"size"` field on factions.
///
/// CDDA stores this as either a string like `"large"`, `"medium"`,
/// or a numeric population count.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum FactionSize {
    /// Named size category.
    Named(String),
    /// Numeric population count.
    Numeric(u32),
}

/// Faction relations — the `"relations"` field.
///
/// A map from faction IDs to relationship values.
/// Each value can be a simple numeric reputation or a structured object.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FactionRelations(pub HashMap<String, FactionRelationValue>);

/// A relationship value to another faction.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum FactionRelationValue {
    /// Simple numeric reputation.
    Numeric(i32),
    /// Structured relationship with kill/talk/trade/favor values.
    Structured(FactionRelationStructured),
}

/// Structured multi-axis relationship to another faction.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FactionRelationStructured {
    #[serde(default)]
    pub kill: Option<i32>,
    #[serde(default)]
    pub talk: Option<i32>,
    #[serde(default)]
    pub trade: Option<i32>,
    #[serde(default)]
    pub favor: Option<i32>,
}

/// Faction price rules — the `"price_rules"` field.
///
/// Maps item IDs to markup percentage (e.g. `{"ammo": 1.5}` means 50% markup).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FactionPriceRules(pub HashMap<String, f64>);

/// Faction reputation values — the `"reputations"` field.
///
/// Maps faction IDs to starting reputation numbers.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FactionReputations(pub HashMap<String, i32>);

/// Faction food supply — the `"fac_food_supply"` / `"food_supply"` field.
///
/// Can be a simple number or an array of food group references.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum FoodSupply {
    /// Numeric food supply rating.
    Numeric(i32),
    /// Array of food item group references.
    Items(Vec<String>),
}

// ---------------------------------------------------------------------------
// Overmap / Terrain
// ---------------------------------------------------------------------------

/// Overmap terrain see cost — the `"see_cost"` field.
///
/// Can be a string like `"high"`, `"low"`, `"none"` or a numeric value.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum SeeCost {
    /// Named visibility category.
    Named(String),
    /// Numeric visibility cost.
    Numeric(u32),
}

// ---------------------------------------------------------------------------
// Material properties
// ---------------------------------------------------------------------------

/// Material damage resistance values — the `"resist"` field on materials.
///
/// Example: `{"bash": 4, "cut": 5, "acid": 10, "heat": 6, "bullet": 3}`
#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
pub struct DamageResistance {
    #[serde(default)]
    pub bash: Option<f64>,
    #[serde(default)]
    pub cut: Option<f64>,
    #[serde(default)]
    pub stab: Option<f64>,
    #[serde(default)]
    pub bullet: Option<f64>,
    #[serde(default)]
    pub heat: Option<f64>,
    #[serde(default)]
    pub cold: Option<f64>,
    #[serde(default)]
    pub electric: Option<f64>,
    #[serde(default)]
    pub acid: Option<f64>,
}

/// Chip resistance — the `"chip_resist"` field on materials.
///
/// Can be a simple number or an object with per-damage-type values.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ChipResist {
    /// Uniform resistance value.
    Uniform(u32),
    /// Per-damage-type resistance.
    ByType(HashMap<String, u32>),
}

// ---------------------------------------------------------------------------
// Copy-from edit modifiers (extend, delete, proportional, relative)
// ---------------------------------------------------------------------------

/// A generic modifier map for `extend`, `delete`, `proportional`, and `relative`
/// operations. Each maps field names to their modification values.
///
/// These are used across items, monsters, terrain, furniture, recipes, etc.
/// A count value that can be a single number or a [min, max] pair.
///
/// Used by bash drops, deconstruction items, and similar fields.
///
/// Examples:
/// - `1` → `Single(1)`
/// - `[2, 5]` → `Range(2, 5)`
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum CountRange {
    /// Single count value: `1`
    Single(u32),
    /// Range [min, max]: `[2, 5]`
    Range(u32, u32),
}

impl Default for CountRange {
    fn default() -> Self {
        CountRange::Single(1)
    }
}

/// A value that can be a single string or an array of strings.
///
/// Used by `connect_groups`, `connects_to`, `rotates_to`, and similar fields.
///
/// Examples:
/// - `"wall"` → `Single("wall")`
/// - `["wall", "fence"]` → `Multi(vec!["wall", "fence"])`
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum StringOrArray {
    /// Single string value.
    Single(String),
    /// Array of strings.
    Multi(Vec<String>),
}

impl Default for StringOrArray {
    fn default() -> Self {
        StringOrArray::Single(String::new())
    }
}

impl StringOrArray {
    /// Get all strings, whether single or multi.
    pub fn all_strings(&self) -> Vec<&str> {
        match self {
            StringOrArray::Single(s) => vec![s.as_str()],
            StringOrArray::Multi(v) => v.iter().map(|s| s.as_str()).collect(),
        }
    }

    /// Returns the first string, or an empty string if empty.
    pub fn first_or_default(&self) -> &str {
        match self {
            StringOrArray::Single(s) => s.as_str(),
            StringOrArray::Multi(v) => v.first().map(|s| s.as_str()).unwrap_or(""),
        }
    }
}

// ---------------------------------------------------------------------------
// Item-specific shared types
// ---------------------------------------------------------------------------

/// Snippet category reference — the `"snippet_category"` field.
///
/// Can be a simple category ID string or a structured snippet list.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum SnippetCategory {
    /// Named snippet category ID.
    Named(String),
    /// Inline array of snippet objects (CDDA format with `id` and `text` fields).
    Inline(Vec<HashMap<String, String>>),
}

/// Vitamin cost mapping — the `"vitamins"` field on comestibles.
///
/// Example: `[["calcium", 2], ["iron", 6], ["meat_allergen", 1]]`
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VitaminContents(pub Vec<VitaminEntry>);

/// A single vitamin entry: `["vitamin_id", amount]`.
///
/// Amount can be a number or a string like "14 mg" or "500 μg".
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum VitaminEntry {
    /// Numeric amount: `["vitC", 14]`
    Numeric(String, u32),
    /// String amount: `["vitC", "14 mg"]`
    Text(String, String),
}

/// Item variant entry — the `"variants"` field on items.
///
/// Each variant is an alternate appearance/description for the same item.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ItemVariant {
    /// Unique variant ID.
    pub id: String,
    /// Alternative name.
    #[serde(default)]
    pub name: Option<LocalizedText>,
    /// Alternative description (plain string or object with `str` key).
    #[serde(default)]
    pub description: Option<LocalizedText>,
    /// Alternative weight (gram delta or absolute).
    #[serde(default)]
    pub weight: Option<i64>,
    /// Whether variant properties append to base item.
    #[serde(default)]
    pub append: Option<bool>,
    /// Alternative color.
    #[serde(default)]
    pub color: Option<CddaColor>,
}

/// Conditional name entry — the `"conditional_names"` field on items.
///
/// Example: `{"type": "VITAMIN", "condition": "human_flesh_vitamin", "name": "raw Mannwurst"}`
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ConditionalName {
    /// Condition type: "VITAMIN", "FLAG", "COMPONENT_ID_SUBSTRING", etc.
    #[serde(rename = "type")]
    pub condition_type: String,
    /// The condition value to match.
    pub condition: String,
    /// The name to use when the condition is met.
    pub name: LocalizedText,
}

/// Material list — can be an array of material references or a single map.
///
/// CDDA supports multiple material formats:
/// - `["wood", "steel"]` — array of strings
/// - `[{"type": "paper", "portion": 2}, ...]` — array of objects
/// - `{"paper": 2, "plastic": 1}` — single map of material ID to count
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum MaterialList {
    /// Array of material references: `["wood", "steel"]` or `[{"type": "paper", "portion": 2}]`.
    Array(Vec<MaterialRef>),
    /// Single map of material ID to count: `{"paper": 2, "plastic": 1}`.
    Map(HashMap<String, f64>),
}

impl Default for MaterialList {
    fn default() -> Self {
        MaterialList::Array(Vec::new())
    }
}

/// Material reference — can be a simple string ("flesh", "steel"),
/// an object with type and portion (for composite materials),
/// or a map of material ID to count (e.g. `{"paper": 2, "plastic": 1}`).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum MaterialRef {
    /// Single material ID.
    Single(String),
    /// Composite material with type and portion.
    Composite(MaterialPortion),
    /// Map of material ID to count (e.g. `{"paper": 2, "plastic": 1}`).
    Map(HashMap<String, f64>),
}

/// A material with a weight portion (for composite items).
///
/// CDDA material objects can have:
/// - `"type"`: material ID (required)
/// - `"portion"`: weight percentage (for composite items)
/// - `"covered_by_mat"`: coverage percentage (for armor layers)
/// - `"thickness"`: material thickness (for armor layers)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MaterialPortion {
    pub r#type: String,
    #[serde(default)]
    pub portion: Option<f64>,
    #[serde(default)]
    pub covered_by_mat: Option<u32>,
    #[serde(default)]
    pub thickness: Option<f64>,
}

// ---------------------------------------------------------------------------
// Scenario / Start
// ---------------------------------------------------------------------------

/// Starting items for a scenario — the `"starting_items"` field.
///
/// Can be a simple list of item IDs or a structured object.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum StartingItems {
    /// Simple list of item IDs.
    Simple(Vec<String>),
    /// Structured item assignment (with counts, charges, etc.).
    Structured(Vec<HashMap<String, RawValue>>),
}

// ---------------------------------------------------------------------------
// Recipes / Crafting
// ---------------------------------------------------------------------------

/// Proficiency requirement — the `"proficiencies"` field on recipes.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProficiencyReq {
    /// Proficiency ID.
    pub proficiency: Option<String>,
    /// The proficiency-id list (alternative single format).
    /// Example: `["prof_leatherworking", 2]` or `["prof_tailoring"]`
    #[serde(default)]
    pub id: Option<String>,
    /// Time multiplier when proficient.
    #[serde(default)]
    pub time_multiplier: Option<f64>,
    /// Failure multiplier when not proficient.
    #[serde(default)]
    pub fail_multiplier: Option<f64>,
    /// Maximum expertise level considered.
    #[serde(default)]
    pub max_expertise: Option<u32>,
    /// Required level (from simple [id, level] form).
    #[serde(default)]
    pub level: Option<u32>,
    /// Learning time estimate.
    #[serde(default)]
    pub learning_time_multiplier: Option<f64>,
}

/// Batch time factors — the `"batch_time_factors"` field on recipes.
/// Batch time factors, either as an object or an array of 2 or 3 floats.
///
/// Object form: `{ "first": 1.5, "subsequent": 0.8, "reduction": 0.5 }`
/// Array forms: `[1.5, 0.8]` or `[1.5, 0.8, 0.5]`
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum BatchTimeFactors {
    /// Object form with named fields.
    Object {
        #[serde(default)]
        first: Option<f64>,
        #[serde(default)]
        subsequent: Option<f64>,
        #[serde(default)]
        reduction: Option<f64>,
    },
    /// Two-element array: `[first, subsequent]`
    Tuple2(f64, f64),
    /// Three-element array: `[first, subsequent, reduction]`
    Tuple3(f64, f64, f64),
}

// ---------------------------------------------------------------------------
// Bionics / Mutations
// ---------------------------------------------------------------------------

/// Limb score modifier — the `"limb_score_mods"` field on effects/bionics.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LimbScoreMod {
    /// Which score this modifies (e.g. "balance", "reaction", "vision").
    pub limb_score: String,
    /// Additive modifier (multiplier for the score).
    #[serde(default)]
    pub modifier: Option<f64>,
    /// Maximum value.
    #[serde(default)]
    pub max: Option<f64>,
    /// Resist modifier (how much resist traits affect this).
    #[serde(default)]
    pub resist_modifier: Option<f64>,
    /// Scaling factor per intensity.
    #[serde(default)]
    pub scaling: Option<f64>,
    /// Resist scaling factor.
    #[serde(default)]
    pub resist_scaling: Option<f64>,
}

/// Social interaction modifier — the `"social_mods"` field on bionics.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SocialMod {
    /// Which social score to modify.
    pub name: String,
    /// Additive modifier.
    #[serde(default)]
    pub modifier: Option<i32>,
    /// Maximum value.
    #[serde(default)]
    pub max: Option<i32>,
}

/// Protection value — the `"protec"` field on bionics.
#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
pub struct Protection {
    /// Armor values.
    #[serde(default)]
    pub armor: Option<ArmorValues>,
    /// Coverage percentage.
    #[serde(default)]
    pub coverage: Option<u32>,
    /// Material thickness.
    #[serde(default)]
    pub material_thickness: Option<u32>,
    /// Environmental protection.
    #[serde(default)]
    pub env_resist: Option<u32>,
}

// ---------------------------------------------------------------------------
// Enchantments
// ---------------------------------------------------------------------------

/// An enchantment entry — used by items, bionics, mutations, and effects.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Enchantment {
    /// Enchantment ID.
    #[serde(default)]
    pub id: Option<String>,
    /// Condition for the enchantment to be active.
    /// Can be a string like "ALWAYS" or an object like {"not": "u_has_weapon"}.
    #[serde(default)]
    pub condition: Option<RawValue>,
    /// Stat values modified by this enchantment.
    #[serde(default)]
    pub values: Option<Vec<HashMap<String, RawValue>>>,
    /// What items/equipment slots this requires active.
    /// Can be a string or an object/array.
    #[serde(default)]
    pub has: Option<RawValue>,
    /// Hit effects (when attacking).
    #[serde(default)]
    pub hit_you_effect: Option<Vec<HashMap<String, RawValue>>>,
    /// Hit effects (when attacked).
    #[serde(default)]
    pub hit_me_effect: Option<Vec<HashMap<String, RawValue>>>,
    /// Secondary hit effects.
    #[serde(default)]
    pub hit_secondary_effect: Option<Vec<HashMap<String, RawValue>>>,
    /// Emitter for fields.
    #[serde(default)]
    pub emitter: Option<String>,
    /// Mutation effect.
    #[serde(default)]
    pub mutator: Option<String>,
    /// Intermittent activation.
    #[serde(default)]
    pub intermittent_activation: Option<Vec<HashMap<String, RawValue>>>,
}

// ---------------------------------------------------------------------------
// Terrain / Furniture examine actions
// ---------------------------------------------------------------------------

/// An examine action — the `"examine_action"` field on terrain/furniture.
///
/// Can be a simple string action ID or an object with a type and parameters.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ExamineAction {
    /// Simple action name.
    Named(String),
    /// Structured action definition.
    Structured(ExamineActionDef),
}

/// A structured examine action definition.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExamineActionDef {
    /// Action type.
    pub r#type: String,
    /// Message shown.
    #[serde(default)]
    pub msg: Option<String>,
    /// Items needed.
    #[serde(default)]
    pub needs: Option<Vec<HashMap<String, RawValue>>>,
    /// Resulting terrain/furniture.
    #[serde(default)]
    pub result: Option<String>,
    /// Move cost.
    #[serde(default)]
    pub moves: Option<u32>,
}

// ---------------------------------------------------------------------------
// Trap types
// ---------------------------------------------------------------------------

/// Trap vehicle data — the `"vehicle_data"` field on traps.
///
/// CDDA traps can have `vehicle_data` in two formats:
/// - `{"sound_volume": N, "sound": "str"}` for noise-making traps
/// - `{"id": "vehicle_id", "chance": N}` for vehicle-spawning traps
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TrapVehicleData {
    /// The vehicle type ID to spawn.
    #[serde(default)]
    pub id: Option<String>,
    /// Chance of spawning.
    #[serde(default)]
    pub chance: Option<u32>,
    /// Faction for the vehicle.
    #[serde(default)]
    pub faction: Option<String>,
    /// Volume of the sound made when triggered (noise-making traps).
    #[serde(default)]
    pub sound_volume: Option<u32>,
    /// Sound made when triggered (noise-making traps).
    #[serde(default)]
    pub sound: Option<String>,
}

/// Trap spell data — the `"spell_data"` field on traps.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TrapSpellData {
    /// Spell ID.
    #[serde(default)]
    pub id: Option<String>,
    /// Minimum level.
    #[serde(default)]
    pub min_level: Option<u32>,
    /// Maximum level.
    #[serde(default)]
    pub max_level: Option<u32>,
    /// Spell difficulty.
    #[serde(default)]
    pub difficulty: Option<u32>,
}

// ---------------------------------------------------------------------------
// Vehicle Part types
// ---------------------------------------------------------------------------

/// Vehicle part bonus — the `"bonus"` field on vehicle parts.
#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
pub struct VehiclePartBonus {
    /// Wheel friction.
    #[serde(default)]
    pub wheel_friction: Option<f64>,
    /// Contact area.
    #[serde(default)]
    pub contact_area: Option<f64>,
    /// Rolling resistance.
    #[serde(default)]
    pub rolling_resistance: Option<f64>,
    /// Engine displacement.
    #[serde(default)]
    pub displacement: Option<u32>,
    /// Engine power.
    #[serde(default)]
    pub power: Option<i64>,
    /// Engine backfire chance.
    #[serde(default)]
    pub backfire: Option<f64>,
    /// Engine noise.
    #[serde(default)]
    pub noise: Option<u32>,
    /// Damage multiplier for collisions.
    #[serde(default)]
    pub damage_reduction: Option<u32>,
    /// Bonus to bash damage.
    #[serde(default)]
    pub bash_bonus: Option<u32>,
    /// Stowage bonus (extra capacity).
    #[serde(default)]
    pub stowage_bonus: Option<u32>,
    /// Seatbelt bonus.
    #[serde(default)]
    pub seatbelt: Option<u32>,
}

// ---------------------------------------------------------------------------
// Spawn / yield types
// ---------------------------------------------------------------------------

/// Monster spawn definition — used in `"spawns"` fields on overmap terrain, etc.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MonsterSpawn {
    /// Monster type ID.
    pub monster: String,
    /// Spawn density (1 = normal).
    #[serde(default)]
    pub density: Option<f64>,
    /// Minimum spawn count.
    #[serde(default)]
    pub min: Option<u32>,
    /// Maximum spawn count.
    #[serde(default)]
    pub max: Option<u32>,
    /// Population multiplier.
    #[serde(default)]
    pub population: Option<HashMap<String, RawValue>>,
}

/// Revive form — used in `"revive_forms"` on monsters.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReviveForm {
    /// Condition under which this revive happens.
    pub condition: Option<String>,
    /// Monster type to revive into.
    pub monster: String,
}

/// Vision levels — used in `"vision_levels"` on overmap terrain.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VisionLevels {
    /// Light level below which vision degrades.
    #[serde(default)]
    pub low: Option<u32>,
    /// Light level above which normal vision works.
    #[serde(default)]
    pub normal: Option<u32>,
    /// Maximum vision distance in this terrain.
    #[serde(default)]
    pub max: Option<u32>,
}
