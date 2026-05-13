//! Catalog of overmap special definitions for generation systems.
//!
//! Extracted from `DefRegistry.overmap_specials` during data loading and
//! inserted as a Bevy [`Resource`].

use bevy_ecs::prelude::*;
use cdda_core_types::core::raw_defs::overmap_terrain::OvermapSpecialDef;
use std::sync::Arc;

/// Catalog of overmap specials extracted from DefRegistry.
///
/// This is a thin wrapper around a `Vec<Arc<OvermapSpecialDef>>` so generation
/// systems can access all specials via `Res<SpecialCatalog>`.
#[derive(Resource, Debug, Clone, Default)]
pub struct SpecialCatalog {
    pub specials: Vec<Arc<OvermapSpecialDef>>,
}

impl SpecialCatalog {
    /// Build from a `DefRegistry`, collecting all resolved overmap specials.
    pub fn from_registry(registry: &cdda_data::DefRegistry) -> Self {
        Self {
            specials: registry.overmap_specials.values().cloned().collect(),
        }
    }

    pub fn len(&self) -> usize {
        self.specials.len()
    }

    pub fn is_empty(&self) -> bool {
        self.specials.is_empty()
    }
}
