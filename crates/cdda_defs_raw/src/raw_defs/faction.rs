use crate::raw_defs::cdda_types::RawValue;
use crate::raw_types::{DefId, LocalizedString};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A faction definition from JSON type `"faction"`.
///
/// Factions are groups with their own goals, relationships, and territories.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FactionDef {
    /// Unique identifier (e.g. "free_merchants", "hells_raiders").
    pub id: DefId<FactionDef>,

    /// Display name.
    pub name: LocalizedString,

    /// Description text.
    #[serde(default)]
    pub description: Option<LocalizedString>,

    /// Likes us.
    #[serde(default)]
    pub likes_u: Option<i32>,

    /// Respects us.
    #[serde(default)]
    pub respects_u: Option<i32>,

    /// Known by us.
    #[serde(default)]
    pub known_by_u: Option<bool>,

    /// Starting currency held.
    #[serde(default)]
    pub currency: Option<String>,

    /// Price markup percentage. CDDA format: array of {"group": "...", "markup": 2} or map.
    #[serde(default)]
    pub price_rules: Option<RawValue>,

    /// Whether the faction is playable.
    #[serde(default)]
    pub playable: Option<bool>,

    /// Whether the faction is standard.
    #[serde(default)]
    pub standard: Option<bool>,

    /// Faction color for the overmap.
    #[serde(default)]
    pub color: Option<crate::raw_defs::cdda_types::CddaColor>,

    /// Faction map symbol.
    #[serde(default)]
    pub map_symbol: Option<String>,

    /// Size of the faction.
    #[serde(default)]
    pub size: Option<crate::raw_defs::cdda_types::FactionSize>,

    /// Power level.
    #[serde(default)]
    pub power: Option<i32>,

    /// Combat ability rating.
    #[serde(default)]
    pub combat_ability: Option<i32>,

    /// Food supply rating (complex CDDA food array).
    #[serde(default)]
    pub fac_food_supply: Option<Vec<RawValue>>,

    /// Whether the faction consumes food.
    #[serde(default)]
    pub consumes_food: Option<bool>,

    /// Wealth.
    #[serde(default)]
    pub wealth: Option<i32>,

    /// Relationship with other factions (nested map).
    #[serde(default)]
    pub relations: Option<HashMap<String, HashMap<String, RawValue>>>,

    /// Likes values.
    #[serde(default)]
    pub likes: Option<Vec<String>>,

    /// Dislikes values.
    #[serde(default)]
    pub dislikes: Option<Vec<String>>,

    /// Monster faction.
    #[serde(default)]
    pub mon_faction: Option<String>,

    /// Epilogues.
    #[serde(default)]
    pub epilogues: Option<Vec<RawValue>>,

    /// Flags.
    #[serde(default)]
    pub flags: Vec<String>,
}

/// Reputation with another faction.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FactionReputation {
    /// Faction ID.
    pub faction: String,
    /// Reputation value.
    pub reputation: i32,
}

/// Price rule for dealing with the faction.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FactionPriceRule {
    /// Resource type.
    pub resource: String,
    /// Markup percentage.
    pub markup: f64,
}

/// Relationship with another faction.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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
