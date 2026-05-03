use crate::raw_types::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A json_flag definition from JSON type `"json_flag"`.
///
/// Defines a flag that can be referenced by other definitions (e.g. terrain, items, monsters).
/// Flags are used to tag game objects with specific behaviors or properties.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct JsonFlagDef {
    /// Unique identifier (e.g. "DEBUG_ONLY", "SWIM_UNDER").
    pub id: DefId<JsonFlagDef>,

    /// Info text describing the flag's purpose.
    #[serde(default)]
    pub info: Option<String>,

    /// Description text.
    #[serde(default)]
    pub description: Option<String>,

    /// Restriction text (e.g. "Item must be a chainmail compatible armor piece").
    #[serde(default)]
    pub restriction: Option<String>,

    /// Abstract flag — if true, this definition is a template that should not be
    /// instantiated directly.
    #[serde(default)]
    pub abstract_: Option<bool>,

    /// Base definition id to copy fields from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_from: Option<String>,
}
