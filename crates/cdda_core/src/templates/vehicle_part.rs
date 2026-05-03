//! # Vehicle-part templates
//!
//! Blueprint types for vehicle-part definitions — the building blocks that
//! make up cars, trucks, bikes, and other player-constructed vehicles.

use crate::flags::FlagSet;

/// The blueprint for a vehicle-part definition.
///
/// Vehicle parts are installed on vehicle frames to add functionality:
/// engines, wheels, seats, storage, armour, etc.
#[derive(Debug, Clone, PartialEq)]
pub struct VehiclePartTemplate {
    /// Display name.
    pub name: String,
    /// Map-display character.
    pub symbol: char,
    /// Boolean tags (e.g. ENGINE, WHEEL, SEAT, CARGO).
    pub flags: FlagSet,
    /// How much damage this part can absorb before breaking.
    pub durability: u32,
    /// Damage modifier applied when this part is used in collisions.
    pub damage_modifier: u32,
}
