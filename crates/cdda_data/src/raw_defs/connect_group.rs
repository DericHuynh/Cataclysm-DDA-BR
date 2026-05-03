use crate::raw_types::{DefId, LocalizedString};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A connect group definition from JSON type `"connect_group"`.
///
/// Defines a group of terrain tiles that visually connect with each other
/// (e.g. WALL, PAVEMENT, WATER).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ConnectGroupDef {
    /// Unique identifier (e.g. "WALL", "PAVEMENT", "WATER").
    pub id: DefId<ConnectGroupDef>,

    /// Display name of the connect group.
    #[serde(default)]
    pub name: Option<LocalizedString>,
}
