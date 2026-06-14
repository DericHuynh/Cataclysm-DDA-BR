use crate::raw_types::{DefId, LocalizedString};
use cdda_core_types::core::units::Time;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A crafting recipe definition from JSON type `"recipe"`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RecipeDef {
    /// Unique identifier (usually the result item ID).
    pub id: Option<String>,

    /// Result item from this recipe.
    /// Optional for abstract or practice recipes.
    #[serde(default)]
    pub result: Option<DefId<crate::raw_defs::item::ItemDef>>,

    /// Result item count.
    #[serde(default)]
    pub result_mult: Option<u32>,

    /// Display name for the recipe.
    #[serde(default)]
    pub name: Option<LocalizedString>,

    /// Description text.
    #[serde(default)]
    pub description: Option<LocalizedString>,

    /// Recipe category (e.g. "CC_CHEM").
    #[serde(default)]
    pub category: Option<String>,

    /// Recipe subcategory (e.g. "CSC_CHEM_DRUGS").
    #[serde(default)]
    pub subcategory: Option<String>,

    /// Activity level required by this recipe.
    #[serde(default)]
    pub activity_level: Option<String>,

    /// Primary skill used.
    #[serde(default)]
    pub skill_used: Option<DefId<crate::raw_defs::skill::SkillDef>>,

    /// Difficulty level (0 = trivial).
    #[serde(default)]
    pub difficulty: u32,

    /// Required skills and their minimum level.
    /// Accepts multiple formats:
    /// - `[["fabrication", 3], ["mechanics", 2]]` (array of tuples)
    /// - `["fabrication", 3]` (single skill shorthand)
    /// - `{"fabrication": 3, "mechanics": 2}` (object map)
    #[serde(default)]
    pub skills_required: Option<SkillsRequired>,

    /// Time to craft (in-game time).
    #[serde(default)]
    pub time: Option<Time>,

    /// Number of charges the result has.
    #[serde(default)]
    pub charges: Option<u32>,

    /// Whether this recipe is automatically learned.
    /// Accepts a boolean or a list of skill requirements
    /// `[["skill_id", level], ...]`.
    #[serde(default)]
    pub autolearn: Option<Autolearn>,

    /// Whether this recipe is reversible (uncraft).
    /// Can be a boolean `true` or an object `{"time": "5 s"}`.
    #[serde(default)]
    pub reversible: Option<Reversible>,

    /// Books that teach this recipe.
    /// Can be an array `[["book_id", skill_level], ...]` or an object
    /// `{"book_id": {"skill_level": 2, ...}}`.
    #[serde(default)]
    pub book_learn: Option<BookLearnList>,

    /// Flags.
    #[serde(default)]
    pub flags: Vec<String>,

    /// Tool qualities required.
    /// Each entry can be a single quality object or a list of alternatives:
    /// - `{"id": "ANVIL", "level": 3}`
    /// - `[{"id": "SAW_M", "level": 1}, {"id": "DRILL", "level": 1}]`
    #[serde(default)]
    pub qualities: Option<Vec<QualityEntry>>,

    /// Tools required.
    #[serde(default)]
    pub tools: Option<Vec<Vec<ToolOption>>>,

    /// Components required.
    #[serde(default)]
    pub components: Option<Vec<Vec<ComponentOption>>>,

    /// Using statement (reuse component lists from other recipes).
    /// Each entry is `["recipe_id", count]`.
    #[serde(default)]
    pub using: Option<Vec<UsingEntry>>,

    /// Container item for result.
    #[serde(default)]
    pub container: Option<DefId<crate::raw_defs::item::ItemDef>>,

    /// Deconstruct learn — how the recipe is learned when deconstructing
    /// the result item. Can be a boolean, a level number, or a list of
    /// skill requirements.
    #[serde(default)]
    pub decomp_learn: Option<DecompLearn>,

    /// Blueprint name (for faction camp blueprints).
    #[serde(default)]
    pub blueprint_name: Option<String>,

    /// Blueprint excludes — other blueprint IDs that conflict with this one.
    #[serde(default)]
    pub blueprint_excludes: Option<Vec<BlueprintReq>>,

    /// Blueprint provides — capabilities this blueprint adds to a camp.
    #[serde(default)]
    pub blueprint_provides: Option<Vec<BlueprintReq>>,

    /// Blueprint requires — other blueprints that must exist first.
    #[serde(default)]
    pub blueprint_requires: Option<Vec<BlueprintReq>>,

    /// Construction blueprint (the camp tile type built).
    #[serde(default)]
    pub construction_blueprint: Option<String>,

    /// Never learn automatically.
    #[serde(default)]
    pub never_learn: Option<bool>,

    /// Byproducts produced by this recipe.
    /// Each entry can be `["item_id"]` (count=1) or `["item_id", count]`.
    #[serde(default)]
    pub byproducts: Option<Vec<ByproductEntry>>,

    /// Batch time factors.
    #[serde(default)]
    pub batch_time_factors: Option<crate::raw_defs::cdda_types::BatchTimeFactors>,

    /// Proficiencies required.
    #[serde(default)]
    pub proficiencies: Option<Vec<crate::raw_defs::cdda_types::ProficiencyReq>>,

    /// ID suffix.
    #[serde(default)]
    pub id_suffix: Option<String>,

    /// Abstract flag.
    #[serde(default)]
    pub abstract_: Option<bool>,

    /// copy-from parent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_from: Option<String>,
}

