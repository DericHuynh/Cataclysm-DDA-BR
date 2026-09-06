//! Publication adapter for worlds using the native inventory catalog.
use bevy_ecs::prelude::*;
use cdda_catalog::{
    definition::{DefCategory, DefinitionWorld},
    interner::{ItemTypeRegistry, QualityRegistry},
    inventory::*,
};
use cdda_components::{def::*, item::ItemQualities, recipe::RecipeIndex};

/// Publish a complete native inventory candidate. Validate before any mutation.
/// This entry point owns the definition set; mixing legacy definition builders
/// into the same world is rejected. Runtime item snapshots retain old definitions.
pub fn publish_inventory(world: &mut World, catalog: InventoryCatalog) -> Result<u64, String> {
    catalog.validate()?;
    if world.contains_resource::<DefinitionWorld>()
        && !world.contains_resource::<InventoryCatalog>()
    {
        return Err("Native publication requires a native catalog world".into());
    }
    let generation = world
        .get_resource::<DefinitionWorld>()
        .map(|d| d.generation())
        .unwrap_or(0)
        .checked_add(1)
        .ok_or("Catalog generation overflow")?;
    let mut types = world
        .get_resource::<ItemTypeRegistry>()
        .cloned()
        .unwrap_or_default();
    let mut qualities = world
        .get_resource::<QualityRegistry>()
        .cloned()
        .unwrap_or_default();
    // Preflight interner capacity, including both supplied and required qualities.
    let new_qualities: std::collections::BTreeSet<_> = catalog
        .items
        .values()
        .flat_map(|i| i.qualities.iter().map(|q| q.0.as_str()))
        .chain(
            catalog
                .recipes
                .values()
                .flat_map(|r| r.qualities.iter().map(|q| q.0.as_str())),
        )
        .filter(|q| qualities.get(q).is_none())
        .collect();
    if qualities.len() + new_qualities.len() >= u16::MAX as usize {
        return Err("Quality token space exhausted".into());
    }
    for key in catalog.items.keys() {
        types.intern(&key.0);
    }
    for key in new_qualities {
        qualities.intern(key);
    }
    let old: Vec<_> = world
        .get_resource::<DefinitionWorld>()
        .into_iter()
        .flat_map(|d| d.iter().map(|(_, _, e)| e))
        .collect();
    let mut index = DefinitionWorld::at_generation(generation);
    for item in catalog.items.values() {
        let entity = world
            .spawn((
                IsDef,
                DefStrId(item.key.0.clone()),
                ItemName(item.name.clone()),
                ItemDescription(item.description.clone()),
                ItemCategory(item.category.clone()),
                ItemVolume(item.volume_ml),
                ItemWeight(item.weight_g),
                ItemDefinitionRef(item.clone()),
                ItemQualities(
                    item.qualities
                        .iter()
                        .map(|(q, n)| (qualities.get(q).unwrap(), *n))
                        .collect(),
                ),
            ))
            .id();
        index.register(DefCategory::Item, item.key.0.clone(), entity);
    }
    let mut recipes = Vec::new();
    for recipe in catalog.recipes.values() {
        let entity = world
            .spawn((
                IsDef,
                IsRecipeDef,
                DefStrId(recipe.key.0.clone()),
                RecipeResult(recipe.result.0.clone()),
                RecipeResultCount(recipe.result_count),
                RecipeTime((recipe.work_ap / 100) as u32),
                RecipeCategory(recipe.category.clone()),
                RecipeSubcategory(recipe.subcategory.clone()),
                RecipeComponents(
                    recipe
                        .ingredients
                        .iter()
                        .map(|s| {
                            s.iter()
                                .map(|i| RecipeComponentEntry {
                                    item_id: types.get(&i.item.0).unwrap(),
                                    count: i.count,
                                    recovered: false,
                                })
                                .collect()
                        })
                        .collect(),
                ),
                RecipeQualities(
                    recipe
                        .qualities
                        .iter()
                        .map(|(q, n)| (qualities.get(q).unwrap(), *n))
                        .collect(),
                ),
            ))
            .id();
        recipes.push(entity);
        index.register(DefCategory::Recipe, recipe.key.0.clone(), entity);
    }
    world.insert_resource(types);
    world.insert_resource(qualities);
    world.insert_resource(RecipeIndex(recipes));
    world.insert_resource(index);
    world.insert_resource(catalog);
    for entity in old {
        world.despawn(entity);
    }
    Ok(generation)
}
