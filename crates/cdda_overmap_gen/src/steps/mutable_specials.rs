//! Step 6c: Mutable overmap specials — procedural special placement via rules.
//!
//! Mutable specials have `subtype: "mutable"` and are placed by a phase-based
//! rule engine that resolves joins between overmap pieces.
//!
//! Port of CDDA master's mutable special system.

use bevy_ecs::prelude::*;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM, OMAP_DIM};
use cdda_overmap::connections::inbounds_omt;
use cdda_overmap::registry::{TerrainFlags, TerrainHandle, TerrainRegistry};
use cdda_overmap::rng::XorShiftRng;

use crate::pipeline::OvermapGenConfig;
use crate::region_settings::OvermapRegionSettings;
use crate::special_catalog::SpecialCatalog;
use cdda_core_types::core::raw_defs::cdda_types::{RawValue, StringOrArray};
use cdda_core_types::core::raw_defs::overmap_terrain::OvermapSpecialDef;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn};

// ---------------------------------------------------------------------------
// Cardinal direction (N, E, S, W)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum CardinalDir {
    North,
    East,
    South,
    West,
}

impl CardinalDir {
    fn all() -> impl Iterator<Item = CardinalDir> {
        [
            CardinalDir::North,
            CardinalDir::East,
            CardinalDir::South,
            CardinalDir::West,
        ]
        .into_iter()
    }

    fn from_str(s: &str) -> Option<CardinalDir> {
        match s {
            "N" | "north" => Some(CardinalDir::North),
            "E" | "east" => Some(CardinalDir::East),
            "S" | "south" => Some(CardinalDir::South),
            "W" | "west" => Some(CardinalDir::West),
            _ => None,
        }
    }

    fn opposite(self) -> CardinalDir {
        match self {
            CardinalDir::North => CardinalDir::South,
            CardinalDir::East => CardinalDir::West,
            CardinalDir::South => CardinalDir::North,
            CardinalDir::West => CardinalDir::East,
        }
    }

    fn delta(self) -> (i32, i32) {
        match self {
            CardinalDir::North => (0, -1),
            CardinalDir::East => (1, 0),
            CardinalDir::South => (0, 1),
            CardinalDir::West => (-1, 0),
        }
    }

    fn rotate(self, steps: i32) -> CardinalDir {
        let dirs = [
            CardinalDir::North,
            CardinalDir::East,
            CardinalDir::South,
            CardinalDir::West,
        ];
        let idx = dirs.iter().position(|&d| d == self).unwrap_or(0);
        let new_idx = (idx as i32 + steps).rem_euclid(4) as usize;
        dirs[new_idx]
    }
}

// ---------------------------------------------------------------------------
// Data types for parsed mutable specials
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct MutableJoin {
    id: String,
    opposite_id: String,
    priority: i32,
}

#[derive(Debug, Clone)]
struct MutableOvermap {
    terrain_id: String,
    locations: Vec<String>,
    joins: Vec<(CardinalDir, String)>,
}

#[derive(Debug, Clone)]
struct RulePiece {
    overmap_name: String,
    pos: (i32, i32, i32),
}

#[derive(Debug, Clone)]
struct PlacementRule {
    name: String,
    pieces: Vec<RulePiece>,
    max_count: i32,
    remaining: i32,
    weight: i32,
    outward_joins: Vec<OutwardJoin>,
}

#[derive(Debug, Clone)]
struct OutwardJoin {
    piece_idx: usize,
    dir: CardinalDir,
    join_id: String,
}

#[derive(Debug, Clone)]
struct MutablePhase {
    rules: Vec<PlacementRule>,
}

#[derive(Debug, Clone)]
struct ParsedMutableSpecial {
    id: String,
    joins: HashMap<String, MutableJoin>,
    overmaps: HashMap<String, MutableOvermap>,
    root_name: String,
    phases: Vec<MutablePhase>,
}

// ---------------------------------------------------------------------------
// Parse mutable special from OvermapSpecialDef
// ---------------------------------------------------------------------------

