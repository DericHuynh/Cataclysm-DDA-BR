//! Immutable, normalized contracts for the counted-item crafting family.
use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;

/// Stable save/import identity. Never an entity index or interner token.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ItemKey(pub String);
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RecipeKey(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PocketDefinition {
    pub volume_ml: u32,
    pub weight_g: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemDefinition {
    pub key: ItemKey,
    pub name: String,
    pub description: String,
    pub category: String,
    pub volume_ml: u32,
    pub weight_g: u32,
    pub qualities: Vec<(String, i32)>,
    pub pockets: Vec<PocketDefinition>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ingredient {
    pub item: ItemKey,
    pub count: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeDefinition {
    pub key: RecipeKey,
    pub result: ItemKey,
    pub result_count: u32,
    pub work_ap: i32,
    pub category: String,
    pub subcategory: String,
    /// Every slot is required; entries within a slot are alternatives.
    pub ingredients: Vec<Vec<Ingredient>>,
    pub qualities: Vec<(String, u32)>,
}

/// Candidate and published catalog use the same immutable value representation.
/// Importers own diagnostics. Publication validates references before replacing
/// this resource; readers may retain an Arc snapshot across replacement.
#[derive(Resource, Debug, Default, Clone, Serialize, Deserialize)]
pub struct InventoryCatalog {
    pub items: BTreeMap<ItemKey, Arc<ItemDefinition>>,
    pub recipes: BTreeMap<RecipeKey, Arc<RecipeDefinition>>,
}
impl InventoryCatalog {
    pub fn validate(&self) -> Result<(), String> {
        for (key, item) in &self.items {
            if key != &item.key || key.0.is_empty() {
                return Err("Invalid item key".into());
            }
        }
        for (key, recipe) in &self.recipes {
            if key != &recipe.key
                || key.0.is_empty()
                || recipe.result_count == 0
                || recipe.work_ap <= 0
                || recipe.work_ap % 100 != 0
            {
                return Err(format!("Invalid recipe {}", key.0));
            }
            if !self.items.contains_key(&recipe.result) {
                return Err(format!(
                    "Recipe {}: unknown result {}",
                    key.0, recipe.result.0
                ));
            }
            for slot in &recipe.ingredients {
                if slot.is_empty() {
                    return Err(format!("Recipe {}: empty ingredient slot", key.0));
                }
                for ingredient in slot {
                    if ingredient.count == 0 || !self.items.contains_key(&ingredient.item) {
                        return Err(format!(
                            "Recipe {}: invalid ingredient {}",
                            key.0, ingredient.item.0
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}

/// Immutable item snapshot held by an instance. Reload policy: existing items
/// retain their generation's definition; newly spawned items use the new one.
#[derive(Component, Debug, Clone)]
pub struct ItemDefinitionRef(pub Arc<ItemDefinition>);
