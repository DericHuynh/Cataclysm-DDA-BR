//! # Overmap-terrain templates
//!
//! Blueprint types for overmap-terrain definitions — the coarse-grained tiles
//! that make up the world-map (overmap) layer.

use crate::flags::FlagSet;

/// The blueprint for an overmap-terrain definition.
///
/// Overmap terrains are the large-scale tiles on the world map — forests,
/// cities, rivers, labs, etc.  Each tile controls mapgen and travel costs
/// at the overmap level.
#[derive(Debug, Clone, PartialEq)]
pub struct OvermapTerrainTemplate {
    /// Display name (e.g. "Forest", "River", "Lab").
    pub name: String,
    /// Map-display character.
    pub symbol: char,
    /// Boolean tags (e.g. CITY, FOREST, WATER, LAB).
    pub flags: FlagSet,
    /// Cost to reveal this tile via overmap sight.
    pub see_cost: u32,
    /// Movement cost when travelling across this tile on the overmap.
    pub travel_cost: u32,
}
