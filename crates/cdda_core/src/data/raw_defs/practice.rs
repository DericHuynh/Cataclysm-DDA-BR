use crate::data::raw_types::{DefId, LocalizedString};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A practice recipe definition from JSON type `"practice"`.
///
/// Defines a practice activity that the player can perform to train skills
/// and proficiencies without consuming resources (or with minimal resource use).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PracticeDef {
    /// Unique identifier (e.g. "research_biochemistry").
    pub id: DefId<PracticeDef>,

    /// Display name (can be localized).
    pub name: LocalizedString,

    /// Description text (can be localized).
    #[serde(default)]
    pub description: Option<LocalizedString>,

    /// Activity level required (e.g. "LIGHT_EXERCISE", "MODERATE_EXERCISE").
    #[serde(default)]
    pub activity_level: Option<String>,

    /// Skill category (e.g. "CC_PRACTICE").
    pub category: String,

    /// Skill subcategory (e.g. "CSC_PRACTICE_SCIENCE").
    pub subcategory: String,

    /// Skill used by this practice (e.g. "chemistry").
    #[serde(default)]
    pub skill_used: Option<String>,

    /// Practice difficulty and skill limit data.
    #[serde(default)]
    pub practice_data: Option<PracticeData>,

    /// Proficiency requirements (e.g. `[{"proficiency": "prof_intro_biology", "required": true}]`).
    #[serde(default)]
    pub proficiencies: Option<Vec<serde_json::Value>>,

    /// Time required to complete the practice (e.g. "1 h").
    pub time: String,

    /// Tools required (flexible CDDA format: can be string, nested array, or complex object).
    #[serde(default)]
    pub tools: Option<serde_json::Value>,

    /// Components required (flexible CDDA format: can be string, nested array, or complex object).
    #[serde(default)]
    pub components: Option<serde_json::Value>,

    /// Autolearn conditions (skill levels at which this is automatically learned).
    #[serde(default)]
    pub autolearn: Option<Vec<Vec<serde_json::Value>>>,

    /// Flags for this practice recipe.
    #[serde(default)]
    pub flags: Vec<String>,
}

/// Difficulty and skill limit data for a practice recipe.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PracticeData {
    /// Minimum required difficulty level.
    pub min_difficulty: u32,

    /// Maximum difficulty level achievable.
    pub max_difficulty: u32,

    /// Maximum skill level achievable through this practice.
    #[serde(default)]
    pub skill_limit: Option<u32>,
}
