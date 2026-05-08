//! String interner — stable u32 IDs for hot-reloadable JSON data.
//!
//! Uses `BiMap<String, u32>` for bidirectional O(1) lookups.
//! Append-only: IDs never change during hot-reload, ensuring runtime
//! entity references remain valid.

use bevy_ecs::prelude::*;
use bidimap::BiMap;

/// Bidirectional string ↔ u32 mapping.  Append-only — once a string gets
/// an ID, that ID is stable for the lifetime of the session.
#[derive(Resource, Debug, Clone)]
pub struct StringInterner {
    map: BiMap<String, u32>,
    next_id: u32,
}

impl Default for StringInterner {
    fn default() -> Self {
        Self {
            map: BiMap::new(),
            next_id: 0,
        }
    }
}

impl StringInterner {
    /// Get the ID for a string, allocating one if it doesn't exist.
    pub fn intern(&mut self, s: &str) -> u32 {
        if let Some(&id) = self.map.get_by_left(s) {
            return id;
        }
        let id = self.next_id;
        self.next_id += 1;
        self.map.insert(s.to_string(), id);
        id
    }

    /// Look up an ID without allocating.  Returns `None` if unknown.
    pub fn get(&self, s: &str) -> Option<u32> {
        self.map.get_by_left(s).copied()
    }

    /// Look up a string by ID.
    pub fn resolve(&self, id: u32) -> Option<&str> {
        self.map.get_by_right(&id).map(|s| s.as_str())
    }
}

// ── Typed ID wrappers ─────────────────────────────────────────────────────

macro_rules! typed_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name(pub u32);
        impl $name {
            pub fn index(self) -> usize {
                self.0 as usize
            }
        }
    };
}

typed_id!(ItemId);
typed_id!(MonsterId);
typed_id!(TerrainId);
typed_id!(FurnitureId);
typed_id!(FlagId);
typed_id!(RecipeId);
typed_id!(BodyPartId);
