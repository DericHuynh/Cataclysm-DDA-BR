use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A snippet definition from JSON type `"snippet"`.
///
/// Defines a snippet category with associated text entries.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SnippetDef {
    /// Category identifier.
    pub category: String,

    /// Text content — can be a string, an array of strings/objects, or a single object.
    #[serde(default)]
    pub text: Option<serde_json::Value>,
}
