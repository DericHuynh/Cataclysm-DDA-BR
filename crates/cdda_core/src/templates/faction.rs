//! # Faction templates
//!
//! Blueprint types for faction definitions — organised groups of NPCs with
//! shared goals, relationships, and currency.

use crate::id::*;

/// The blueprint for a faction definition.
///
/// Factions represent organised groups in the game world — survivor camps,
/// bandit gangs, trade caravans, etc.  Each faction has a currency item and
/// manages its own relationship graph with other factions.
#[derive(Debug, Clone, PartialEq)]
pub struct FactionTemplate {
    /// Display name.
    pub name: String,
    /// Flavour / examine description.
    pub description: String,
    /// The item used as this faction's internal currency (e.g. bottle caps,
    /// bullets, old currency).
    pub currency: ItemId,
}
