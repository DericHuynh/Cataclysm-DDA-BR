use crate::raw_types::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// An OMT placeholder definition from JSON type `"omt_placeholder"`.
///
/// Defines a placeholder overmap tile that can be used for
/// terrain generation within an overmap tile.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OmtPlaceholderDef {
    /// Unique identifier (e.g. "empty_omt", "full_omt").
    pub id: DefId<OmtPlaceholderDef>,

    /// The grid of characters representing the terrain map.
    pub grid: Vec<String>,
}
