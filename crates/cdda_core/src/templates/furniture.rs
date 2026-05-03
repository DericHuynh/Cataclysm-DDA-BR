//! # Furniture templates
//!
//! Blueprint types for furniture definitions — objects placed *on top of*
//! terrain (tables, chairs, counters, beds, workbenches, etc.).

use crate::flags::FlagSet;

/// The blueprint for a furniture definition.
///
/// Furniture is the dynamic overlay layer on terrain — it can be moved,
/// destroyed, or interacted with separately from the underlying terrain tile.
#[derive(Debug, Clone, PartialEq)]
pub struct FurnitureTemplate {
    /// Display name.
    pub name: String,
    /// Flavour / examine text.
    pub description: String,
    /// Map-display character.
    pub symbol: char,
    /// Boolean tags (e.g. BLOCK_MOVEMENT, TRANSPARENT, SEALED).
    pub flags: FlagSet,
    /// Movement-cost modifier added to the terrain's base move cost.
    pub move_cost_mod: i32,
    /// Percentage of cover it provides (0–100).
    pub coverage: u32,
    /// Strength required to move / bash this furniture.
    pub required_str: i32,
}
