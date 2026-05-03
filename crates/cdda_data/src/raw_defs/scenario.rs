use crate::raw_types::{DefId, LocalizedString};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A scenario definition from JSON type `"scenario"`.
///
/// Scenarios define the starting conditions for a new game:
/// what profession, location, traits, and equipment the player begins with.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ScenarioDef {
    /// Unique identifier (e.g. "evacuee", "lab_chal").
    pub id: DefId<ScenarioDef>,

    /// Display name.
    pub name: LocalizedString,

    /// Description text.
    pub description: LocalizedString,

    /// Points cost (negative = bonus points, positive = costs points).
    #[serde(default)]
    pub points: Option<i32>,

    /// Starting location ID.
    pub start_location: Option<DefId<crate::raw_defs::start_location::StartLocationDef>>,

    /// Allowed starting locations.
    #[serde(default)]
    pub allowed_locations: Vec<String>,

    /// Professions allowed in this scenario.
    #[serde(default)]
    pub professions: Vec<String>,

    /// Default profession if none selected.
    #[serde(default)]
    pub default_profession: Option<String>,

    /// Professions that are forbidden.
    #[serde(default)]
    pub forbidden_professions: Vec<String>,

    /// Traits the player always starts with.
    #[serde(default)]
    pub forced_traits: Vec<DefId<crate::raw_defs::mutation::MutationDef>>,

    /// Traits that are forbidden.
    #[serde(default)]
    pub forbidden_traits: Vec<DefId<crate::raw_defs::mutation::MutationDef>>,

    /// Allowed traits.
    #[serde(default)]
    pub allowed_traits: Vec<DefId<crate::raw_defs::mutation::MutationDef>>,

    /// Additional items to start with.
    #[serde(default)]
    pub starting_items: Option<crate::raw_defs::cdda_types::StartingItems>,

    /// Starting vehicle.
    #[serde(default)]
    pub vehicle: Option<String>,

    /// Missions automatically started (array of mission IDs).
    #[serde(default)]
    pub missions: Vec<String>,

    /// Flags.
    #[serde(default)]
    pub flags: Vec<String>,

    /// EOC effects to run at game start.
    #[serde(default)]
    pub eoc_at_start: Option<Vec<String>>,

    /// Whether this scenario adds a vehicle.
    #[serde(default)]
    pub add_vehicle: Option<bool>,

    /// Whether use of the scenario is allowed (locked / unlocked).
    #[serde(default)]
    pub unlocked: Option<bool>,

    /// Monsters to spawn at start.
    #[serde(default)]
    pub start_monsters: Option<Vec<ScenarioStartMonster>>,

    /// Terrain types that surround the start location.
    #[serde(default)]
    pub surrounding_terrain: Option<String>,

    /// Map extras.
    #[serde(default)]
    pub map_extra: Option<String>,

    /// Whether to place player in a shelter.
    #[serde(default)]
    pub shelter: Option<bool>,

    /// Start of cataclysm
    #[serde(default)]
    pub start_of_cataclysm: Option<bool>,

    /// Reveal locale
    #[serde(default)]
    pub reveal_locale: Option<bool>,

    /// EOC effects
    #[serde(default)]
    pub eoc: Option<Vec<String>>,

    /// Distance initial visibility
    #[serde(default)]
    pub distance_initial_visibility: Option<u32>,

    /// Requirements
    #[serde(default)]
    pub requirement: Option<String>,

    /// Start name (for display)
    #[serde(default)]
    pub start_name: Option<LocalizedString>,

    /// Allowed locations (short form)
    #[serde(default)]
    pub allowed_locs: Option<Vec<String>>,

    /// copy-from parent
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_from: Option<String>,
}

/// A monster that spawns at game start.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ScenarioStartMonster {
    /// Monster ID.
    pub monster: String,
    /// Position offset from player.
    #[serde(default)]
    pub x: Option<i32>,
    #[serde(default)]
    pub y: Option<i32>,
    /// Chance of spawning.
    #[serde(default)]
    pub chance: Option<u32>,
}
