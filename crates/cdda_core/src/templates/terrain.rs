//! # Terrain templates
//!
//! Blueprint types for map terrain definitions — the tiles that make up the
//! game world.

use crate::flags::FlagSet;
use crate::id::*;

/// The blueprint for a terrain tile definition.
///
/// Terrain is the static base layer of the map — floors, walls, windows,
/// doors, grass, water, etc.
#[derive(Debug, Clone, PartialEq)]
pub struct TerrainTemplate {
    /// Display name.
    pub name: String,
    /// Flavour / examine text.
    pub description: String,
    /// Map-display character.
    pub symbol: char,
    /// Boolean tags (e.g. FLAMMABLE, NO_FLOOR, TRANSPARENT).
    pub flags: FlagSet,
    /// Movement cost in moves (0 = impassable, ≥1 = walkable).
    pub move_cost: i32,
    /// How much light this tile emits (0 = none).
    pub light_emitted: u32,
    /// Terrain placed above this one (if any — e.g. floor → roof).
    pub roof: Option<TerrainId>,
    /// Damage needed to bash through (None = un-bashable).
    pub bash: Option<u32>,
}
