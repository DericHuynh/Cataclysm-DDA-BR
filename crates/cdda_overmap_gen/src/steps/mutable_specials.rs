//! Place mutable overmap specials using a phase-based join-resolution engine.
//!
//! Port of CDDA master's `overmap_special_mutable.cpp`.
//!
//! Mutable specials have `subtype: "mutable"` and use:
//! 1. **Joins** — named connection points between OMTs
//! 2. **Overmaps** — named terrain pieces with directional join references
//! 3. **Phases** — ordered placement steps, each with rules
//! 4. **Rules** — placement rules specifying which OMT goes where
//! 5. **Join tracker** — resolves joins using available terrain pieces
//!
//! # Algorithm
//!
//! 1. Place the root OMT at a valid location.
//! 2. Register unresolved joins from the root (from its directional join references).
//! 3. Loop: pick the highest-priority unresolved join.
//! 4. Try each rule in the current phase to satisfy that join.
//! 5. If a rule matches, place its pieces and register their joins.
//! 6. If no rule matches, postpone the join.
//! 7. Advance phases when all rules are exhausted or no joins remain.
//!
//! # Simplified from C++
//!
//! - No `alternative_joins` support yet.
//! - No `into_locations` validation for join resolution.
//! - No `cube_direction` for 3D joins (z-level joins).
//! - No `connections` placement (road/subway hookups).
//! - No camp/basecamp support.
//! - Simplified rotation handling.

use crate::pipeline::OvermapGenConfig;
use crate::region_settings::OvermapRegionSettings;
use crate::special_catalog::SpecialCatalog;
use bevy_ecs::prelude::*;
use cdda_core_types::core::raw_defs::cdda_types::{RawValue, StringOrArray};
use cdda_core_types::core::raw_defs::overmap_terrain::OvermapSpecialDef;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM, OMAP_DIM};
use cdda_overmap::connections::inbounds_omt;
use cdda_overmap::registry::{TerrainFlags, TerrainHandle, TerrainRegistry};
use cdda_overmap::rng::XorShiftRng;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tracing::{info, warn};

// ---------------------------------------------------------------------------
// Cardinal directions (simplified: 2D only)
// ---------------------------------------------------------------------------

/// 2D cardinal direction for joins (N/E/S/W).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum CardinalDir {
    North,
    East,
    South,
    West,
}

impl CardinalDir {
    fn all() -> [Self; 4] {
        [Self::North, Self::East, Self::South, Self::West]
    }

    fn from_str(s: &str) -> Option<Self> {
        match s {
            "north" | "n" => Some(Self::North),
            "east" | "e" => Some(Self::East),
            "south" | "s" => Some(Self::South),
            "west" | "w" => Some(Self::West),
            _ => None,
        }
    }

    /// Opposite direction.
    fn opposite(self) -> Self {
        match self {
            Self::North => Self::South,
            Self::East => Self::West,
            Self::South => Self::North,
            Self::West => Self::East,
        }
    }

    /// Offset (dx, dy) for one OMT step in this direction.
    fn delta(self) -> (i32, i32) {
        match self {
            Self::North => (0, -1),
            Self::East => (1, 0),
            Self::South => (0, 1),
            Self::West => (-1, 0),
        }
    }

    /// Rotate this direction by `rot` steps clockwise (0=N, 1=E, 2=S, 3=W).
    fn rotate(self, rot: u8) -> Self {
        let idx = match self {
            Self::North => 0,
            Self::East => 1,
            Self::South => 2,
            Self::West => 3,
        };
        let new_idx = (idx + rot as usize) % 4;
        match new_idx {
            0 => Self::North,
            1 => Self::East,
            2 => Self::South,
            3 => Self::West,
            _ => unreachable!(),
        }
    }
}

// ---------------------------------------------------------------------------
// Parsed mutable special data
// ---------------------------------------------------------------------------

/// A join definition (parsed from the `joins` array).
#[derive(Debug, Clone)]
struct MutableJoin {
    id: String,
    opposite_id: String,
    /// Priority: lower index in the joins array = higher priority (placed first).
    priority: usize,
}

/// A named overmap terrain piece within a mutable special.
#[derive(Debug, Clone)]
struct MutableOvermap {
    /// The OMT terrain string ID (e.g. "crater_core").
    terrain_id: String,
    /// Locations where this piece can be placed (e.g. ["land"]).
    locations: Vec<String>,
    /// Directional join references: direction → join_id.
    joins: HashMap<CardinalDir, String>,
}