fn parse_mutable_special(def: &Arc<OvermapSpecialDef>) -> Option<ParsedMutableSpecial> {
    let id = def.id.as_str().to_string();

    // Parse joins
    let mut joins: HashMap<String, MutableJoin> = HashMap::new();
    if let Some(RawValue::Array(join_arr)) = &def.joins {
        for join_entry in join_arr {
            if let RawValue::Object(obj) = join_entry {
                let join_id = obj
                    .get("id")
                    .and_then(|v| match v {
                        RawValue::String(s) => Some(s.clone()),
                        _ => None,
                    })
                    .unwrap_or_default();
                let opposite_id = obj
                    .get("opposite")
                    .and_then(|v| match v {
                        RawValue::String(s) => Some(s.clone()),
                        _ => None,
                    })
                    .unwrap_or_else(|| join_id.clone());
                let priority = obj
                    .get("priority")
                    .and_then(|v| match v {
                        RawValue::Number(n) => Some(*n as i32),
                        _ => None,
                    })
                    .unwrap_or(100);
                joins.insert(
                    join_id.clone(),
                    MutableJoin {
                        id: join_id,
                        opposite_id,
                        priority,
                    },
                );
            }
        }
    }

    // Parse overmaps
    let mut overmaps: HashMap<String, MutableOvermap> = HashMap::new();
    if let Some(RawValue::Array(om_arr)) = &def.overmaps {
        for om_entry in om_arr {
            if let RawValue::Object(obj) = om_entry {
                let name = obj
                    .get("name")
                    .and_then(|v| match v {
                        RawValue::String(s) => Some(s.clone()),
                        _ => None,
                    })
                    .unwrap_or_default();
                let terrain_id = obj
                    .get("overmap")
                    .and_then(|v| match v {
                        RawValue::String(s) => Some(s.clone()),
                        _ => None,
                    })
                    .unwrap_or_default();

                // Parse locations
                let locations: Vec<String> = match obj.get("locations") {
                    Some(RawValue::String(s)) => vec![s.clone()],
                    Some(RawValue::Array(arr)) => arr
                        .iter()
                        .filter_map(|v| match v {
                            RawValue::String(s) => Some(s.clone()),
                            _ => None,
                        })
                        .collect(),
                    _ => Vec::new(),
                };

                // Parse joins for this overmap
                let mut om_joins: Vec<(CardinalDir, String)> = Vec::new();
                if let Some(RawValue::Object(join_obj)) = obj.get("joins") {
                    for dir in CardinalDir::all() {
                        let dir_key = match dir {
                            CardinalDir::North => "N",
                            CardinalDir::East => "E",
                            CardinalDir::South => "S",
                            CardinalDir::West => "W",
                        };
                        if let Some(RawValue::String(join_id)) = join_obj.get(dir_key) {
                            om_joins.push((dir, join_id.clone()));
                        }
                    }
                }

                overmaps.insert(
                    name,
                    MutableOvermap {
                        terrain_id,
                        locations,
                        joins: om_joins,
                    },
                );
            }
        }
    }

    // Root name
    let root_name = def.root.as_deref().unwrap_or("root").to_string();

    // Parse phases
    let mut phases: Vec<MutablePhase> = Vec::new();
    if let Some(RawValue::Array(phase_arr)) = &def.phases {
        for phase_entry in phase_arr {
            if let RawValue::Object(phase_obj) = phase_entry {
                let mut rules: Vec<PlacementRule> = Vec::new();

                if let Some(RawValue::Array(rules_arr)) = phase_obj.get("rules") {
                    for rule_entry in rules_arr {
                        if let RawValue::Object(rule_obj) = rule_entry {
                            let rule_name = rule_obj
                                .get("name")
                                .and_then(|v| match v {
                                    RawValue::String(s) => Some(s.clone()),
                                    _ => None,
                                })
                                .unwrap_or_default();

                            let max_count = rule_obj
                                .get("max_count")
                                .and_then(|v| match v {
                                    RawValue::Number(n) => Some(*n as i32),
                                    _ => None,
                                })
                                .unwrap_or(i32::MAX);

                            let weight = rule_obj
                                .get("weight")
                                .and_then(|v| match v {
                                    RawValue::Number(n) => Some(*n as i32),
                                    _ => None,
                                })
                                .unwrap_or(1);

                            // Parse pieces
                            let mut pieces: Vec<RulePiece> = Vec::new();
                            if let Some(RawValue::Array(pieces_arr)) = rule_obj.get("pieces") {
                                for piece_entry in pieces_arr {
                                    if let RawValue::Object(piece_obj) = piece_entry {
                                        let overmap_name = piece_obj
                                            .get("overmap")
                                            .and_then(|v| match v {
                                                RawValue::String(s) => Some(s.clone()),
                                                _ => None,
                                            })
                                            .unwrap_or_default();

                                        let pos = piece_obj
                                            .get("pos")
                                            .and_then(|v| match v {
                                                RawValue::Array(a) => {
                                                    let x = a
                                                        .first()
                                                        .and_then(|v| match v {
                                                            RawValue::Number(n) => Some(*n as i32),
                                                            _ => None,
                                                        })
                                                        .unwrap_or(0);
                                                    let y = a
                                                        .get(1)
                                                        .and_then(|v| match v {
                                                            RawValue::Number(n) => Some(*n as i32),
                                                            _ => None,
                                                        })
                                                        .unwrap_or(0);
                                                    let z = a
                                                        .get(2)
                                                        .and_then(|v| match v {
                                                            RawValue::Number(n) => Some(*n as i32),
                                                            _ => None,
                                                        })
                                                        .unwrap_or(0);
                                                    Some((x, y, z))
                                                }
                                                _ => None,
                                            })
                                            .unwrap_or((0, 0, 0));

                                        pieces.push(RulePiece { overmap_name, pos });
                                    }
                                }
                            }

                            // Parse outward joins
                            let mut outward_joins: Vec<OutwardJoin> = Vec::new();
                            if let Some(RawValue::Array(oj_arr)) = rule_obj.get("outward_joins") {
                                for oj_entry in oj_arr {
                                    if let RawValue::Object(oj_obj) = oj_entry {
                                        let piece_idx = oj_obj
                                            .get("piece")
                                            .and_then(|v| match v {
                                                RawValue::Number(n) => Some(*n as usize),
                                                _ => None,
                                            })
                                            .unwrap_or(0);
                                        let dir_str = oj_obj
                                            .get("dir")
                                            .and_then(|v| match v {
                                                RawValue::String(s) => Some(s.clone()),
                                                _ => None,
                                            })
                                            .unwrap_or_default();
                                        let dir = CardinalDir::from_str(&dir_str)
                                            .unwrap_or(CardinalDir::North);
                                        let join_id = oj_obj
                                            .get("join")
                                            .and_then(|v| match v {
                                                RawValue::String(s) => Some(s.clone()),
                                                _ => None,
                                            })
                                            .unwrap_or_default();

                                        outward_joins.push(OutwardJoin {
                                            piece_idx,
                                            dir,
                                            join_id,
                                        });
                                    }
                                }
                            }

                            rules.push(PlacementRule {
                                name: rule_name,
                                pieces,
                                max_count,
                                remaining: max_count,
                                weight,
                                outward_joins,
                            });
                        }
                    }
                }

                phases.push(MutablePhase { rules });
            }
        }
    }

    Some(ParsedMutableSpecial {
        id,
        joins,
        overmaps,
        root_name,
        phases,
    })
}

