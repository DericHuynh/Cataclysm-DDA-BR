//! # Scenario templates
//!
//! Blueprint types for scenario definitions — starting conditions that
//! determine the player's initial location, equipment, and challenge.

use crate::id::*;

/// The blueprint for a scenario definition.
///
/// Scenarios are the character-creation starting points — they set the
/// player's initial location, optional starting gear, and sometimes
/// scenario-specific challenges or restrictions.
#[derive(Debug, Clone, PartialEq)]
pub struct ScenarioTemplate {
    /// Display name.
    pub name: String,
    /// Flavour / examine description.
    pub description: String,
    /// The start location / shelter type (e.g. evacuee shelter, lab start).
    pub start_location: StartLocationId,
}
