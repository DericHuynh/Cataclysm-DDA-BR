//! Terrain registry — maps type indices to definition data.
//!
//! Built from `DefRegistry` during data loading. Provides O(1) lookup
//! of terrain properties needed during generation and gameplay.

use bevy_ecs::prelude::*;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// TerrainHandle
// ---------------------------------------------------------------------------

/// Compact terrain reference: type_index in upper 24 bits, rotation in lower 8.
///
/// Supports up to ~16M terrain types and 256 rotation variants.
/// `type_index = 0` is reserved for NULL/unset terrain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
#[repr(transparent)]
pub struct TerrainHandle(pub u32);

impl TerrainHandle {
    /// Null/invalid terrain — never placed during generation.
    pub const NULL: Self = Self(0);

    /// Create from a type index and optional rotation (0..255).
    pub const fn new(type_index: u32, rotation: u8) -> Self {
        debug_assert!(type_index < (1 << 24), "type_index out of range");
        Self((type_index << 8) | rotation as u32)
    }

    /// The base terrain type index.
    #[inline]
    pub const fn type_index(self) -> u32 {
        self.0 >> 8
    }

    /// Rotation variant (0 = north, 1 = east, etc.).
    #[inline]
    pub const fn rotation(self) -> u8 {
        (self.0 & 0xFF) as u8
    }

    /// The base type handle (rotation 0).
    #[inline]
    pub const fn base(self) -> Self {
        Self(self.0 & !0xFF)
    }
}

// ---------------------------------------------------------------------------
// TerrainFlags
// ---------------------------------------------------------------------------

/// Per-terrain-type flags used for spatial queries during generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TerrainFlags(u16);

impl TerrainFlags {
    pub const LINE_DRAWING: u16 = 1 << 0;
    pub const RIVER: u16 = 1 << 1;
    pub const LAKE: u16 = 1 << 2;
    pub const OCEAN: u16 = 1 << 3;
    pub const ROAD: u16 = 1 << 4;
    pub const HIGHWAY: u16 = 1 << 5;
    pub const RAILROAD: u16 = 1 << 6;
    pub const FOREST: u16 = 1 << 7;
    pub const IMPASSABLE: u16 = 1 << 8;
    pub const UNDERGROUND: u16 = 1 << 9;
    pub const BRIDGE: u16 = 1 << 10;
    pub const MANHOLE: u16 = 1 << 11;
    pub const SUBWAY: u16 = 1 << 12;
    pub const SEWER: u16 = 1 << 13;

    pub const fn empty() -> Self { Self(0) }
    pub const fn contains(self, other: u16) -> bool { self.0 & other != 0 }
    pub const fn intersects(self, other: Self) -> bool { self.0 & other.0 != 0 }
    pub fn set(&mut self, flag: u16) { self.0 |= flag; }
}

// ---------------------------------------------------------------------------
// TerrainRegistry
// ---------------------------------------------------------------------------

/// O(1) lookup of terrain properties by type index.
#[derive(Resource, Debug, Clone)]
pub struct TerrainRegistry {
    def_entities: Vec<Option<Entity>>,
    flags: Vec<TerrainFlags>,
    travel_costs: Vec<u8>,
    mapgen_ids: Vec<String>,
    rotated_handles: Vec<[TerrainHandle; 8]>,
    id_to_index: HashMap<String, u32>,

    pub field_index: u32,
    pub forest_index: u32,
    pub forest_thick_index: u32,
    pub forest_water_index: u32,
    pub road_ns_index: u32,
    pub road_ew_index: u32,
    pub road_nesw_index: u32,
    pub lake_surface_index: u32,
    pub lake_shore_index: u32,
    pub ocean_index: u32,
    pub river_center_index: u32,
}

