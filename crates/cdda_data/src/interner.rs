//! String interner — stable typed IDs for hot-reloadable JSON data.
//!
//! Uses `BiMap<String, T>` for bidirectional O(1) lookups per token type.
//! Append-only: IDs never change during hot-reload, ensuring runtime
//! entity references remain valid.
//!
//! ## Registry types
//!
//! - `QualityRegistry` — quality-name → QualityId mapping
//! - `ItemTypeRegistry` — item-type string → ItemTypeId mapping
//! - Additional registries via the `token_registry!` macro
//!
//! ## Adding a new token type
//!
//! 1. Define the newtype in `cdda_components/src/tokens.rs`:
//!    ```ignore
//!    intern_token!(SkillId, u16);
//!    ```
//! 2. Create the registry via the macro:
//!    ```ignore
//!    token_registry!(SkillRegistry, SkillId, u16);
//!    ```
//! 3. Init it in `CddaDataPlugin::build()`.

use bevy_ecs::prelude::*;
use bidimap::BiMap;
use cdda_components::item::QualityId;
use cdda_components::AmmoTypeId;
use cdda_components::BodyPartId;
use cdda_components::ComestibleId;
use cdda_components::ItemTypeId;
use cdda_components::SkillId;

// ---------------------------------------------------------------------------
// Macro: token_registry! — generates a typed string-interning registry
// ---------------------------------------------------------------------------

/// Generates a typed `Resource` registry for interning strings.
///
/// ```ignore
/// token_registry!(SkillRegistry, SkillId, u16);
/// ```
///
/// Produces a struct with `intern`, `get`, `resolve`, `iter`, `len`, `is_empty`.
macro_rules! token_registry {
    ($name:ident, $token:ty, $inner:ty) => {
        #[doc = concat!("Registry mapping string → ", stringify!($token), ".")]
        #[derive(Resource, Debug, Clone)]
        pub struct $name {
            map: BiMap<String, $token>,
            next_id: $inner,
        }

        impl Default for $name {
            fn default() -> Self {
                Self {
                    map: BiMap::new(),
                    next_id: 0,
                }
            }
        }

        impl $name {
            /// Get the token for a string, allocating one if new.
            pub fn intern(&mut self, s: &str) -> $token {
                if let Some(&id) = self.map.get_by_left(s) {
                    return id;
                }
                let id = <$token>::new(self.next_id);
                self.next_id += 1;
                self.map.insert(s.to_string(), id);
                id
            }

            /// Look up a string without allocating. Returns `None` if unknown.
            pub fn get(&self, s: &str) -> Option<$token> {
                self.map.get_by_left(s).copied()
            }

            /// Look up a string by its token.
            pub fn resolve(&self, id: $token) -> Option<&str> {
                self.map.get_by_right(&id).map(|s| s.as_str())
            }

            /// Iterate over all registered (string, token) pairs.
            pub fn iter(&self) -> impl Iterator<Item = (&str, $token)> + '_ {
                self.map.iter().map(|(s, &id)| (s.as_str(), id))
            }

            /// Number of distinct strings registered.
            pub fn len(&self) -> usize {
                self.map.len()
            }

            /// Returns true if no strings have been registered yet.
            pub fn is_empty(&self) -> bool {
                self.map.is_empty()
            }
        }
    };
}

// ---------------------------------------------------------------------------
// ItemTypeRegistry — item type strings
// ---------------------------------------------------------------------------

token_registry!(ItemTypeRegistry, ItemTypeId, u32);
token_registry!(SkillRegistry, SkillId, u16);
token_registry!(AmmoTypeRegistry, AmmoTypeId, u16);
token_registry!(BodyPartRegistry, BodyPartId, u16);
token_registry!(ComestibleRegistry, ComestibleId, u16);

// ---------------------------------------------------------------------------
// QualityRegistry — quality-name ↔ QualityId mapping
// ---------------------------------------------------------------------------

/// Bidirectional mapping from quality-name string to `QualityId`.
///
/// Populated during `build_def_world`.  Append-only — once a quality name
/// gets an ID, that ID is stable for the lifetime of the session.
///
/// ## Query patterns
///
/// ```ignore
/// // Filter: find all items with a quality matching "CUT*"
/// let matching: Vec<QualityId> = quality_registry
///     .iter()
///     .filter(|(name, _)| name.starts_with("CUT"))
///     .map(|(_, id)| id)
///     .collect();
///
/// // Then query items with those QualityTokens
/// for (item, qualities) in &item_qualities_query {
///     if qualities.0.iter().any(|(qid, _)| matching.contains(qid)) {
///         // this item has a CUT* quality
///     }
/// }
/// ```
#[derive(Resource, Debug, Clone)]
pub struct QualityRegistry {
    map: BiMap<String, QualityId>,
    next_id: u16,
}

impl Default for QualityRegistry {
    fn default() -> Self {
        Self {
            map: BiMap::new(),
            next_id: 0,
        }
    }
}

impl QualityRegistry {
    /// Get the `QualityId` for a quality name, allocating one if new.
    pub fn intern(&mut self, name: &str) -> QualityId {
        if let Some(&id) = self.map.get_by_left(name) {
            return id;
        }
        let id = QualityId(self.next_id);
        self.next_id += 1;
        self.map.insert(name.to_string(), id);
        id
    }

    /// Look up a quality name without allocating.  Returns `None` if unknown.
    pub fn get(&self, name: &str) -> Option<QualityId> {
        self.map.get_by_left(name).copied()
    }

    /// Look up a quality name by its ID.
    pub fn resolve(&self, id: QualityId) -> Option<&str> {
        self.map.get_by_right(&id).map(|s| s.as_str())
    }

    /// Iterate over all registered (name, QualityId) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&str, QualityId)> + '_ {
        self.map.iter().map(|(s, &id)| (s.as_str(), id))
    }

    /// Number of distinct qualities registered.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Returns true if no qualities have been registered yet.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

// ---------------------------------------------------------------------------
// StringInterner — general-purpose string → u32 mapping
// ---------------------------------------------------------------------------

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
