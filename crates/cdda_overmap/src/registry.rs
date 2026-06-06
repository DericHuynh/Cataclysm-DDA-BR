//! Terrain registry — O(1) lookup of terrain properties by handle.
//!
//! # Design
//!
//! `TerrainRegistry` is a flat SoA (struct-of-arrays) lookup table keyed by
//! `type_index`. This is intentional: mapgen and gameplay read millions of
//! handle properties per frame (flags, travel cost, family), and a single
//! indexed array access is faster than the ECS archetype machinery for this
//! access pattern. Terrain definitions that need general-purpose component
//! queries can still be ECS entities; the registry just holds the hot-path
//! properties separately.
//!
//! # String matching
//!
//! `is_ot_match` string matching (prefix, contains, type) must only run
//! during asset loading, never in mapgen or gameplay loops. At registration
//! time, assign a `family_id` (shared by all variants of the same terrain
//! family) and set the appropriate `TerrainFlags`. Runtime checks should
//! compare flags or family IDs, not strings.
//!
//! # Game-specific terrain handles
//!
//! Hardcoded indices for specific CDDA terrains (`field`, `forest`, `road`,
//! etc.) live in `CoreTerrains`, a separate resource populated during data
//! loading. They are NOT fields on `TerrainRegistry` itself, which remains
//! a generic lookup table.

use bevy_ecs::prelude::*;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// TerrainHandle
// ---------------------------------------------------------------------------

/// Compact terrain reference: type_index in upper 24 bits, rotation in lower 8.
///
/// `type_index = 0` is reserved for NULL/unset terrain.
/// Supports up to ~16 M terrain types and 256 rotation variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
#[repr(transparent)]
pub struct TerrainHandle(pub u32);

impl TerrainHandle {
    /// Null/invalid terrain — never placed during generation.
    pub const NULL: Self = Self(0);

    /// Create from a type index and rotation (0..255).
    #[inline]
    pub const fn new(type_index: u32, rotation: u8) -> Self {
        debug_assert!(type_index < (1 << 24), "type_index out of range");
        Self((type_index << 8) | rotation as u32)
    }

    /// The base terrain type index (rotation-independent).
    #[inline]
    pub const fn type_index(self) -> u32 {
        self.0 >> 8
    }

    /// Rotation variant (0 = north / default, 1 = east, 2 = south, 3 = west).
    #[inline]
    pub const fn rotation(self) -> u8 {
        (self.0 & 0xFF) as u8
    }

    /// The base-rotation handle (rotation 0) for this type.
    #[inline]
    pub const fn base(self) -> Self {
        Self(self.0 & !0xFF)
    }

    /// Returns `true` for the reserved null sentinel.
    #[inline]
    pub const fn is_null(self) -> bool {
        self.0 == 0
    }
}

// ---------------------------------------------------------------------------
// TerrainFlags
// ---------------------------------------------------------------------------

/// Per-terrain-type flags used for spatial queries and mapgen decisions.
///
/// Assign at registration time from string-parsed data. Runtime code checks
/// flags only — never string-matches at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TerrainFlags(pub u16);

impl TerrainFlags {
    pub const LINE_DRAWING: u16 = 1 << 0;
    pub const RIVER:        u16 = 1 << 1;
    pub const LAKE:         u16 = 1 << 2;
    pub const OCEAN:        u16 = 1 << 3;
    pub const ROAD:         u16 = 1 << 4;
    pub const HIGHWAY:      u16 = 1 << 5;
    pub const RAILROAD:     u16 = 1 << 6;
    pub const FOREST:       u16 = 1 << 7;
    pub const IMPASSABLE:   u16 = 1 << 8;
    pub const UNDERGROUND:  u16 = 1 << 9;
    pub const BRIDGE:       u16 = 1 << 10;
    pub const MANHOLE:      u16 = 1 << 11;
    pub const SUBWAY:       u16 = 1 << 12;
    pub const SEWER:        u16 = 1 << 13;

    #[inline] pub const fn empty() -> Self { Self(0) }
    #[inline] pub const fn from_bits(bits: u16) -> Self { Self(bits) }
    #[inline] pub const fn contains(self, flag: u16) -> bool { self.0 & flag != 0 }
    #[inline] pub const fn intersects(self, other: Self) -> bool { self.0 & other.0 != 0 }
    #[inline] pub fn set(&mut self, flag: u16) { self.0 |= flag; }
    #[inline] pub fn clear(&mut self, flag: u16) { self.0 &= !flag; }
}

// ---------------------------------------------------------------------------
// TerrainRegistry
// ---------------------------------------------------------------------------

