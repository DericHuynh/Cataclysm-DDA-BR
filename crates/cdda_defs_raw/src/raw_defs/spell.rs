use crate::raw_types::{DefId, LocalizedString};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A spell definition from JSON type `"SPELL"`.
///
/// Defines a magical spell or special ability with various effects,
/// targeting rules, scaling parameters, and resource costs.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SpellDef {
    /// Unique identifier (e.g. "summon_trapspider", "burning_hands").
    pub id: DefId<SpellDef>,

    /// Display name of the spell.
    #[serde(default)]
    pub name: Option<LocalizedString>,

    /// Description of the spell's effect.
    #[serde(default)]
    pub description: Option<LocalizedString>,

    /// Primary effect type (e.g. "spawn_item", "summon", "attack", "revive", "upgrade", "noise", "guilt").
    #[serde(default)]
    pub effect: Option<serde_json::Value>,

    /// Effect target string (e.g. item ID, monster group ID, effect ID).
    #[serde(default)]
    pub effect_str: Option<String>,

    /// Shape of the spell's area of effect (e.g. "blast", "cone", "line").
    #[serde(default)]
    pub shape: Option<String>,

    /// Valid target types for the spell (e.g. ["ground", "self", "hostile", "ally"]).
    /// Can be a sequence or a map/object.
    #[serde(default)]
    pub valid_targets: Option<serde_json::Value>,

    /// Spell flags (e.g. "SILENT", "CONCENTRATE", "VERBAL", "SOMATIC", "PERMANENT", "NO_EXPLOSION_SFX").
    /// Can be a sequence or a map/object.
    #[serde(default)]
    pub flags: Option<serde_json::Value>,

    /// Maximum level attainable for this spell (numeric or math expression).
    #[serde(default)]
    pub max_level: Option<serde_json::Value>,

    /// Minimum damage at level 0 (numeric or math expression).
    #[serde(default)]
    pub min_damage: Option<serde_json::Value>,

    /// Maximum damage at max_level (numeric or math expression).
    #[serde(default)]
    pub max_damage: Option<serde_json::Value>,

    /// Damage increment per level (numeric or math expression).
    #[serde(default)]
    pub damage_increment: Option<serde_json::Value>,

    /// Minimum range at level 0 (numeric or math expression).
    #[serde(default)]
    pub min_range: Option<serde_json::Value>,

    /// Maximum range at max_level (numeric or math expression).
    #[serde(default)]
    pub max_range: Option<serde_json::Value>,

    /// Range increment per level (numeric or math expression).
    #[serde(default)]
    pub range_increment: Option<serde_json::Value>,

    /// Minimum area of effect radius at level 0 (numeric or math expression).
    #[serde(default)]
    pub min_aoe: Option<serde_json::Value>,

    /// Maximum area of effect radius at max_level (numeric or math expression).
    #[serde(default)]
    pub max_aoe: Option<serde_json::Value>,

    /// Area of effect increment per level (numeric or math expression).
    #[serde(default)]
    pub aoe_increment: Option<serde_json::Value>,

    /// Minimum duration (in movement points/moves) at level 0 (numeric or math expression).
    #[serde(default)]
    pub min_duration: Option<serde_json::Value>,

    /// Maximum duration at max_level (numeric or math expression).
    #[serde(default)]
    pub max_duration: Option<serde_json::Value>,

    /// Duration increment per level (numeric or math expression).
    #[serde(default)]
    pub duration_increment: Option<serde_json::Value>,

    /// Spell class (e.g. "NONE", "ALCHEMIST", "MAGUS", "KEEN").
    #[serde(default)]
    pub spell_class: Option<String>,

    /// Base casting time in moves (numeric or math expression).
    #[serde(default)]
    pub base_casting_time: Option<serde_json::Value>,

    /// Final casting time at max_level (numeric or math expression).
    #[serde(default)]
    pub final_casting_time: Option<serde_json::Value>,

    /// Casting time change per level (can be negative, numeric or math expression).
    #[serde(default)]
    pub casting_time_increment: Option<serde_json::Value>,

    /// Base energy cost (numeric or math expression).
    #[serde(default)]
    pub base_energy_cost: Option<serde_json::Value>,

    /// Final energy cost at max_level (numeric or math expression).
    #[serde(default)]
    pub final_energy_cost: Option<serde_json::Value>,

    /// Energy cost change per level (can be negative, numeric or math expression).
    #[serde(default)]
    pub energy_increment: Option<serde_json::Value>,

    /// Energy source (e.g. "MANA", "HP", "STAMINA", "BIONIC", "FATIGUE").
    #[serde(default)]
    pub energy_source: Option<String>,

    /// Spell difficulty (numeric or math expression).
    #[serde(default)]
    pub difficulty: Option<serde_json::Value>,

    /// Field type to spawn (e.g. "fd_fatigue", "fd_acid", "fd_blood", "fd_sludge").
    #[serde(default)]
    pub field_id: Option<String>,

    /// Chance of field spawning each tick (numeric or math expression).
    #[serde(default)]
    pub field_chance: Option<serde_json::Value>,

    /// Minimum field intensity (numeric or math expression).
    #[serde(default)]
    pub min_field_intensity: Option<serde_json::Value>,

    /// Maximum field intensity (numeric or math expression).
    #[serde(default)]
    pub max_field_intensity: Option<serde_json::Value>,

    /// Field intensity increment per level (numeric or math expression).
    #[serde(default)]
    pub field_intensity_increment: Option<serde_json::Value>,

    /// Sound type played on cast (e.g. "combat", "spell").
    #[serde(default)]
    pub sound_type: Option<String>,

    /// Sound ID played on cast.
    #[serde(default)]
    pub sound_id: Option<String>,

    /// Sound variant played on cast.
    #[serde(default)]
    pub sound_variant: Option<String>,

    /// Message displayed when the spell is cast.
    #[serde(default)]
    pub message: Option<String>,

    /// Skill that improves this spell.
    #[serde(default)]
    pub skill: Option<String>,

    /// List of spells learned when this spell reaches certain levels.
    /// Can be a sequence or a map of spell_id: level.
    #[serde(default)]
    pub learn_spells: Option<serde_json::Value>,

    /// Components required to cast the spell (references a component definition).
    #[serde(default)]
    pub components: Option<serde_json::Value>,

    /// Additional spells linked to this one (extra effects).
    #[serde(default)]
    pub extra_effects: Option<Vec<serde_json::Value>>,

    /// Additional linked spells that are also cast.
    #[serde(default)]
    pub additional_linked_spells: Option<Vec<serde_json::Value>>,

    /// Magic type for the spell.
    #[serde(default)]
    pub magic_type: Option<String>,

    /// Sound of the spell's field.
    #[serde(default)]
    pub sound: Option<String>,
}
