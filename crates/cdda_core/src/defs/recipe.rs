use crate::types::{DefId, LocalizedString};
use crate::units::Time;
use serde::{Deserialize, Serialize};

/// A crafting recipe definition from JSON type `"recipe"`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeDef {
    /// Unique identifier (usually the result item ID).
    pub id: Option<String>,

    /// Result item from this recipe.
    pub result: DefId<crate::defs::item::ItemDef>,

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
    pub skill_used: Option<DefId<crate::defs::skill::SkillDef>>,

    /// Difficulty level (0 = trivial).
    #[serde(default)]
    pub difficulty: u32,

    /// Required skills and their minimum level.
    #[serde(default)]
    pub skills_required: Option<Vec<SkillRequirement>>,

    /// Time to craft (in-game time).
    #[serde(default)]
    pub time: Option<Time>,

    /// Number of charges the result has.
    #[serde(default)]
    pub charges: Option<u32>,

    /// Whether this recipe is automatically learned.
    #[serde(default)]
    pub autolearn: Option<bool>,

    /// Whether this recipe is reversible (uncraft).
    #[serde(default)]
    pub reversible: Option<bool>,

    /// Books that teach this recipe.
    #[serde(default)]
    pub book_learn: Option<Vec<BookLearn>>,

    /// Flags.
    #[serde(default)]
    pub flags: Vec<String>,

    /// Tool qualities required.
    #[serde(default)]
    pub qualities: Option<Vec<QualReq>>,

    /// Tools required.
    #[serde(default)]
    pub tools: Option<Vec<Vec<ToolOption>>>,

    /// Components required.
    #[serde(default)]
    pub components: Option<Vec<Vec<ComponentOption>>>,

    /// Using statement (reuse component lists from other recipes).
    #[serde(default)]
    pub using: Option<Vec<String>>,

    /// Container item for result.
    #[serde(default)]
    pub container: Option<DefId<crate::defs::item::ItemDef>>,

    /// Catch-all
    #[serde(default)]
    pub extra: Option<serde_json::Value>,

    /// Deconstruct learn
    #[serde(default)]
    pub decomp_learn: Option<serde_json::Value>,

    /// Blueprint name
    #[serde(default)]
    pub blueprint_name: Option<String>,

    /// Blueprint excludes
    #[serde(default)]
    pub blueprint_excludes: Option<serde_json::Value>,

    /// Blueprint provides
    #[serde(default)]
    pub blueprint_provides: Option<serde_json::Value>,

    /// Blueprint requires
    #[serde(default)]
    pub blueprint_requires: Option<serde_json::Value>,

    /// Construction blueprint
    #[serde(default)]
    pub construction_blueprint: Option<String>,

    /// Never learn automatically
    #[serde(default)]
    pub never_learn: Option<bool>,

    /// Byproducts
    #[serde(default)]
    pub byproducts: Option<Vec<String>>,

    /// Batch time factors
    #[serde(default)]
    pub batch_time_factors: Option<serde_json::Value>,

    /// Proficiencies required
    #[serde(default)]
    pub proficiencies: Option<Vec<serde_json::Value>>,

    /// ID suffix
    #[serde(default)]
    pub id_suffix: Option<String>,

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

/// A skill level requirement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillRequirement {
    /// Skill ID.
    pub id: DefId<crate::defs::skill::SkillDef>,
    /// Minimum level.
    pub level: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillRequirementTuple(pub DefId<crate::defs::skill::SkillDef>, pub u32);

/// Book-learn entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookLearn {
    /// Book item ID.
    pub book: DefId<crate::defs::item::ItemDef>,
    /// Skill level required.
    pub skill_level: u32,
}

/// Quality requirement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualReq {
    /// Quality type ID.
    pub id: String,
    /// Quality level needed.
    pub level: u32,
}

/// Tool option (multiple alternatives per tool slot).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolOption {
    /// Simple: ["item_id", required_count]
    Simple(String, u32),
    /// With charge requirement: ["item_id", required_count, charge_count]
    WithCharges(String, u32, u32),
    /// Object form.
    Object(ToolOptionObj),
}

/// Tool option object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOptionObj {
    pub item: String,
    #[serde(default)]
    pub count: Option<u32>,
    #[serde(default)]
    pub charges: Option<u32>,
    #[serde(default)]
    pub list: Option<bool>,
}

/// Component option (multiple alternatives per component slot).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ComponentOption {
    /// Simple pair: ["item_id", count]
    Simple(String, u32),
    /// Object form.
    Object(ComponentOptionObj),
}

/// Component option object.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
