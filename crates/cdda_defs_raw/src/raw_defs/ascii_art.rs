use cdda_core_types::core::id::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// An ascii_art definition from JSON type `"ascii_art"`.
///
/// Defines an ASCII art picture that can be referenced by items and other game objects.
/// The picture is an array of strings, each representing a line of ASCII art.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AsciiArtDef {
    /// Unique identifier (e.g. "10mm_fmj", "223").
    pub id: DefId<AsciiArtDef>,

    /// The ASCII art picture, as an array of strings (each string is a line).
    #[serde(default)]
    pub picture: Vec<String>,

    /// Base definition id to copy fields from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_from: Option<String>,
}
