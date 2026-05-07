//! Temperature and spoilage system — body temperature regulation, spoilage processing.
//!
//! Handles:
//! - Body temperature updates for creatures (ambient temp + clothing)
//! - Warmth and insulation calculations from worn items
//! - Spoilage rate for perishable items based on temperature
//! - Tick processing for both temperature and spoilage

use crate::def_components::ArmourPart;
use bevy_ecs::prelude::*;
use cdda_actor::components::BodyTemperature;
use cdda_item::components::Spoilable;

/// Update body temperature for a creature based on ambient temperature and worn items.
pub fn update_body_temperature(world: &mut World, entity: Entity, ambient_temp_celsius: f64) {
    let _ = (world, entity, ambient_temp_celsius);
    todo!("body temp regulation: ambient + warmth - insulation → new BodyTemperature")
}

/// Calculate total warmth from all worn items on a creature.
pub fn calculate_total_warmth(world: &World, entity: Entity) -> i32 {
    let _ = (world, entity);
    todo!("sum warmth from all WornBy items")
}

/// Calculate total insulation from armour parts and material thickness.
pub fn calculate_insulation(armour_parts: &[ArmourPart], material_thickness: f32) -> f32 {
    let _ = (armour_parts, material_thickness);
    todo!("insulation = material_thickness * sum(part coverage fraction)")
}

/// Calculate the spoilage rate multiplier for a given temperature and container state.
pub fn spoilage_rate(temp_celsius: f64, is_sealed: bool, preserves_temp: bool) -> f64 {
    let _ = (temp_celsius, is_sealed, preserves_temp);
    todo!("spoilage rate: if frozen → 0, if sealed+preserves → 0, else temperature-based curve")
}

/// Process spoilage for all items with the Spoilable component.
pub fn tick_spoilage(world: &mut World) {
    let _ = world;
}

/// Process temperature regulation for all creatures.
pub fn tick_temperature(world: &mut World) {
    let _ = world;
}

/// Keep old stub for backward compatibility.
pub fn temperature_phase(world: &mut World) {
    tick_temperature(world);
    tick_spoilage(world);
}
