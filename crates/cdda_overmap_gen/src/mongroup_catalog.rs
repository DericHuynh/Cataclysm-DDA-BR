//! Catalog of monster group definitions for generation systems.
//!
//! Extracted from `DefRegistry.monster_groups` during data loading and
//! inserted as a Bevy [`Resource`].

use bevy_ecs::prelude::*;
use cdda_core_types::core::raw_defs::monstergroup::MonsterGroupDef;
use std::sync::Arc;

/// Catalog of monster groups extracted from DefRegistry.
///
/// Monster groups define weighted sets of monsters that can spawn in various
/// overmap locations (city streets, forests, swamps, labs, etc.).
#[derive(Resource, Debug, Clone, Default)]
pub struct MongroupCatalog {
    pub groups: Vec<Arc<MonsterGroupDef>>,
}

impl MongroupCatalog {
    /// Build from a `DefRegistry`, collecting all resolved monster groups.
    pub fn from_registry(registry: &cdda_data::DefRegistry) -> Self {
        Self {
            groups: registry.monster_groups.values().cloned().collect(),
        }
    }

    pub fn len(&self) -> usize {
        self.groups.len()
    }

    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }
}