/// Skill requirements in one of several CDDA JSON formats.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum SkillsRequired {
    /// Array of skill requirement entries: `[["fabrication", 3], ["mechanics", 2]]`
    Multi(Vec<SkillRequirement>),
    /// Single skill shorthand: `["fabrication", 3]`
    SingleFlat(DefId<crate::raw_defs::skill::SkillDef>, u32),
    /// Object map: `{"fabrication": 3, "mechanics": 2}`
    Map(HashMap<String, u32>),
}

/// A skill level requirement, either as an object `{"id": "...", "level": N}`
/// or a tuple `["skill_name", level]`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum SkillRequirement {
    /// Object form: `{ "id": "fabrication", "level": 3 }`
    Object {
        id: DefId<crate::raw_defs::skill::SkillDef>,
        level: u32,
    },
    /// Tuple form: `["fabrication", 3]`
    Tuple(DefId<crate::raw_defs::skill::SkillDef>, u32),
}

/// How a recipe is learned from deconstructing its result item.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum DecompLearn {
    /// A skill level (numeric shorthand).
    Level(u32),
    /// Boolean flag (`true` = learn from deconstructing).
    Bool(bool),
    /// A list of skill requirements.
    Skills(Vec<SkillRequirement>),
}

/// A reference to another blueprint (by ID), with an optional amount.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BlueprintReq {
    /// Blueprint ID.
    pub id: String,
    /// Optional amount.
    #[serde(default)]
    pub amount: Option<u32>,
}

/// Whether a recipe is reversible. Can be `true`/`false` or an object
/// specifying uncraft time: `{"time": "5 m"}`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Reversible {
    /// Simple boolean: `true` or `false`.
    Bool(bool),
    /// Uncraft properties.
    Obj {
        /// Time the uncraft takes.
        #[serde(default)]
        time: Option<Time>,
        /// Whether the uncraft can fail.
        #[serde(default)]
        move_cost: Option<u32>,
    },
}

/// An entry in a `using` list, which reuses component lists from another
/// recipe. Each entry is `["recipe_id", count]` where count may be a
/// floating-point multiplier.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UsingEntry(
    /// Recipe ID whose component lists to reuse.
    pub String,
    /// Scale factor (may be a float like `1.5`).
    pub f64,
);

/// An entry in a `byproducts` list. Each entry can be:
/// - `["item_id"]` (count defaults to 1)
/// - `["item_id", count]`
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ByproductEntry {
    /// Single-element array: `["item_id"]` (count=1).
    Single([String; 1]),
    /// Two-element array: `["item_id", count]`.
    WithCount(String, u32),
}

/// How a recipe is automatically learned.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Autolearn {
    /// Simple boolean: `true` or `false`.
    Bool(bool),
    /// Skill requirements: `[["skill_id", level], ...]`.
    Skills(Vec<SkillRequirement>),
}

/// Book-learn entries, either as a list or a map of book IDs to details.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum BookLearnList {
    /// Array format: `[["book_id", skill_level], ...]`
    Array(Vec<BookLearnEntry>),
    /// Object/map format: `{"book_id": {"skill_level": 2, ...}}`
    Map(std::collections::HashMap<String, BookLearnEntryObj>),
}

