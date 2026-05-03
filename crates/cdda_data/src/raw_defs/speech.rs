use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A speech definition from JSON type `"speech"`.
///
/// Defines a sound that a monster or NPC can make, along with its volume.
/// CDDA format: `{"speaker": ["mon_id1", "mon_id2"], "sound": "...", "volume": 20}`
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SpeechDef {
    /// List of monster IDs that can produce this speech.
    #[serde(default)]
    pub speaker: Vec<String>,

    /// The sound/text produced.
    pub sound: String,

    /// Volume of the sound.
    #[serde(default)]
    pub volume: u32,
}
