use bevy_app::{App, Plugin};
use crate::worldgen::setup::WorldMapResource;
use crate::worldgen::dev::DevWorldgenConfig;

pub struct WorldgenPlugin;

impl Plugin for WorldgenPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldMapResource>();
        app.init_resource::<DevWorldgenConfig>();
    }
}
