//! Catalog of overmap connection definitions for generation systems.
//!
//! Extracted from `DefRegistry.overmap_connections` during data loading and
//! inserted as a Bevy [`Resource`].

use bevy_ecs::prelude::*;
use cdda_core_types::core::raw_defs::overmap_terrain::OvermapConnectionDef;
use std::sync::Arc;

/// Catalog of overmap connections extracted from DefRegistry.
///
/// Connections define roads, railroads, sewer lines, and other linear features
/// that link overmap locations.
#[derive(Resource, Debug, Clone, Default)]
pub struct ConnectionCatalog {
    pub connections: Vec<Arc<OvermapConnectionDef>>,
}

impl ConnectionCatalog {
    /// Build from a `DefRegistry`, collecting all resolved overmap connections.
    pub fn from_registry(registry: &cdda_data::DefRegistry) -> Self {
        Self {
            connections: registry.overmap_connections.values().cloned().collect(),
        }
    }

    pub fn len(&self) -> usize {
        self.connections.len()
    }

    pub fn is_empty(&self) -> bool {
        self.connections.is_empty()
    }
}
