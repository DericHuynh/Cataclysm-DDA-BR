use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A mood_face definition from JSON type `"mood_face"`.
///
/// Defines a set of mood faces for the player character, with different
/// faces shown at different morale values.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MoodFaceDef {
    /// Unique identifier (e.g. "DEFAULT", "THRESH_FELINE").
    pub id: String,

    /// List of mood value-to-face mappings.
    #[serde(default)]
    pub values: Option<Vec<MoodFaceValue>>,

    /// Base definition id to copy fields from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_from: Option<String>,

    /// Abstract flag — if true, this definition is a template that should not be
    /// instantiated directly.
    #[serde(default)]
    pub abstract_: Option<bool>,
}

/// A single mood face entry mapping a morale value threshold to a face string.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MoodFaceValue {
    /// Morale value threshold.
    pub value: i32,

    /// Face string (may include color markup like `<color_green>:)</color>`).
    pub face: String,
}