/// O(1) lookup of terrain properties by type index.
///
/// This is a SoA table, not an ECS registry. Each `Vec` is indexed by
/// `TerrainHandle::type_index()`. Index 0 is the null sentinel.
///
/// For game-specific handles (field, forest, road, etc.) use `CoreTerrains`.
#[derive(Resource, Debug, Clone)]
pub struct TerrainRegistry {
    // Per-type data — all Vecs must stay the same length.
    def_entities:    Vec<Option<Entity>>,
    flags:           Vec<TerrainFlags>,
    travel_costs:    Vec<u8>,
    mapgen_ids:      Vec<String>,
    /// Family ID shared by all variants/rotations of the same terrain family
    /// (e.g. every `road_*` variant shares one family_id). Used for O(1)
    /// prefix/type matching without string operations at runtime.
    family_ids:      Vec<u32>,
    rotated_handles: Vec<[TerrainHandle; 8]>,
    id_to_index:     HashMap<String, u32>,
    /// Maps a family string (e.g. `"road"`, `"sub_station"`) to a numeric ID.
    /// Populated at registration time; never touched at runtime.
    family_name_to_id: HashMap<String, u32>,
    next_family_id:  u32,
}

impl TerrainRegistry {
    /// Create an empty registry with slot 0 reserved as the null sentinel.
    pub fn empty() -> Self {
        Self {
            def_entities:      vec![None],
            flags:             vec![TerrainFlags::empty()],
            travel_costs:      vec![0],
            mapgen_ids:        vec![String::new()],
            family_ids:        vec![0],
            rotated_handles:   vec![[TerrainHandle::NULL; 8]],
            id_to_index:       HashMap::new(),
            family_name_to_id: HashMap::new(),
            next_family_id:    1, // 0 reserved for "no family"
        }
    }

    // -- Registration ----------------------------------------------------------

    /// Resolve or create a numeric family ID for a family name string.
    ///
    /// Call this during asset loading when you know which family a terrain
    /// belongs to (e.g. all `road_*` terrains share family `"road"`).
    pub fn get_or_create_family(&mut self, family_name: &str) -> u32 {
        if let Some(&id) = self.family_name_to_id.get(family_name) {
            return id;
        }
        let id = self.next_family_id;
        self.next_family_id += 1;
        self.family_name_to_id.insert(family_name.to_string(), id);
        id
    }

    /// Register a terrain type backed by an existing ECS definition entity.
    ///
    /// `family_id` should be obtained from `get_or_create_family`. Pass `0`
    /// if this terrain has no family (it will never match family queries).
    pub fn register(
        &mut self,
        def_entity: Entity,
        string_id: &str,
        flags: TerrainFlags,
        travel_cost: u8,
        mapgen_id: String,
        family_id: u32,
    ) -> u32 {
        let idx = self.push_slot(None, flags, travel_cost, mapgen_id, family_id);
        self.def_entities[idx as usize] = Some(def_entity);
        self.id_to_index.insert(string_id.to_string(), idx);
        idx
    }

    /// Register a terrain type with no backing ECS entity.
    pub fn register_no_entity(
        &mut self,
        string_id: &str,
        flags: TerrainFlags,
        travel_cost: u8,
        mapgen_id: String,
        family_id: u32,
    ) -> u32 {
        let idx = self.push_slot(None, flags, travel_cost, mapgen_id, family_id);
        self.id_to_index.insert(string_id.to_string(), idx);
        idx
    }

    fn push_slot(
        &mut self,
        def_entity: Option<Entity>,
        flags: TerrainFlags,
        travel_cost: u8,
        mapgen_id: String,
        family_id: u32,
    ) -> u32 {
        let idx = self.def_entities.len() as u32;
        self.def_entities.push(def_entity);
        self.flags.push(flags);
        self.travel_costs.push(travel_cost);
        self.mapgen_ids.push(mapgen_id);
        self.family_ids.push(family_id);
        let base = TerrainHandle::new(idx, 0);
        let mut rotations = [base; 8];
        for r in 1..8u8 {
            rotations[r as usize] = TerrainHandle::new(idx, r);
        }
        self.rotated_handles.push(rotations);
        idx
    }

    /// Override the handle stored for a specific rotation slot of a base type.
    ///
    /// Used when a rotation variant is registered as its own separate type
    /// (e.g. `road_ns` and `road_ew` as independent entries) and must be
    /// addressable via `rotate(base_handle, dir)`.
    pub fn register_rotation(&mut self, base_index: u32, rotation: u8, variant_index: u32) {
        if let Some(handles) = self.rotated_handles.get_mut(base_index as usize) {
            handles[rotation as usize] = TerrainHandle::new(variant_index, 0);
        }
    }

    // -- Hot-path lookups (called millions of times during mapgen/gameplay) ----

    /// Flags for a terrain handle. O(1) indexed read.
    #[inline]
    pub fn flags_for(&self, handle: TerrainHandle) -> TerrainFlags {
        self.flags
            .get(handle.type_index() as usize)
            .copied()
            .unwrap_or_default()
    }

    /// Travel cost for a terrain handle. O(1) indexed read.
    #[inline]
    pub fn travel_cost(&self, handle: TerrainHandle) -> u8 {
        self.travel_costs
            .get(handle.type_index() as usize)
            .copied()
            .unwrap_or(2)
    }

    /// Family ID for a terrain handle. O(1) indexed read.
    ///
    /// Returns `0` if no family is assigned. Use this instead of string
    /// prefix/type matching in any hot loop.
    #[inline]
    pub fn family_id(&self, handle: TerrainHandle) -> u32 {
        self.family_ids
            .get(handle.type_index() as usize)
            .copied()
            .unwrap_or(0)
    }