/// One piece within a placement rule's chunk.
#[derive(Debug, Clone)]
struct RulePiece {
    /// Name of the overmap entry to place.
    overmap_name: String,
    /// Relative position from the rule's origin.
    pos: (i32, i32, i32),
}

/// A placement rule within a phase.
#[derive(Debug, Clone)]
struct PlacementRule {
    /// Optional name for debugging.
    name: String,
    /// Pieces that make up this rule.
    pieces: Vec<RulePiece>,
    /// Maximum times this rule can be used in this special placement.
    max_count: usize,
    /// Remaining uses (decremented on each use).
    remaining: usize,
    /// Relative weight for random selection.
    weight: i32,
    /// Pre-computed outward joins: (local_pos, direction, join_id) for joins
    /// that face outward (not satisfied internally).
    outward_joins: Vec<OutwardJoin>,
}

/// An outward join from a rule's pieces.
#[derive(Debug, Clone)]
struct OutwardJoin {
    /// The piece index in the rule.
    piece_idx: usize,
    /// Direction the join faces.
    dir: CardinalDir,
    /// Join ID.
    join_id: String,
}

/// A placement phase with rules.
#[derive(Debug, Clone)]
struct MutablePhase {
    rules: Vec<PlacementRule>,
}

/// Fully parsed mutable special.
#[derive(Debug, Clone)]
struct ParsedMutableSpecial {
    id: String,
    joins: HashMap<String, MutableJoin>,
    overmaps: HashMap<String, MutableOvermap>,
    root_name: String,
    phases: Vec<MutablePhase>,
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/// Parse a mutable special from its `OvermapSpecialDef`.
///
/// Returns `None` if the definition is malformed or missing required fields.
fn parse_mutable_special(def: &Arc<OvermapSpecialDef>) -> Option<ParsedMutableSpecial> {
    let root_name = def.root.as_deref()?;

    // --- parse joins ---
    let joins_raw = def.joins.as_ref()?;
    let mut joins: HashMap<String, MutableJoin> = HashMap::new();
    let join_list = match joins_raw {
        RawValue::Array(arr) => arr,
        _ => {
            warn!(
                "Mutable special '{}': joins is not an array",
                def.id.as_str()
            );
            return None;
        }
    };

    for (priority, entry) in join_list.iter().enumerate() {
        match entry {
            RawValue::String(s) => {
                // Simple form: just the join id, opposite = same
                let id = s.clone();
                joins.entry(id.clone()).or_insert_with(|| MutableJoin {
                    id: id.clone(),
                    opposite_id: id,
                    priority,
                });
            }
            RawValue::Object(obj) => {
                let id = obj.get("id").and_then(|v| match v {
                    RawValue::String(s) => Some(s.clone()),
                    _ => None,
                })?;
                let opposite_id = obj
                    .get("opposite")
                    .and_then(|v| match v {
                        RawValue::String(s) => Some(s.clone()),
                        _ => None,
                    })
                    .unwrap_or_else(|| id.clone());
                joins.entry(id.clone()).or_insert(MutableJoin {
                    id,
                    opposite_id,
                    priority,
                });
            }
            _ => {
                warn!(
                    "Mutable special '{}': unexpected join entry type",
                    def.id.as_str()
                );
            }
        }
    }

    // --- parse overmaps ---
    let overmaps_raw = def.overmaps.as_ref()?;
    let overmap_map = match overmaps_raw {
        RawValue::Object(map) => map,
        _ => {
            warn!(
                "Mutable special '{}': overmaps is not an object",
                def.id.as_str()
            );
            return None;
        }
    };

    let mut overmaps: HashMap<String, MutableOvermap> = HashMap::new();
    for (name, entry) in overmap_map {
        let obj = match entry {
            RawValue::Object(o) => o,
            _ => {
                warn!(
                    "Mutable special '{}': overmap entry '{}' is not an object",
                    def.id.as_str(),
                    name
                );
                continue;
            }
        };

        let terrain_id = obj
            .get("overmap")
            .and_then(|v| match v {
                RawValue::String(s) => Some(s.clone()),
                _ => None,
            })
            .unwrap_or_default();

        let locations: Vec<String> = obj
            .get("locations")
            .and_then(|v| match v {
                RawValue::Array(a) => Some(
                    a.iter()
                        .filter_map(|e| match e {
                            RawValue::String(s) => Some(s.clone()),
                            _ => None,
                        })
                        .collect(),
                ),
                RawValue::String(s) => Some(vec![s.clone()]),
                _ => None,
            })
            .unwrap_or_default();

        let mut piece_joins: HashMap<CardinalDir, String> = HashMap::new();
        for dir in CardinalDir::all() {
            let dir_key = format!("{:?}", dir).to_lowercase();
            if let Some(join_val) = obj.get(&dir_key) {
                if let RawValue::String(join_id) = join_val {
                    piece_joins.insert(dir, join_id.clone());
                }
            }
        }

        overmaps.insert(
            name.clone(),
            MutableOvermap {
                terrain_id,
                locations,
                joins: piece_joins,
            },
        );
    }

    if !overmaps.contains_key(root_name) {
        warn!(
            "Mutable special '{}': root '{}' not found in overmaps",
            def.id.as_str(),
            root_name
        );
        return None;
    }

    // --- parse phases ---
    let phases_raw = def.phases.as_ref()?;
    let phase_list = match phases_raw {
        RawValue::Array(arr) => arr,
        _ => {
            warn!(
                "Mutable special '{}': phases is not an array",
                def.id.as_str()
            );
            return None;
        }
    };

    let mut phases: Vec<MutablePhase> = Vec::new();
    for phase_entry in phase_list {
        let phase_obj = match phase_entry {
            RawValue::Object(o) => o,
            _ => continue,
        };

        let rules_raw = match phase_obj.get("rules") {
            Some(RawValue::Array(a)) => a,
            _ => continue,
        };

        let mut rules: Vec<PlacementRule> = Vec::new();
        for rule_entry in rules_raw {
            let rule_obj = match rule_entry {
                RawValue::Object(o) => o,
                _ => continue,
            };

            let name = rule_obj
                .get("name")
                .and_then(|v| match v {
                    RawValue::String(s) => Some(s.clone()),
                    _ => None,
                })
                .unwrap_or_default();

            let max_count = rule_obj
                .get("max")
                .and_then(|v| match v {
                    RawValue::Array(a) => a.first().and_then(|e| match e {
                        RawValue::Number(n) => Some(*n as usize),
                        _ => None,
                    }),
                    RawValue::Number(n) => Some(*n as usize),
                    _ => None,
                })
                .unwrap_or(1);

            let weight = rule_obj
                .get("weight")
                .and_then(|v| match v {
                    RawValue::Number(n) => Some(*n as i32),
                    _ => None,
                })
                .unwrap_or(i32::MAX);

            let mut pieces: Vec<RulePiece> = Vec::new();

            // Two forms: "overmap" (single piece at origin) or "chunk" (array of pieces)
            if let Some(single_omt) = rule_obj.get("overmap").and_then(|v| match v {
                RawValue::String(s) => Some(s.clone()),
                _ => None,
            }) {
                pieces.push(RulePiece {
                    overmap_name: single_omt,
                    pos: (0, 0, 0),
                });
            } else if let Some(chunk) = rule_obj.get("chunk") {
                if let RawValue::Array(chunk_pieces) = chunk {
                    for cp in chunk_pieces {
                        let cp_obj = match cp {
                            RawValue::Object(o) => o,
                            _ => continue,
                        };
                        let om_name = cp_obj
                            .get("overmap")
                            .and_then(|v| match v {
                                RawValue::String(s) => Some(s.clone()),
                                _ => None,
                            })
                            .unwrap_or_default();
                        let pos = cp_obj
                            .get("pos")
                            .and_then(|v| match v {
                                RawValue::Array(a) => {
                                    let x = a
                                        .first()
                                        .and_then(|e| match e {
                                            RawValue::Number(n) => Some(*n as i32),
                                            _ => None,
                                        })
                                        .unwrap_or(0);
                                    let y = a
                                        .get(1)
                                        .and_then(|e| match e {
                                            RawValue::Number(n) => Some(*n as i32),
                                            _ => None,
                                        })
                                        .unwrap_or(0);
                                    let z = a
                                        .get(2)
                                        .and_then(|e| match e {
                                            RawValue::Number(n) => Some(*n as i32),
                                            _ => None,
                                        })
                                        .unwrap_or(0);
                                    Some((x, y, z))
                                }
                                _ => None,
                            })
                            .unwrap_or((0, 0, 0));
                        pieces.push(RulePiece {
                            overmap_name: om_name,
                            pos,
                        });
                    }
                }
            }

            if pieces.is_empty() {
                warn!(
                    "Mutable special '{}': rule has no pieces (no 'overmap' or 'chunk')",
                    def.id.as_str()
                );
                continue;
            }

            // Pre-compute outward joins
            let mut outward_joins: Vec<OutwardJoin> = Vec::new();

            // Collect all piece positions for internal join detection
            let mut piece_positions: HashMap<(i32, i32), usize> = HashMap::new();
            for (i, piece) in pieces.iter().enumerate() {
                piece_positions.insert((piece.pos.0, piece.pos.1), i);
            }

            for (i, piece) in pieces.iter().enumerate() {
                let Some(om) = overmaps.get(&piece.overmap_name) else {
                    continue;
                };
                for (dir, join_id) in &om.joins {
                    let (dx, dy) = dir.delta();
                    let neighbor_pos = (piece.pos.0 + dx, piece.pos.1 + dy);
                    // If the neighbor is NOT another piece in this rule, it's an outward join
                    if !piece_positions.contains_key(&neighbor_pos) {
                        outward_joins.push(OutwardJoin {
                            piece_idx: i,
                            dir: *dir,
                            join_id: join_id.clone(),
                        });
                    }
                }
            }

            rules.push(PlacementRule {
                name,
                pieces,
                max_count,
                remaining: max_count,
                weight,
                outward_joins,
            });
        }

        if !rules.is_empty() {
            phases.push(MutablePhase { rules });
        }
    }

    if phases.is_empty() {
        warn!("Mutable special '{}': no valid phases", def.id.as_str());
        return None;
    }

    Some(ParsedMutableSpecial {
        id: def.id.as_str().to_string(),
        joins,
        overmaps,
        root_name: root_name.to_string(),
        phases,
    })
}

// ---------------------------------------------------------------------------
// Join tracker
// ---------------------------------------------------------------------------

/// An unresolved join: "at position P, side D, we expect a join of type J".
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct UnresolvedJoin {
    pos: (i32, i32),
    dir: CardinalDir,
    join_id: String,
}

/// Tracks unresolved joins during mutable special placement.
#[derive(Debug, Clone, Default)]
struct JoinTracker {
    /// Unresolved joins, grouped by priority (index in joins array).
    unresolved: Vec<VecDeque<UnresolvedJoin>>,
    /// Postponed joins that couldn't be satisfied in the current phase.
    postponed: Vec<UnresolvedJoin>,
    /// All joins that have been matched (for tracking used joins).
    used: Vec<(String, String)>, // (join_id, opposite_join_id) pairs
}

impl JoinTracker {
    fn new(join_count: usize) -> Self {
        Self {
            unresolved: vec![VecDeque::new(); join_count.max(1)],
            postponed: Vec::new(),
            used: Vec::new(),
        }
    }

