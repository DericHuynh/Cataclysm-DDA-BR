//! Healing phase — natural healing, first aid, body part recovery.
//!
//! STUB: Not yet implemented.

use bevy_ecs::prelude::*;

pub fn healing_phase(world: &mut World) {
    // STUB: no-op until healing implemented
    let _ = world;
}

pub fn calculate_healing_rate(
    _current_hp: i32,
    _max_hp: i32,
    _sleeping: bool,
    _well_fed: bool,
) -> f32 {
    todo!("healing rate calculation not yet implemented");
}

pub fn apply_first_aid(_body_part: Entity, _bandage_quality: u32, _disinfectant: bool) -> i32 {
    todo!("first aid application not yet implemented");
}
