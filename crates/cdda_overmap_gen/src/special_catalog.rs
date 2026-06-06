//! Special catalog — overmap special definitions loaded from JSON.
//!
//! In the full implementation, this is populated from `DefRegistry.overmap_specials`
//! during data loading. For now, it's a lightweight placeholder.

use bevy_ecs::prelude::*;

/// An overmap special definition (fixed or mutable).
#[derive(Debug, Clone)]
pub struct SpecialDef {
    pub id: String,
    pub subtype: String,
    pub locations: Vec<String>,
    pub occurrences: Option<(i32, i32)>,
    pub city_sizes: Option<(i32, i32)>,
    pub city_distance: Option<(i32, i32)>,
    pub flags: Vec<String>,
    pub overmaps: Option<Vec<SpecialOmt>>,
    pub root: Option<String>,
    pub phases: Option<Vec<SpecialPhase>>,
}

/// An individual OMT placement within a special.
#[derive(Debug, Clone)]
pub struct SpecialOmt {
    pub id: String,
    pub dx: i32,
    pub dy: i32,
    pub dz: i32,
}

/// A phase of a mutable special.
#[derive(Debug, Clone)]
pub struct SpecialPhase {
    pub rules: Vec<SpecialRule>,
}

/// A placement rule within a mutable special phase.
#[derive(Debug, Clone)]
pub struct SpecialRule {
    pub max: i32,
    pub weight: i32,
    pub overmap_ids: Vec<String>,
}

/// Catalog of overmap specials available for placement.
#[derive(Resource, Debug, Clone, Default)]
pub struct SpecialCatalog {
    /// Fixed specials.
    pub fixed_specials: Vec<SpecialDef>,
    /// Mutable specials.
    pub mutable_specials: Vec<SpecialDef>,
}

impl SpecialCatalog {
    pub fn empty() -> Self {
        Self::default()
    }

    /// Stub: construct from a DefRegistry. In the full implementation this
    /// reads `overmap_specials` from the registry.
    pub fn from_registry(_registry: &cdda_data::DefRegistry) -> Self {
        Self::default()
    }
}