impl TerrainRegistry {
    pub fn empty() -> Self {
        Self {
            def_entities: vec![None; 1],
            flags: vec![TerrainFlags::empty(); 1],
            travel_costs: vec![2; 1],
            mapgen_ids: vec![String::new(); 1],
            rotated_handles: vec![[TerrainHandle::NULL; 8]; 1],
            id_to_index: HashMap::new(),
            field_index: 0, forest_index: 0, forest_thick_index: 0, forest_water_index: 0,
            road_ns_index: 0, road_ew_index: 0, road_nesw_index: 0,
            lake_surface_index: 0, lake_shore_index: 0,
            ocean_index: 0, river_center_index: 0,
        }
    }

    pub fn register(
        &mut self, def_entity: Entity, string_id: &str,
        flags: TerrainFlags, travel_cost: u8, mapgen_id: String,
    ) -> u32 {
        let idx = self.def_entities.len() as u32;
        self.def_entities.push(Some(def_entity));
        self.flags.push(flags);
        self.travel_costs.push(travel_cost);
        self.mapgen_ids.push(mapgen_id);
        let base = TerrainHandle::new(idx, 0);
        let mut rotations = [base; 8];
        for r in 1..8 { rotations[r] = TerrainHandle::new(idx, r as u8); }
        self.rotated_handles.push(rotations);
        self.id_to_index.insert(string_id.to_string(), idx);
        idx
    }

    pub fn register_no_entity(
        &mut self, string_id: &str,
        flags: TerrainFlags, travel_cost: u8, mapgen_id: String,
    ) -> u32 {
        let idx = self.def_entities.len() as u32;
        self.def_entities.push(None);
        self.flags.push(flags);
        self.travel_costs.push(travel_cost);
        self.mapgen_ids.push(mapgen_id);
        let base = TerrainHandle::new(idx, 0);
        let mut rotations = [base; 8];
        for r in 1..8 { rotations[r] = TerrainHandle::new(idx, r as u8); }
        self.rotated_handles.push(rotations);
        self.id_to_index.insert(string_id.to_string(), idx);
        idx
    }

    pub fn register_rotation(&mut self, base_index: u32, rotation: u8, variant_index: u32) {
        if let Some(handles) = self.rotated_handles.get_mut(base_index as usize) {
            handles[rotation as usize] = TerrainHandle::new(variant_index, 0);
        }
    }

    #[inline]
    pub fn flags_for(&self, handle: TerrainHandle) -> TerrainFlags {
        self.flags.get(handle.type_index() as usize).copied().unwrap_or_default()
    }

    #[inline]
    pub fn travel_cost(&self, handle: TerrainHandle) -> u8 {
        self.travel_costs.get(handle.type_index() as usize).copied().unwrap_or(2)
    }

    pub fn mapgen_id(&self, handle: TerrainHandle) -> &str {
        self.mapgen_ids.get(handle.type_index() as usize).map(|s| s.as_str()).unwrap_or("")
    }

    pub fn def_entity(&self, handle: TerrainHandle) -> Option<Entity> {
        self.def_entities.get(handle.type_index() as usize).copied().flatten()
    }

    /// Get the string ID for a handle (reverse lookup of index_by_id).
    pub fn string_id_for(&self, handle: TerrainHandle) -> Option<&str> {
        self.id_to_index.iter().find(|(_, &v)| v == handle.type_index()).map(|(k, _)| k.as_str())
    }

    #[inline]
    pub fn rotate(&self, handle: TerrainHandle, dir: u8) -> TerrainHandle {
        self.rotated_handles.get(handle.type_index() as usize)
            .and_then(|h| h.get(dir as usize)).copied().unwrap_or(handle)
    }

    pub fn index_by_id(&self, id: &str) -> Option<u32> {
        self.id_to_index.get(id).copied()
    }

    pub fn handle_by_id(&self, id: &str) -> Option<TerrainHandle> {
        self.id_to_index.get(id).map(|&idx| TerrainHandle::new(idx, 0))
    }

    pub fn len(&self) -> usize {
        self.def_entities.len()
    }
}
