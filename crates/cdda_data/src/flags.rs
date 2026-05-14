//! Per-category bitflag system — FixedBitSet backed, one registry per category.
//!
//! ## Architecture
//!
//! Each CDDA category (item, monster, terrain, …) has:
//! - A **registry** (`*FlagRegistry`) — a `Resource` that maps flag string → bit index.
//! - A **component** (`*Flags`) — a `FixedBitSet` stored on each ECS entity.
//!
//! The `CddaDataPlugin` inserts all registries into the world. Systems that
//! build def entities (e.g. `build_def_world`) read flag strings from the
//! `DefRegistry`, register them via `FlagMap::register_all`, and write the
//! resulting bitset onto the entity's flag component.
//!
//! ## Query patterns
//!
//! ```ignore
//! // Check if an item has a flag
//! fn burning_items(
//!     query: Query<&ItemFlags>,
//!     item_reg: Res<ItemFlagRegistry>,
//! ) {
//!     for flags in &query {
//!         if flags.has(&item_reg, "FLAMING") { /* ... */ }
//!     }
//! }
//!
//! // Archetype filter: only items that have a known flag
//! fn flammable(query: Query<&ItemFlags>, reg: Res<ItemFlagRegistry>) {
//!     let idx = reg.0.idx("FLAMMABLE");
//!     for flags in &query {
//!         if flags.has_idx(idx) { /* guaranteed flammable */ }
//!     }
//! }
//! ```

use bevy_app::{App, Plugin};
use bevy_ecs::prelude::*;
use fixedbitset::FixedBitSet;

use crate::interner::{
    AmmoTypeRegistry, BodyPartRegistry, ComestibleRegistry, ItemTypeRegistry, QualityRegistry,
    SkillRegistry,
};

const MAX_CATEGORY_FLAGS: usize = 4096;

// ---------------------------------------------------------------------------
// FlagMap — bidirectional string ↔ u16 mapping
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct FlagMap {
    map: bidimap::BiMap<String, u16>,
    next_idx: u16,
}

impl Default for FlagMap {
    fn default() -> Self {
        Self {
            map: bidimap::BiMap::new(),
            next_idx: 0,
        }
    }
}

impl FlagMap {
    /// Register a single flag string, returning its index.
    /// Idempotent — returns the existing index if already registered.
    pub fn register(&mut self, flag: &str) -> u16 {
        if let Some(&idx) = self.map.get_by_left(flag) {
            return idx;
        }
        let idx = self.next_idx;
        assert!((idx as usize) < MAX_CATEGORY_FLAGS);
        self.map.insert(flag.to_string(), idx);
        self.next_idx += 1;
        idx
    }

    /// Look up a flag's index. Panics if the flag was never registered.
    #[deprecated(since = "0.2.0", note = "use `try_idx` instead")]
    pub fn idx(&self, flag: &str) -> u16 {
        *self.map.get_by_left(flag).unwrap()
    }

    /// Look up a flag's index, returning `None` if it was never registered.
    pub fn try_idx(&self, flag: &str) -> Option<u16> {
        self.map.get_by_left(flag).copied()
    }

    /// Register every flag in `flags` and return a `FixedBitSet` with all of
    /// them set. This is the one-stop method for converting a list of flag
    /// strings into an entity's flag component.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut reg = ItemFlagRegistry::default();
    /// let bitset = reg.0.register_all(&["FIRE", "WET"]);
    /// commands.spawn(ItemFlags(bitset));
    /// ```
    pub fn register_all(&mut self, flags: &[String]) -> FixedBitSet {
        let mut bs = FixedBitSet::with_capacity(MAX_CATEGORY_FLAGS);
        for flag in flags {
            bs.insert(self.register(flag) as usize);
        }
        bs
    }

    /// Register every flag found in a CDDA JSON value.
    ///
    /// Handles all CDDA flag formats:
    /// - `"flags": ["FIRE", "WET"]` — array of strings
    /// - `"flags": "FIRE"` — single string shorthand
    /// - `"extend": {"flags": [...]}` — mod extension
    /// - `"delete": {"flags": [...]}` — mod deletion
    ///
    /// This mirrors the C++ `auto_flags_reader` from `generic_factory.h`.
    pub fn register_flags_from_json(&mut self, value: &serde_json::Value) {
        fn extract(v: &serde_json::Value, out: &mut FlagMap) {
            match v {
                // Array: ["FIRE", "WET"]
                serde_json::Value::Array(arr) => {
                    for item in arr {
                        if let Some(s) = item.as_str() {
                            out.register(s);
                        }
                    }
                }
                // Single string: "FIRE"
                serde_json::Value::String(s) => {
                    out.register(s);
                }
                _ => {}
            }
        }

        // Top-level "flags" field.
        if let Some(v) = value.get("flags") {
            extract(v, self);
        }
        // Nested inside extend / delete (mod entries).
        for key in &["extend", "delete"] {
            if let Some(obj) = value.get(*key).and_then(|v| v.as_object()) {
                if let Some(v) = obj.get("flags") {
                    extract(v, self);
                }
            }
        }
    }