/// A single book-learn entry in array format.
///
/// CDDA stores each entry as either:
/// - `["book_id"]` (just the book ID)
/// - `["book_id", skill_level]`
///
/// Note: `["book_id"]` is a 1-element array, so we use `[String; 1]`
/// to handle it (a 1-tuple newtype would deserialize from a bare string).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum BookLearnEntry {
    /// `["book_id", skill_level]`
    WithLevel(String, u32),
    /// `["book_id"]` (skill_level defaults to 0)
    IdOnly([String; 1]),
}

/// A single book-learn entry in object format (value side of the map).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BookLearnEntryObj {
    /// Skill level required to learn from this book.
    #[serde(default)]
    pub skill_level: Option<u32>,
    /// Custom recipe name shown for this book.
    #[serde(default)]
    pub recipe_name: Option<String>,
    /// If true, can only be learned from this book.
    #[serde(default)]
    pub exclusive: Option<bool>,
    /// If true, hides the recipe from the book.
    #[serde(default)]
    pub secret: Option<bool>,
    /// If true, the book is consumed on use.
    #[serde(default)]
    pub consumed: Option<bool>,
}

/// A single quality entry: either a quality object or an array of
/// alternative quality objects.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum QualityEntry {
    /// Single quality: `{"id": "ANVIL", "level": 3}`
    Single(QualReq),
    /// Alternatives list: `[{"id": "SAW_M", "level": 1}, {"id": "CUT", "level": 2}]`
    Alternative(Vec<QualReq>),
}

/// Quality requirement.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct QualReq {
    /// Quality type ID.
    pub id: String,
    /// Quality level needed.
    pub level: u32,
}

/// Tool option (multiple alternatives per tool slot).
///
/// CDDA accepts these JSON shapes:
/// - A plain string: `"hammer"` (count=1, no charges)
/// - A 2-element array: `["hammer", 2]` (item ID + count; count may be -1)
/// - A 3-element array: `["soldering_iron", 1, 100]` (item ID + count + charges)
/// - A 3-element array with string flag: `["metalworking_tongs_any", 1, "LIST"]`
/// - An object: `{"item": "soldering_iron", "count": 1, "charges": 100}`
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ToolOption {
    /// Plain string ID: `"hammer"` (implies count=1, no charges).
    SimpleId(String),
    /// Simple pair: `["item_id", required_count]` (count may be -1).
    Simple(String, i32),
    /// With charge requirement: `["item_id", required_count, charge_count]`.
    WithCharges(String, i32, u32),
    /// With a string flag (e.g. "LIST"): `["item_id", count, "FLAG"]`.
    WithFlag(String, i32, String),
    /// Object form.
    Object(ToolOptionObj),
}

/// Tool option object.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ToolOptionObj {
    /// Item ID.
    pub item: String,
    /// Required count (may be -1 for "use but don't consume").
    #[serde(default)]
    pub count: Option<i32>,
    /// Required charges.
    #[serde(default)]
    pub charges: Option<u32>,
    /// List flag.
    #[serde(default)]
    pub list: Option<bool>,
}

/// Component option (multiple alternatives per component slot).
///
/// CDDA accepts these JSON shapes:
/// - A plain string: `"nail"` (matches just the item ID)
/// - A 2-element array: `["nail", 100]` (item ID + count)
/// - A 3-element array with string flag: `["nail", 100, "LIST"]`
/// - An object: `{"item": "nail", "count": 100, ...}`
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ComponentOption {
    /// Plain string ID: `"nail"`
    SimpleId(String),
    /// Simple pair: `["item_id", count]`
    Simple(String, u32),
    /// With a string flag (e.g. "LIST"): `["item_id", count, "FLAG"]`.
    WithFlag(String, u32, String),
    /// Object form.
    Object(ComponentOptionObj),
}

/// Component option object.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ComponentOptionObj {
    pub item: String,
    #[serde(default)]
    pub count: Option<u32>,
    #[serde(default)]
    pub charges: Option<u32>,
    #[serde(default)]
    pub list: Option<bool>,
    #[serde(default)]
    pub prob: Option<u32>,
    #[serde(default)]
    pub required: Option<bool>,
    #[serde(default)]
    pub recoverable: Option<bool>,
}
