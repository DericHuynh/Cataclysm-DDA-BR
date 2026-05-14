use bevy_ecs::prelude::*;
use serde_json::Value;
use std::collections::HashMap;

/// Stores raw JSON values from data files, keyed by (type_category → id → raw_value).
/// Used by the registry viewer for side-by-side comparison with parsed structs.
#[derive(Resource, Clone)]
pub struct RawDefinitionValues {
    /// Map from type category string ("ITEM", "MONSTER", etc.) to (id → raw JSON value).
    pub values: HashMap<String, HashMap<String, Value>>,
}

impl RawDefinitionValues {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    /// Look up the raw JSON value for a specific definition.
    pub fn get_raw(&self, category: &str, id: &str) -> Option<&Value> {
        self.values.get(category).and_then(|m| m.get(id))
    }
}

impl Default for RawDefinitionValues {
    fn default() -> Self {
        Self::new()
    }
}
