use crate::worldgen::dev::DevWorldgenConfig;
use crate::worldgen::setup::WorldMapResource;
use bevy_app::{App, Plugin};

pub struct WorldgenPlugin;

impl Plugin for WorldgenPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldMapResource>();
        app.init_resource::<DevWorldgenConfig>();
    }
}