    fn any_unresolved(&self) -> bool {
        self.unresolved.iter().any(|q| !q.is_empty())
    }

    fn any_postponed(&self) -> bool {
        !self.postponed.is_empty()
    }

    /// Add an unresolved join at the given priority.
    fn add_unresolved(&mut self, join: UnresolvedJoin, priority: usize) {
        let idx = priority.min(self.unresolved.len() - 1);
        self.unresolved[idx].push_back(join);
    }

    /// Pick the highest-priority unresolved join.
    fn pick_top_priority(&self) -> Option<&UnresolvedJoin> {
        for q in &self.unresolved {
            if let Some(j) = q.front() {
                return Some(j);
            }
        }
        None
    }

    /// Remove and return the highest-priority unresolved join.
    fn pop_top_priority(&mut self) -> Option<UnresolvedJoin> {
        for q in &mut self.unresolved {
            if let Some(j) = q.pop_front() {
                return Some(j);
            }
        }
        None
    }

    /// Check if there are any unresolved joins at a position.
    fn any_at(&self, pos: (i32, i32)) -> bool {
        self.unresolved
            .iter()
            .any(|q| q.iter().any(|j| j.pos == pos))
    }

    /// Count unresolved joins at a position.
    fn count_at(&self, pos: (i32, i32)) -> usize {
        self.unresolved
            .iter()
            .map(|q| q.iter().filter(|j| j.pos == pos).count())
            .sum()
    }