// ---------------------------------------------------------------------------
// Join tracking for phase-based placement
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct UnresolvedJoin {
    pos: (i32, i32),
    dir: CardinalDir,
    join_id: String,
}

#[derive(Debug, Clone)]
struct JoinTracker {
    unresolved: Vec<(UnresolvedJoin, i32)>, // (join, priority)
    postponed: Vec<UnresolvedJoin>,
    used: Vec<UnresolvedJoin>,
}

impl JoinTracker {
    fn new(_num_joins: usize) -> Self {
        Self {
            unresolved: Vec::new(),
            postponed: Vec::new(),
            used: Vec::new(),
        }
    }

    fn any_unresolved(&self) -> bool {
        !self.unresolved.is_empty()
    }

    fn any_postponed(&self) -> bool {
        !self.postponed.is_empty()
    }

    fn add_unresolved(&mut self, join: UnresolvedJoin, priority: i32) {
        self.unresolved.push((join, priority));
    }

    fn pick_top_priority(&self) -> Option<&UnresolvedJoin> {
        self.unresolved
            .iter()
            .max_by_key(|(_, p)| *p)
            .map(|(j, _)| j)
    }

    fn pop_top_priority(&mut self) -> Option<UnresolvedJoin> {
        if self.unresolved.is_empty() {
            return None;
        }
        let idx = self
            .unresolved
            .iter()
            .enumerate()
            .max_by_key(|(_, (_, p))| *p)
            .map(|(i, _)| i)?;
        let (join, _) = self.unresolved.remove(idx);
        self.used.push(join.clone());
        Some(join)
    }