    /// Check if two handles belong to the same terrain family.
    ///
    /// Returns `false` if either handle has family_id 0 (no family).
    #[inline]
    pub fn same_family(&self, a: TerrainHandle, b: TerrainHandle) -> bool {
        let fa = self.family_id(a);
        fa != 0 && fa == self.family_id(b)
    }

    /// Check if a handle belongs to a named family. Slightly more expensive
    /// than `same_family` due to the HashMap lookup on `family_name`.
    ///
    /// Prefer pre-resolving the family ID with `family_id_by_name` and
    /// comparing integers if calling in a loop.
    #[inline]
    pub fn is_family(&self, handle: TerrainHandle, family_name: &str) -> bool {
        let Some(&target_id) = self.family_name_to_id.get(family_name) else {
            return false;
        };
        self.family_id(handle) == target_id
    }

    /// Resolve a family name to its numeric ID.
    ///
    /// Returns `None` if the family was never registered. Cache the result
    /// and compare integers in hot loops instead of calling `is_family`.
    #[inline]
    pub fn family_id_by_name(&self, family_name: &str) -> Option<u32> {
        self.family_name_to_id.get(family_name).copied()
    }

    // -- Cold-path lookups (asset loading, debugging, serialization) -----------

    pub fn mapgen_id(&self, handle: TerrainHandle) -> &str {
        self.mapgen_ids
            .get(handle.type_index() as usize)
            .map(|s| s.as_str())
            .unwrap_or("")
    }

    pub fn def_entity(&self, handle: TerrainHandle) -> Option<Entity> {
        self.def_entities
            .get(handle.type_index() as usize)
            .copied()
            .flatten()
    }

    /// Reverse lookup: string ID for a handle. O(N) scan — do not call in
    /// hot loops. For debugging and cold paths only.
    pub fn string_id_for(&self, handle: TerrainHandle) -> Option<&str> {
        self.id_to_index
            .iter()
            .find(|(_, &v)| v == handle.type_index())
            .map(|(k, _)| k.as_str())
    }

    /// Get a rotated handle variant. O(1) indexed read.
    #[inline]
    pub fn rotate(&self, handle: TerrainHandle, dir: u8) -> TerrainHandle {
        self.rotated_handles
            .get(handle.type_index() as usize)
            .and_then(|h| h.get(dir as usize))
            .copied()
            .unwrap_or(handle)
    }

    pub fn index_by_id(&self, id: &str) -> Option<u32> {
        self.id_to_index.get(id).copied()
    }

    pub fn handle_by_id(&self, id: &str) -> Option<TerrainHandle> {
        self.id_to_index
            .get(id)
            .map(|&idx| TerrainHandle::new(idx, 0))
    }

    pub fn len(&self) -> usize {
        self.def_entities.len()
    }

    pub fn is_empty(&self) -> bool {
        self.def_entities.len() <= 1
    }
}

// ---------------------------------------------------------------------------
// CoreTerrains — game-specific handles, separate from the generic registry
// ---------------------------------------------------------------------------

/// Pre-resolved handles for CDDA's core terrain types.
///
/// Populated during data loading after all terrain definitions are registered.
/// Systems that need to place or test for specific terrain types use this
/// resource rather than going through `registry.handle_by_id()` at runtime
/// or storing raw indices on `TerrainRegistry`.
///
/// All fields default to `TerrainHandle::NULL` until populated by the loader.
#[derive(Resource, Debug, Clone, Default)]
pub struct CoreTerrains {
    pub field:         TerrainHandle,
    pub forest:        TerrainHandle,
    pub forest_thick:  TerrainHandle,
    pub forest_water:  TerrainHandle,
    pub road_ns:       TerrainHandle,
    pub road_ew:       TerrainHandle,
    pub road_nesw:     TerrainHandle,
    pub lake_surface:  TerrainHandle,
    pub lake_shore:    TerrainHandle,
    pub ocean:         TerrainHandle,
    pub river_center:  TerrainHandle,
}

impl CoreTerrains {
    /// Populate from a fully-loaded `TerrainRegistry`.
    ///
    /// Logs warnings for any ID that is not found in the registry so that
    /// missing data definitions are caught at startup, not at runtime.
    pub fn from_registry(registry: &TerrainRegistry) -> Self {
        let resolve = |id: &str| -> TerrainHandle {
            registry.handle_by_id(id).unwrap_or_else(|| {
                // In a real build this would use bevy's warn! macro.
                eprintln!("CoreTerrains: terrain ID '{}' not found in registry", id);
                TerrainHandle::NULL
            })
        };

        Self {
            field:        resolve("field"),
            forest:       resolve("forest"),
            forest_thick: resolve("forest_thick"),
            forest_water: resolve("forest_water"),
            road_ns:      resolve("road_ns"),
            road_ew:      resolve("road_ew"),
            road_nesw:    resolve("road_nesw"),
            lake_surface: resolve("lake_surface"),
            lake_shore:   resolve("lake_shore"),
            ocean:        resolve("ocean"),
            river_center: resolve("river_center"),
        }
    }
}