    /// Iterate over all registered (flag_string, index) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&str, u16)> + '_ {
        self.map.iter().map(|(s, &i)| (s.as_str(), i))
    }

    /// All registered flag strings, sorted by registration order.
    pub fn flags(&self) -> Vec<&str> {
        let mut v: Vec<(&str, u16)> = self.iter().collect();
        v.sort_by_key(|(_, i)| *i);
        v.into_iter().map(|(s, _)| s).collect()
    }

    /// Number of distinct flags registered so far.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Returns true if no flags have been registered yet.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Alias for `register_all` — kept for backward compatibility.
    #[deprecated(since = "0.2.0", note = "use `register_all` instead")]
    pub fn to_bitset(&mut self, flags: &[String]) -> FixedBitSet {
        self.register_all(flags)
    }
}

// ---------------------------------------------------------------------------
// Per-category registries (Bevy Resources)
// ---------------------------------------------------------------------------

macro_rules! flag_registry {
    ($name:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(Resource, Debug, Clone, Default)]
        pub struct $name(pub FlagMap);
    };
}

flag_registry!(ItemFlagRegistry, "Flag registry for item definitions.");
flag_registry!(
    MonsterFlagRegistry,
    "Flag registry for monster definitions."
);
flag_registry!(
    TerrainFlagRegistry,
    "Flag registry for terrain definitions."
);
flag_registry!(
    FurnitureFlagRegistry,
    "Flag registry for furniture definitions."
);
flag_registry!(MeleeFlagRegistry, "Flag registry for melee weapon flags.");
flag_registry!(ArmorFlagRegistry, "Flag registry for armor flags.");
flag_registry!(GunFlagRegistry, "Flag registry for gun flags.");

// ---------------------------------------------------------------------------
// Per-category flag components (Bevy Components)
// ---------------------------------------------------------------------------

macro_rules! flag_comp {
    ($n:ident, $r:ty) => {
        #[derive(Component, Debug, Clone)]
        pub struct $n(pub FixedBitSet);
        impl $n {
            pub fn new() -> Self {
                Self(FixedBitSet::with_capacity(MAX_CATEGORY_FLAGS))
            }
            pub fn has(&self, reg: &$r, flag: &str) -> bool {
                reg.0
                    .map
                    .get_by_left(flag)
                    .map_or(false, |&i| self.0.contains(i as usize))
            }
            pub fn len(&self) -> usize {
                self.0.count_ones(..)
            }
            pub fn is_empty(&self) -> bool {
                self.0.count_ones(..) == 0
            }
            pub fn has_idx(&self, idx: u16) -> bool {
                self.0.contains(idx as usize)
            }
        }
        impl Default for $n {
            fn default() -> Self {
                Self::new()
            }
        }
    };
}

flag_comp!(ItemFlags, ItemFlagRegistry);
flag_comp!(MonsterFlags, MonsterFlagRegistry);
flag_comp!(TerrainFlags, TerrainFlagRegistry);
flag_comp!(FurnitureFlags, FurnitureFlagRegistry);
flag_comp!(MeleeFlags, MeleeFlagRegistry);
flag_comp!(ArmorFlags, ArmorFlagRegistry);
flag_comp!(GunFlags, GunFlagRegistry);

pub type ItemFlagList = ItemFlags;

// ---------------------------------------------------------------------------
// CddaDataPlugin — inserts all registries into the Bevy world
// ---------------------------------------------------------------------------

/// Plugin that inserts all CDDA flag registries into the Bevy world as `Resource`s.
///
/// Add this to your app before any system that builds definition entities:
///
/// ```ignore
/// app.add_plugins(CddaDataPlugin);
/// ```
///
/// After this plugin runs, `build_def_world` (or your own def-building code)
/// can access `ResMut<ItemFlagRegistry>` etc. to populate flag components.
pub struct CddaDataPlugin;

impl Plugin for CddaDataPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ItemFlagRegistry>();
        app.init_resource::<MonsterFlagRegistry>();
        app.init_resource::<TerrainFlagRegistry>();
        app.init_resource::<FurnitureFlagRegistry>();
        app.init_resource::<MeleeFlagRegistry>();
        app.init_resource::<ArmorFlagRegistry>();
        app.init_resource::<GunFlagRegistry>();
        app.init_resource::<QualityRegistry>();
        app.init_resource::<ItemTypeRegistry>();
        app.init_resource::<SkillRegistry>();
        app.init_resource::<AmmoTypeRegistry>();
        app.init_resource::<BodyPartRegistry>();
        app.init_resource::<ComestibleRegistry>();
    }
}
