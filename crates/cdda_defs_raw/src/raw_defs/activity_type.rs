use crate::raw_types::{DefId, LocalizedString};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// How activity progress is measured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum BasedOnType {
    /// Progress measured in real time (independent of character speed).
    Time,
    /// Progress measured in character move points (speed-dependent).
    #[default]
    Speed,
    /// Progress driven entirely by the do_turn handler or actor.
    Neither,
}

/// An event that can interrupt an ongoing activity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DistractionType {
    Noise,
    Pain,
    Attacked,
    HostileSpottedFar,
    HostileSpottedNear,
    TalkedTo,
    Asthma,
    MotionAlarm,
    WeatherChange,
    PortalStormPopup,
    Eoc,
    DangerousField,
    Hunger,
    Thirst,
    Temperature,
    Mutation,
    Oxygen,
    Withdrawal,
}

/// Named exertion level applied to an ongoing activity's stamina cost.
///
/// CDDA's `activity_level` is a string enum in JSON (e.g. `"NO_EXERCISE"`),
/// interpreted as a multiplier. We model the named level here; the numeric
/// multiplier is derived in the simulation layer.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Serialize, Deserialize, JsonSchema, Default,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ActivityExertion {
    /// No stamina cost.
    #[default]
    NoExercise,
    /// Very light exertion.
    LightExercise,
    /// Moderate exertion.
    ModerateExercise,
    /// Brisk exertion.
    BriskExercise,
    /// Heavy exertion.
    ActiveExercise,
    /// Extra-heavy exertion.
    ExtraExercise,
}

/// An activity type definition from JSON type `"activity_type"`.
///
/// Defines a player activity (e.g. reading, reloading, crafting) with its
/// verb display text and behavioral flags. Maps directly to the C++
/// `activity_type` class fields loaded from `activity_type.cpp::load()`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ActivityTypeDef {
    /// Unique identifier (e.g. "ACT_RELOAD", "ACT_READ").
    pub id: DefId<ActivityTypeDef>,

    /// Verb phrase describing the activity (e.g. "reloading", "reading").
    #[serde(default)]
    pub verb: Option<LocalizedString>,

    /// Whether the character is rooted in place during the activity.
    #[serde(default)]
    pub rooted: Option<bool>,

    /// Whether the activity can be interrupted by distractions.
    #[serde(default = "default_true")]
    pub interruptable: bool,

    /// Whether the activity can be interrupted by keyboard input.
    #[serde(default = "default_true")]
    pub interruptable_with_kb: bool,

    /// Whether the activity can be suspended and resumed.
    #[serde(default = "default_true")]
    pub can_resume: bool,

    /// Whether this is a multi-activity (runs multiple sub-activities).
    #[serde(default)]
    pub multi_activity: bool,

    /// Whether the activity should fetch items to an associated zone.
    #[serde(default = "default_true")]
    pub fetch_items_to_zone: bool,

    /// If true, the character will refuel an adjacent fire if firewood is nearby.
    #[serde(default)]
    pub refuel_fires: bool,

    /// If true, the character will automatically consume from auto-eat/drink zones.
    #[serde(default)]
    pub auto_needs: bool,

    /// How activity progress is measured.
    #[serde(default)]
    pub based_on: BasedOnType,

    /// Exertion level during this activity (NO_EXERCISE = 0.0 to MAX_EXERCISE).
    #[serde(default)]
    pub activity_level: ActivityExertion,

    /// Distraction types that are ignored by default for this activity.
    #[serde(default)]
    pub ignored_distractions: Vec<DistractionType>,

    /// Effect-on-condition to run when the activity completes.
    #[serde(default)]
    pub completion_eoc: Option<String>,

    /// Effect-on-condition to run each turn of the activity.
    #[serde(default)]
    pub do_turn_eoc: Option<String>,
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Regression: CDDA `activity_type` JSON uses string `activity_level`
    /// (e.g. "NO_EXERCISE") and lowercase `based_on` ("time"/"speed"/"neither").
    /// These must deserialize (the struct previously declared `f32` and
    /// SCREAMING_SNAKE_CASE, breaking parse).
    #[test]
    fn deserializes_real_activity_type() {
        let json = json!({
            "type": "activity_type",
            "id": "act_read_test",
            "verb": "reading",
            "rooted": true,
            "based_on": "time",
            "activity_level": "MODERATE_EXERCISE",
            "interruptable_with_kb": false
        });
        let def: ActivityTypeDef = serde_json::from_value(json).unwrap();
        assert_eq!(def.based_on, BasedOnType::Time);
        assert_eq!(def.activity_level, ActivityExertion::ModerateExercise);
        assert_eq!(def.interruptable_with_kb, false);
    }

    /// All six exertion levels parse from their CDDA string forms.
    #[test]
    fn all_exertion_levels_parse() {
        for (s, expect) in [
            ("NO_EXERCISE", ActivityExertion::NoExercise),
            ("LIGHT_EXERCISE", ActivityExertion::LightExercise),
            ("MODERATE_EXERCISE", ActivityExertion::ModerateExercise),
            ("BRISK_EXERCISE", ActivityExertion::BriskExercise),
            ("ACTIVE_EXERCISE", ActivityExertion::ActiveExercise),
            ("EXTRA_EXERCISE", ActivityExertion::ExtraExercise),
        ] {
            let v = serde_json::Value::String(s.to_string());
            let parsed: ActivityExertion = serde_json::from_value(v).unwrap();
            assert_eq!(parsed, expect);
        }
    }
}
