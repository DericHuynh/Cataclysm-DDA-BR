use crate::data::raw_types::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// An NPC definition from JSON type `"npc"`.
///
/// Defines a non-player character template with class, faction,
/// attitude, and optional inline traits/skills.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NpcDef {
    /// Unique identifier (e.g. "deserter", "marloss_voice", "apis").
    pub id: DefId<NpcDef>,

    /// Unique display name (overrides the NPC class name).
    #[serde(default)]
    pub name_unique: Option<String>,

    /// Name suffix appended to the class name (e.g. "Deserter", "chef").
    /// Can be a plain string or a translatable object {"str": "..."}.
    #[serde(default)]
    pub name_suffix: Option<serde_json::Value>,

    /// NPC class ID (e.g. "NC_SOLDIER", "NC_FARMER").
    #[serde(default)]
    pub class: Option<String>,

    /// Attitude towards the player (numeric 0-10 or string).
    /// 0 = friendly, 10 = hostile.
    #[serde(default)]
    pub attitude: Option<i32>,

    /// Mission value (numeric).
    #[serde(default)]
    pub mission: Option<i32>,

    /// Starting chat topic (e.g. "TALK_HELLO", "TALK_DONE").
    #[serde(default)]
    pub chat: Option<String>,

    /// Faction ID (e.g. "no_faction", "marloss", "free_merchants").
    #[serde(default)]
    pub faction: Option<String>,

    /// Optional mission(s) offered by this NPC.
    /// Can be a single mission ID string or an array of mission IDs.
    #[serde(default)]
    pub mission_offered: Option<serde_json::Value>,

    /// Age of the NPC.
    #[serde(default)]
    pub age: Option<i32>,

    /// Height of the NPC.
    #[serde(default)]
    pub height: Option<i32>,

    /// Gender: "male", "female", or other.
    #[serde(default)]
    pub gender: Option<String>,

    /// Body type.
    #[serde(default)]
    pub body_type: Option<String>,

    /// Personality traits (complex object).
    #[serde(default)]
    pub personality: Option<serde_json::Value>,

    /// Optional inline traits. Can be an array of strings or
    /// an array of objects with `trait`, `group`, and `weight`.
    #[serde(default)]
    pub traits: Option<serde_json::Value>,

    /// Optional inline skills. Array of objects with `skill` and `level`.
    #[serde(default)]
    pub skills: Option<serde_json::Value>,

    /// Optional inline proficiencies.
    #[serde(default)]
    pub proficiencies: Option<serde_json::Value>,

    /// Stats overrides (complex object with str/dex/int/per).
    #[serde(default)]
    pub stats: Option<serde_json::Value>,

    /// Melee skill level.
    #[serde(default)]
    pub melee_skill: Option<serde_json::Value>,

    /// Optional list of bionics.
    #[serde(default)]
    pub bionics: Option<serde_json::Value>,

    /// Worn armor item group override.
    #[serde(default)]
    pub worn_armor: Option<String>,

    /// Carry inventory item group override.
    #[serde(default)]
    pub carry_override: Option<String>,

    /// Abstract flag — if true, this definition is a template.
    #[serde(default)]
    pub abstract_: Option<bool>,

    /// Base definition id to copy fields from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_from: Option<String>,
}
