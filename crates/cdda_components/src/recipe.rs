//! Recipe-related types.

use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::Resource;

/// All recipe definition entities, built during `build_def_world`.
/// Used by the crafting menu to enumerate available recipes.
#[derive(Resource, Default, Clone)]
pub struct RecipeIndex(pub Vec<Entity>);
