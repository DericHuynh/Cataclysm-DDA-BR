use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// An overlay order definition from JSON type `"overlay_order"`.
///
/// Defines the rendering order for mutation/trait overlays on the player character.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OverlayOrderDef {
    /// List of overlay ordering entries with IDs and order values.
    pub overlay_ordering: Vec<serde_json::Value>,
}
