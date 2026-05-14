use crate::registry::DefRegistry;
use bevy_ecs::prelude::*;
use std::sync::Arc;

/// Bevy Resource wrapper for the resolved DefRegistry.
/// Saved in the World after data loading so registry viewer and other
/// runtime tools can access all definitions.
#[derive(Resource, Clone)]
pub struct DefRegistryResource(pub Arc<DefRegistry>);
