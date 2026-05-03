use crate::types::{DefId, LocalizedString};
use serde::{Deserialize, Serialize};

/// A faction definition from JSON type `"faction"`.
///
/// Factions are groups with their own goals, relationships, and territories.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactionDef {
    /// Unique identifier (e.g. "free_merchants", "hells_raiders").
    pub id: DefId<FactionDef>,

    /// Display name.
    pub name: LocalizedString,

    /// Description text.
    #[serde(default)]
    pub description: Option<LocalizedString>,

    /// Starting reputation with the faction.
    #[serde(default)]
    pub reputations: Option<serde_json::Value>,

    /// Starting currency held.
    #[serde(default)]
    pub currency: Option<String>,

    /// Price markup percentage.
    #[serde(default)]
    pub price_rules: Option<serde_json::Value>,

    /// Whether the faction is playable.
    #[serde(default)]
    pub playable: Option<bool>,

    /// Whether the faction is standard.
    #[serde(default)]
    pub standard: Option<bool>,

    /// Faction color for the overmap.
    #[serde(default)]
    pub color: Option<serde_json::Value>,

    /// Faction map symbol.
    #[serde(default)]
    pub map_symbol: Option<String>,

    /// Size of the faction.
    #[serde(default)]
    pub size: Option<serde_json::Value>,

    /// Power level.
    #[serde(default)]
    pub power: Option<i32>,

    /// Combat ability rating.
    #[serde(default)]
    pub combat_ability: Option<i32>,

    /// Food supply rating (can be integer or complex CDDA food array).
    #[serde(default)]
    #[serde(alias = "fac_food_supply")]
    pub food_supply: Option<serde_json::Value>,

    /// Wealth.
    #[serde(default)]
    pub wealth: Option<i32>,

    /// Relationship with other factions.
    #[serde(default)]
    pub relations: Option<serde_json::Value>,

    /// Likes values.
    #[serde(default)]
    #[serde(alias = "likes_u")]
    pub likes: Option<serde_json::Value>,

    /// Dislikes values.
    #[serde(default)]
    pub dislikes: Option<serde_json::Value>,

    /// Flags.
    #[serde(default)]
    pub flags: Vec<String>,
}

/// Reputation with another faction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactionReputation {
    /// Faction ID.
    pub faction: String,
    /// Reputation value.
    pub reputation: i32,
}

/// Price rule for dealing with the faction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactionPriceRule {
    /// Resource type.
    pub resource: String,
    /// Markup percentage.
    pub markup: f64,
}

/// Relationship with another faction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactionRelation {
    /// Faction ID.
    pub faction: String,
    /// Whether they are allied.
    #[serde(default)]
    pub ally: Option<bool>,
    /// Whether they are neutral.
    #[serde(default)]
    pub neutral: Option<bool>,
    /// Whether they are hostile.
    #[serde(default)]
    pub hostile: Option<bool>,
    /// Whether they are part of the same faction.
    #[serde(default)]
    pub same_faction: Option<bool>,
}
