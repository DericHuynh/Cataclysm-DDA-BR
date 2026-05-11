use bevy_ecs::prelude::Resource;
use bevy_app::{App, Plugin};

use crate::spatial::EntitySpatialIndex;

/// Wrapper to store `WorldMap` as a Bevy resource.
/// `WorldMap` lives in the zero-bevy `cdda_map` crate, so it cannot
/// derive `Resource` directly.
#[derive(Resource, Debug, Clone)]
pub struct WorldMapResource(pub crate::WorldMap);

impl Default for WorldMapResource {
    fn default() -> Self {
        Self(crate::WorldMap::new())
    }
}

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EntitySpatialIndex>();
        app.init_resource::<WorldMapResource>();
    }
}