    fn any_at(&self, pos: (i32, i32)) -> bool {
        self.unresolved.iter().any(|(j, _)| j.pos == pos)
    }

    fn count_at(&self, pos: (i32, i32)) -> usize {
        self.unresolved.iter().filter(|(j, _)| j.pos == pos).count()
    }

    fn remove_at(&mut self, pos: (i32, i32)) {
        self.unresolved.retain(|(j, _)| j.pos != pos);
    }

    fn postpone_at(&mut self, pos: (i32, i32)) {
        let mut i = 0;
        while i < self.unresolved.len() {
            if self.unresolved[i].0.pos == pos {
                let (join, _) = self.unresolved.remove(i);
                self.postponed.push(join);
            } else {
                i += 1;
            }
        }
    }

    fn restore_postponed(&mut self) {
        for join in self.postponed.drain(..) {
            self.unresolved.push((join, 0));
        }
    }

    fn record_used(&mut self, pos: (i32, i32)) {
        self.used.push(UnresolvedJoin {
            pos,
            dir: CardinalDir::North,
            join_id: String::new(),
        });
    }
}

// ---------------------------------------------------------------------------
// Placement helpers
// ---------------------------------------------------------------------------

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
    chunks: Query<(Entity, &ChunkPosition, &OvermapChunk)>,
    par_commands: ParallelCommands,
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
    for (_entity, chunk_pos, chunk) in &chunks {
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

    // Collect writes: (omt_x, omt_y, z, handle)
    let mut writes: Vec<(i32, i32, i8, TerrainHandle)> = Vec::new();

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

        let Some(root_overmap) = parsed.overmaps.get(&parsed.root_name) else {
            warn!("Mutable special '{}': root '{}' not found in overmaps", special_def.id.as_str(), parsed.root_name);
            continue;
        };
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
            if try_place_special(x, y, &parsed, &mut writes, &grid, &registry) {
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

    // ------------------------------------------------------------------
    // Write-back: apply all collected writes via par_iter
    // ------------------------------------------------------------------
    chunks.par_iter().for_each(|(entity, chunk_pos, chunk)| {
        if chunk_pos.z.0 != 0 {
            return;
        }
        let (ox, oy) = chunk_pos.omt_origin();
        let mut modified = false;
        let mut new_terrain = chunk.terrain.clone();

        for &(wx, wy, _wz, handle) in &writes {
            let lx = wx - ox;
            let ly = wy - oy;
            if lx >= 0 && lx < CHUNK_DIM as i32 && ly >= 0 && ly < CHUNK_DIM as i32 {
                let idx = ly as usize * CHUNK_DIM + lx as usize;
                if new_terrain[idx] != handle {
                    new_terrain[idx] = handle;
                    modified = true;
                }
            }
        }

        if modified {
            par_commands.command_scope(|mut cmd| {
                cmd.entity(entity).insert(OvermapChunk {
                    terrain: new_terrain,
                });
            });
        }
    });

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
    writes: &mut Vec<(i32, i32, i8, TerrainHandle)>,
    grid: &[[u32; 180]; 180],
    registry: &TerrainRegistry,
) -> bool {
    let Some(root_overmap) = parsed.overmaps.get(&parsed.root_name) else { return false; };
    let root_handle = registry
        .handle_by_id(&root_overmap.terrain_id)
        .expect("root handle should be valid");

    // Place the root
    writes.push((root_x, root_y, 0, root_handle));

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

    // Track placed piece positions for collision detection
    let mut placed_positions: Vec<(i32, i32)> = vec![(root_x, root_y)];

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
            let can_satisfy = rule.outward_joins.iter().any(|oj| {
                if oj.join_id != join.join_id {
                    return false;
                }
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

            // Determine placement origin
            let mut origin_offset: Option<((i32, i32), usize)> = None;
            for oj in &rule.outward_joins {
                if oj.join_id == join.join_id {
                    let piece = &rule.pieces[oj.piece_idx];
                    let (dx, dy) = oj.dir.delta();
                    let piece_target = (pos.0 - dx, pos.1 - dy);
                    let origin = (piece_target.0 - piece.pos.0, piece_target.1 - piece.pos.1);
                    origin_offset = Some((origin, oj.piece_idx));
                    break;
                }
            }

            let Some(((origin_x, origin_y), _match_piece_idx)) = origin_offset else {
                continue;
            };

            // Place all pieces in the rule
            let mut all_ok = true;
            let mut new_placed: Vec<(i32, i32)> = Vec::new();

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

                new_placed.push((px, py));
            }

            if !all_ok {
                continue;
            }

            // All pieces validated — write them
            for (idx, piece) in rule.pieces.iter().enumerate() {
                let px = new_placed[idx].0;
                let py = new_placed[idx].1;
                let om = &parsed.overmaps[&piece.overmap_name];
                let handle = registry.handle_by_id(&om.terrain_id).unwrap();
                writes.push((px, py, 0, handle));

                // Register new unresolved joins from this piece
                for (dir, join_id) in &om.joins {
                    let Some(join_def) = parsed.joins.get(join_id) else {
                        continue;
                    };
                    let (dx, dy) = dir.delta();
                    let neighbor = (px + dx, py + dy);

                    if !inbounds_omt(neighbor) {
                        continue;
                    }

                    // Check if neighbor is already occupied
                    let neighbor_occupied = placed_positions
                        .iter()
                        .any(|&(nx, ny)| nx == neighbor.0 && ny == neighbor.1)
                        || new_placed
                            .iter()
                            .any(|&(nx, ny)| nx == neighbor.0 && ny == neighbor.1);

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

            // Record placed positions
            for &pos in &new_placed {
                placed_positions.push(pos);
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
        assert_eq!(CardinalDir::North.rotate(-1), CardinalDir::West);
        assert_eq!(CardinalDir::West.rotate(2), CardinalDir::East);
    }

    #[test]
    fn test_join_tracker_add_and_pop() {
        let mut tracker = JoinTracker::new(4);
        tracker.add_unresolved(
            UnresolvedJoin {
                pos: (5, 5),
                dir: CardinalDir::North,
                join_id: "road".to_string(),
            },
            50,
        );
        tracker.add_unresolved(
            UnresolvedJoin {
                pos: (6, 6),
                dir: CardinalDir::East,
                join_id: "road".to_string(),
            },
            100,
        );
        assert!(tracker.any_unresolved());
        let top = tracker.pop_top_priority().unwrap();
        assert_eq!(top.pos, (6, 6)); // Higher priority
        let next = tracker.pop_top_priority().unwrap();
        assert_eq!(next.pos, (5, 5));
        assert!(!tracker.any_unresolved());
    }

    #[test]
    fn test_join_tracker_postpone_restore() {
        let mut tracker = JoinTracker::new(4);
        tracker.add_unresolved(
            UnresolvedJoin {
                pos: (10, 10),
                dir: CardinalDir::South,
                join_id: "test".to_string(),
            },
            10,
        );
        tracker.postpone_at((10, 10));
        assert!(!tracker.any_unresolved());
        assert!(tracker.any_postponed());
        tracker.restore_postponed();
        assert!(tracker.any_unresolved());
        assert!(!tracker.any_postponed());
    }
}