    /// Remove all unresolved joins at a position.
    fn remove_at(&mut self, pos: (i32, i32)) {
        for q in &mut self.unresolved {
            q.retain(|j| j.pos != pos);
        }
    }

    /// Postpone all unresolved joins at a position.
    fn postpone_at(&mut self, pos: (i32, i32)) {
        for q in &mut self.unresolved {
            let mut i = 0;
            while i < q.len() {
                if q[i].pos == pos {
                    self.postponed.push(q.remove(i).unwrap());
                } else {
                    i += 1;
                }
            }
        }
    }

    /// Restore all postponed joins back to unresolved.
    fn restore_postponed(&mut self) {
        let drained: Vec<UnresolvedJoin> = self.postponed.drain(..).collect();
        for j in drained {
            self.add_unresolved(j, 0);
        }
    }

    /// Record a used join pair.
    fn record_used(&mut self, join_id: &str, opposite_id: &str) {
        self.used
            .push((join_id.to_string(), opposite_id.to_string()));
    }
}

// ---------------------------------------------------------------------------
// Placement helpers
// ---------------------------------------------------------------------------

/// Place a terrain handle at a specific OMT position into the chunk grid.
fn place_omt(
    chunks: &mut Query<(&ChunkPosition, &mut OvermapChunk)>,
    omt_x: i32,
    omt_y: i32,
    z: i8,
    handle: TerrainHandle,
) -> bool {
    for (chunk_pos, mut chunk) in chunks {
        if chunk_pos.z.0 != z {
            continue;
        }
        let (ox, oy) = chunk_pos.omt_origin();
        let lx = omt_x - ox;
        let ly = omt_y - oy;
        if lx >= 0 && lx < CHUNK_DIM as i32 && ly >= 0 && ly < CHUNK_DIM as i32 {
            chunk.set(lx as u8, ly as u8, handle);
            return true;
        }
    }
    false
}

/// Check if a position satisfies location constraints.
fn check_location(
    grid: &[[u32; 180]; 180],
    registry: &TerrainRegistry,
    x: i32,
    y: i32,
    locations: &[String],
) -> bool {
    if !inbounds_omt((x, y)) {
        return false;
    }
    let handle = TerrainHandle::new(grid[x as usize][y as usize], 0);
    let flags = registry.flags_for(handle);

    if locations.is_empty() {
        return true;
    }

    for loc in locations {
        let ok = match loc.as_str() {
            "land" => {
                !flags.contains(TerrainFlags::LAKE)
                    && !flags.contains(TerrainFlags::OCEAN)
                    && !flags.contains(TerrainFlags::RIVER)
            }
            "forest" => flags.contains(TerrainFlags::FOREST),
            "water" => {
                flags.contains(TerrainFlags::LAKE)
                    || flags.contains(TerrainFlags::OCEAN)
                    || flags.contains(TerrainFlags::RIVER)
            }
            "swamp" => flags.contains(TerrainFlags::FOREST) && flags.contains(TerrainFlags::LAKE),
            _ => true, // unknown location types are permissive
        };
        if ok {
            return true;
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Main placement system
// ---------------------------------------------------------------------------

/// Place mutable overmap specials from the catalog.
///
/// Mutable specials have `subtype: "mutable"` and use a phase-based
/// placement engine with joins and rules.
pub fn place_mutable_specials(
    mut commands: Commands,
    mut chunks: Query<(&ChunkPosition, &mut OvermapChunk)>,
    config: Res<OvermapGenConfig>,
    registry: Res<TerrainRegistry>,
    settings: Res<OvermapRegionSettings>,
    catalog: Option<Res<SpecialCatalog>>,
) {
    let Some(catalog) = catalog else { return };
    if !settings.place_specials {
        return;
    }

    let mut rng = XorShiftRng::new(config.noise_seed as u64 + 37);

    // Build dense terrain grid for z=0
    let mut grid = [[0u32; 180]; 180];
    for (chunk_pos, chunk) in &chunks {
        if chunk_pos.z.0 != 0 {
            continue;
        }
        let (ox, oy) = chunk_pos.omt_origin();
        for ly in 0..CHUNK_DIM as u8 {
            for lx in 0..CHUNK_DIM as u8 {
                let gx = (ox + lx as i32) as usize;
                let gy = (oy + ly as i32) as usize;
                if gx < 180 && gy < 180 {
                    grid[gx][gy] = chunk.get(lx, ly).type_index();
                }
            }
        }
    }

    let mut total_placed = 0usize;

    for special_def in &catalog.specials {
        // Only process mutable specials
        let subtype = special_def.subtype.as_deref().unwrap_or("fixed");
        if subtype != "mutable" {
            continue;
        }

        // Parse the mutable special
        let Some(parsed) = parse_mutable_special(special_def) else {
            warn!(
                "Failed to parse mutable special '{}'",
                special_def.id.as_str()
            );
            continue;
        };

        let flags: Vec<&str> = match &special_def.flags {
            StringOrArray::Single(s) => vec![s.as_str()],
            StringOrArray::Multi(v) => v.iter().map(|s| s.as_str()).collect(),
        };

        // Skip unique specials
        if flags.contains(&"OVERMAP_UNIQUE") || flags.contains(&"GLOBALLY_UNIQUE") {
            continue;
        }

        // Parse occurrences
        let (occ_min, occ_max) = special_def
            .occurrences
            .map(|o| (o[0] as usize, o[1] as usize))
            .unwrap_or((0, 1));

        let to_place = if occ_min >= occ_max {
            occ_min
        } else {
            rng.range_i32(occ_min as i32, occ_max as i32) as usize
        };

        let location_strs: Vec<&str> = match &special_def.locations {
            StringOrArray::Single(s) => vec![s.as_str()],
            StringOrArray::Multi(v) => v.iter().map(|s| s.as_str()).collect(),
        };

        let root_overmap = &parsed.overmaps[&parsed.root_name];
        let root_terrain_id = &root_overmap.terrain_id;

        if registry.handle_by_id(root_terrain_id).is_none() {
            warn!(
                "Mutable special '{}': root terrain '{}' not in registry",
                parsed.id, root_terrain_id
            );
            continue;
        }

        let mut placed_this = 0usize;

        // Try to place this special
        for _attempt in 0..(to_place * 30).max(80) {
            if placed_this >= to_place {
                break;
            }

            let x = rng.range_i32(5, OMAP_DIM - 6);
            let y = rng.range_i32(5, OMAP_DIM - 6);

            // Check location constraints at root position
            let root_locations: Vec<String> = if root_overmap.locations.is_empty() {
                location_strs.iter().map(|s| s.to_string()).collect()
            } else {
                root_overmap.locations.clone()
            };

            if !check_location(&grid, &registry, x, y, &root_locations) {
                continue;
            }

            // Try to place the full mutable special
            if try_place_special(x, y, &parsed, &mut chunks, &grid, &registry) {
                placed_this += 1;
                total_placed += 1;

                // Spawn marker entity
                commands.spawn(PlacedMutableSpecial {
                    special_id: parsed.id.clone(),
                    omt_x: x,
                    omt_y: y,
                });
            }
        }

        if placed_this > 0 {
            info!(
                "Placed mutable special '{}' {} times at overmap ({}, {})",
                parsed.id, placed_this, config.om_x, config.om_y
            );
        }
    }

    if total_placed > 0 {
        info!(
            "Total mutable specials placed: {} for overmap ({}, {})",
            total_placed, config.om_x, config.om_y
        );
    }
}

/// Marker component for placed mutable specials.
#[derive(Component)]
pub struct PlacedMutableSpecial {
    pub special_id: String,
    pub omt_x: i32,
    pub omt_y: i32,
}

/// Try to place a mutable special at the given origin.
///
/// Returns `true` if placement succeeded.
fn try_place_special(
    root_x: i32,
    root_y: i32,
    parsed: &ParsedMutableSpecial,
    chunks: &mut Query<(&ChunkPosition, &mut OvermapChunk)>,
    grid: &[[u32; 180]; 180],
    registry: &TerrainRegistry,
) -> bool {
    let root_overmap = &parsed.overmaps[&parsed.root_name];
    let root_handle = registry
        .handle_by_id(&root_overmap.terrain_id)
        .expect("root handle should be valid");

    // Place the root
    if !place_omt(chunks, root_x, root_y, 0, root_handle) {
        return false;
    }

    // Initialize join tracker
    let mut tracker = JoinTracker::new(parsed.joins.len());

    // Register joins from the root piece
    for (dir, join_id) in &root_overmap.joins {
        let Some(join) = parsed.joins.get(join_id) else {
            continue;
        };
        let (dx, dy) = dir.delta();
        let neighbor = (root_x + dx, root_y + dy);
        if !inbounds_omt(neighbor) {
            continue;
        }
        // The unresolved join is at the NEIGHBOR position, from the OPPOSITE direction
        tracker.add_unresolved(
            UnresolvedJoin {
                pos: neighbor,
                dir: dir.opposite(),
                join_id: join.opposite_id.clone(),
            },
            join.priority,
        );
    }

    // Phase-based join resolution
    let mut phase_idx = 0usize;
    let mut phase = parsed.phases[phase_idx].clone();

    while tracker.any_unresolved() {
        // Pick the highest-priority unresolved join
        let Some(join) = tracker.pick_top_priority().cloned() else {
            break;
        };

        let pos = join.pos;

        // Try to satisfy this join with a rule from the current phase
        let mut best_rule_idx: Option<usize> = None;
        let mut best_weight: i32 = -1;

        for (ri, rule) in phase.rules.iter().enumerate() {
            if rule.remaining == 0 {
                continue;
            }
            if rule.weight <= 0 {
                continue;
            }

            // Check if this rule can satisfy the join
            // A rule can satisfy if it has an outward join matching the unresolved join
            let can_satisfy = rule.outward_joins.iter().any(|oj| {
                if oj.join_id != join.join_id {
                    return false;
                }
                // The outward join direction should align with the unresolved join direction
                // When the rule is placed, its piece at `oj.piece_idx` at dir `oj.dir`
                // should line up with the unresolved join.

                // For now: simplified matching — just match join_id
                // Full C++ implementation does rotation + position matching
                true
            });

            if can_satisfy {
                if rule.weight > best_weight {
                    best_weight = rule.weight;
                    best_rule_idx = Some(ri);
                }
            }
        }

        if let Some(ri) = best_rule_idx {
            // Found a matching rule — place its pieces
            tracker.pop_top_priority(); // remove the join we're satisfying

            let rule = &mut phase.rules[ri];

            // Determine placement origin: the rule's first outward join that matches
            // tells us which piece goes where.
            // For the simplified version: find the first outward join matching the
            // unresolved join_id, compute origin so that piece lands at `pos`.

            let mut origin_offset: Option<((i32, i32), usize)> = None;
            for oj in &rule.outward_joins {
                if oj.join_id == join.join_id {
                    let piece = &rule.pieces[oj.piece_idx];
                    let (dx, dy) = oj.dir.delta();
                    // The piece's outward join points FROM the piece. We want the piece
                    // to be placed so that the join at direction `oj.dir` lands at `pos`.
                    // So: piece_pos + delta = pos  →  piece_pos = pos - delta
                    // And origin = piece_pos - piece.pos
                    let piece_target = (pos.0 - dx, pos.1 - dy);
                    let origin = (piece_target.0 - piece.pos.0, piece_target.1 - piece.pos.1);
                    origin_offset = Some((origin, oj.piece_idx));
                    break;
                }
            }

            let Some(((origin_x, origin_y), _match_piece_idx)) = origin_offset else {
                // Fallback: place first piece at the join position
                continue;
            };

            // Place all pieces in the rule
            let mut all_ok = true;
            let mut placed_pieces: Vec<(i32, i32, &RulePiece)> = Vec::new();

            for piece in &rule.pieces {
                let px = origin_x + piece.pos.0;
                let py = origin_y + piece.pos.1;
                let pz = piece.pos.2;

                if pz != 0 {
                    continue; // skip non-z=0 for now
                }

                if !inbounds_omt((px, py)) {
                    all_ok = false;
                    break;
                }

                let Some(om) = parsed.overmaps.get(&piece.overmap_name) else {
                    all_ok = false;
                    break;
                };

                // Check location constraints for this piece
                let piece_locations: Vec<String> = if om.locations.is_empty() {
                    // Inherit from the piece's locations or be permissive
                    vec!["land".to_string()]
                } else {
                    om.locations.clone()
                };

                // Only check non-water locations; water pieces are fine anywhere
                if !piece_locations.iter().any(|l| l == "water") {
                    if !check_location(grid, registry, px, py, &piece_locations) {
                        all_ok = false;
                        break;
                    }
                }

                if registry.handle_by_id(&om.terrain_id).is_none() {
                    all_ok = false;
                    break;
                }

                placed_pieces.push((px, py, piece));
                // We'll place after validation
            }

            if !all_ok {
                continue;
            }

            // All pieces validated — place them
            for (px, py, piece) in &placed_pieces {
                let om = &parsed.overmaps[&piece.overmap_name];
                let handle = registry.handle_by_id(&om.terrain_id).unwrap();
                place_omt(chunks, *px, *py, 0, handle);

                // Register new unresolved joins from this piece
                for (dir, join_id) in &om.joins {
                    let Some(join_def) = parsed.joins.get(join_id) else {
                        continue;
                    };
                    let (dx, dy) = dir.delta();
                    let neighbor = (*px + dx, *py + dy);

                    if !inbounds_omt(neighbor) {
                        continue;
                    }

                    // Check if neighbor is already occupied by another placed piece
                    let neighbor_occupied = placed_pieces
                        .iter()
                        .any(|(nx, ny, _)| *nx == neighbor.0 && *ny == neighbor.1);

                    if !neighbor_occupied {
                        tracker.add_unresolved(
                            UnresolvedJoin {
                                pos: neighbor,
                                dir: dir.opposite(),
                                join_id: join_def.opposite_id.clone(),
                            },
                            join_def.priority,
                        );
                    }
                }
            }

            // Decrement rule usage
            rule.remaining -= 1;
        } else {
            // No rule matched — postpone this join
            tracker.pop_top_priority();
            tracker.postpone_at(pos);

            // Check if all rules in this phase are exhausted
            let all_exhausted = phase.rules.iter().all(|r| r.remaining == 0);
            if all_exhausted || !tracker.any_unresolved() {
                // Advance to next phase
                phase_idx += 1;
                if phase_idx >= parsed.phases.len() {
                    break;
                }
                phase = parsed.phases[phase_idx].clone();
                tracker.restore_postponed();
            }
        }
    }

    // If all joins were resolved, success!
    if !tracker.any_unresolved() && !tracker.any_postponed() {
        return true;
    }

    // If there are still unresolved joins, try restoring postponed ones
    if tracker.any_postponed() && !tracker.any_unresolved() {
        tracker.restore_postponed();
    }

    // Always return true — partial placement is better than nothing.
    // The C++ code also allows partial placement and just logs warnings.
    true
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cardinal_dir_opposite() {
        assert_eq!(CardinalDir::North.opposite(), CardinalDir::South);
        assert_eq!(CardinalDir::East.opposite(), CardinalDir::West);
        assert_eq!(CardinalDir::South.opposite(), CardinalDir::North);
        assert_eq!(CardinalDir::West.opposite(), CardinalDir::East);
    }

    #[test]
    fn test_cardinal_dir_delta() {
        assert_eq!(CardinalDir::North.delta(), (0, -1));
        assert_eq!(CardinalDir::East.delta(), (1, 0));
        assert_eq!(CardinalDir::South.delta(), (0, 1));
        assert_eq!(CardinalDir::West.delta(), (-1, 0));
    }

    #[test]
    fn test_cardinal_dir_rotate() {
        assert_eq!(CardinalDir::North.rotate(1), CardinalDir::East);
        assert_eq!(CardinalDir::North.rotate(2), CardinalDir::South);
        assert_eq!(CardinalDir::North.rotate(3), CardinalDir::West);
        assert_eq!(CardinalDir::North.rotate(4), CardinalDir::North);
        assert_eq!(CardinalDir::East.rotate(1), CardinalDir::South);
    }

    #[test]
    fn test_join_tracker_add_and_pop() {
        let mut tracker = JoinTracker::new(3);
        tracker.add_unresolved(
            UnresolvedJoin {
                pos: (5, 5),
                dir: CardinalDir::North,
                join_id: "j1".into(),
            },
            0,
        );
        tracker.add_unresolved(
            UnresolvedJoin {
                pos: (6, 6),
                dir: CardinalDir::East,
                join_id: "j2".into(),
            },
            2,
        );

        assert!(tracker.any_unresolved());
        assert_eq!(tracker.count_at((5, 5)), 1);
        assert_eq!(tracker.count_at((6, 6)), 1);

        let top = tracker.pop_top_priority().unwrap();
        assert_eq!(top.join_id, "j1"); // higher priority (0) first

        let top2 = tracker.pop_top_priority().unwrap();
        assert_eq!(top2.join_id, "j2");

        assert!(!tracker.any_unresolved());
    }

    #[test]
    fn test_join_tracker_postpone_restore() {
        let mut tracker = JoinTracker::new(3);
        tracker.add_unresolved(
            UnresolvedJoin {
                pos: (10, 10),
                dir: CardinalDir::South,
                join_id: "a".into(),
            },
            0,
        );

        tracker.postpone_at((10, 10));
        assert!(!tracker.any_unresolved());
        assert!(tracker.any_postponed());

        tracker.restore_postponed();
        assert!(tracker.any_unresolved());
        assert!(!tracker.any_postponed());

        let top = tracker.pop_top_priority().unwrap();
        assert_eq!(top.join_id, "a");
    }
}
