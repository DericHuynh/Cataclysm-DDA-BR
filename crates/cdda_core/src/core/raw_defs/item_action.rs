use crate::core::raw_types::{DefId, LocalizedString};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// An item_action definition from JSON type `"item_action"`.
///
/// Defines an action that can be performed with an item (e.g. "CROWBAR", "PICKAXE",
/// "repair_fabric"). These are referenced from item definitions' `"use_action"` field.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ItemActionDef {
    /// Unique identifier (e.g. "CROWBAR", "repair_fabric", "PICK_LOCK").
    pub id: DefId<ItemActionDef>,

    /// Display name describing the action (may be a plain string or localized object).
    #[serde(default)]
    pub name: Option<LocalizedString>,
}
