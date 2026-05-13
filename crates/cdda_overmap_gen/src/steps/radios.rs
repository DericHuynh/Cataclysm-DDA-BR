//! Step: Place radio towers — one per city with a 1-in-3 chance.
//!
//! Simplified port of CDDA's `overmap::place_radios()` (overmap.cpp L3666-3697).

use bevy_ecs::prelude::*;
use cdda_overmap::rng::XorShiftRng;
use crate::pipeline::OvermapGenConfig;
use crate::steps::cities::City;
use tracing::info;

/// A placed radio tower entity.
#[derive(Component)]
pub struct RadioTower {
    pub strength: i32,
    pub message: String,
    pub omt_x: i32,
    pub omt_y: i32,
}

/// Place radio towers — one per city.
pub fn place_radios(
    mut commands: Commands,
    cities: Query<&City>,
    config: Res<OvermapGenConfig>,
) {
    let mut rng = XorShiftRng::new(config.noise_seed as u64 + 17);

    for city in &cities {
        if rng.one_in(3) {
            let strength = rng.range_i32(80, 120);
            commands.spawn(RadioTower {
                strength,
                message: format!("radio_station_{}", city.omt_x),
                omt_x: city.omt_x,
                omt_y: city.omt_y,
            });
        }
    }

    info!(
        "Radio towers placed for overmap ({}, {})",
        config.om_x, config.om_y
    );
}
