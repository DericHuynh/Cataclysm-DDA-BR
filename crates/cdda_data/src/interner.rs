//! String interner — stable typed IDs for hot-reloadable JSON data.
//!
//! Uses `BiMap<String, T>` for bidirectional O(1) lookups per token type.
//! Append-only: IDs never change during hot-reload, ensuring runtime
//! entity references remain valid.
//!
//! ## Registry types
//!
//! - `QualityRegistry` — quality-name → QualityToken mapping
//! - `ItemTypeRegistry` — item-type string → ItemTypeToken mapping
//! - Additional registries via the `token_registry!` macro
//!
//! ## Adding a new token type
//!
//! 1. Define the newtype in `cdda_components/src/tokens.rs`:
//!    ```ignore
//!    intern_token!(SkillToken, u16);
//!    ```
//! 2. Create the registry via the macro:
//!    ```ignore
//!    token_registry!(SkillRegistry, SkillToken, u16);
//!    ```
//! 3. Init it in `CddaDataPlugin::build()`.

use bevy_ecs::prelude::*;
use bidimap::BiMap;
use cdda_components::item::QualityToken;
use cdda_components::AmmoTypeToken;
use cdda_components::BodyPartToken;
use cdda_components::ComestibleToken;
use cdda_components::ItemTypeToken;
use cdda_components::SkillToken;

// ---------------------------------------------------------------------------
// Macro: token_registry! — generates a typed string-interning registry
// ---------------------------------------------------------------------------

/// Generates a typed `Resource` registry for interning strings.
///
/// ```ignore
/// token_registry!(SkillRegistry, SkillToken, u16);
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

token_registry!(ItemTypeRegistry, ItemTypeToken, u32);
token_registry!(SkillRegistry, SkillToken, u16);
token_registry!(AmmoTypeRegistry, AmmoTypeToken, u16);
token_registry!(BodyPartRegistry, BodyPartToken, u16);
token_registry!(ComestibleRegistry, ComestibleToken, u16);

// ---------------------------------------------------------------------------
// QualityRegistry — quality-name ↔ QualityToken mapping
// ---------------------------------------------------------------------------

/// Bidirectional mapping from quality-name string to `QualityToken`.
///
/// Populated during `build_def_world`.  Append-only — once a quality name
/// gets an ID, that ID is stable for the lifetime of the session.
///
/// ## Query patterns
///
/// ```ignore
/// // Filter: find all items with a quality matching "CUT*"
/// let matching: Vec<QualityToken> = quality_registry
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
    map: BiMap<String, QualityToken>,
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
    /// Get the `QualityToken` for a quality name, allocating one if new.
    pub fn intern(&mut self, name: &str) -> QualityToken {
        if let Some(&id) = self.map.get_by_left(name) {
            return id;
        }
        let id = QualityToken(self.next_id);
        self.next_id += 1;
        self.map.insert(name.to_string(), id);
        id
    }

    /// Look up a quality name without allocating.  Returns `None` if unknown.
    pub fn get(&self, name: &str) -> Option<QualityToken> {
        self.map.get_by_left(name).copied()
    }

    /// Look up a quality name by its ID.
    pub fn resolve(&self, id: QualityToken) -> Option<&str> {
        self.map.get_by_right(&id).map(|s| s.as_str())
    }

    /// Iterate over all registered (name, QualityId) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&str, QualityToken)> + '_ {
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
