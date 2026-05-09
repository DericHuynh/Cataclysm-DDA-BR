use bevy_app::{App, Plugin};

use crate::map::spatial::EntitySpatialIndex;

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EntitySpatialIndex>();
    }
}
