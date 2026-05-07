//! CDDA tileset loading.
//!
//! Parses `tile_info.json` for per-sheet dimensions/offsets, then walks every
//! non-filler `pngs_*` directory and reads the per-entity JSON manifests to
//! build a registry mapping CDDA entity IDs → image handles + metadata.

use bevy::math::Vec2;
use bevy::prelude::*;
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Per-tile rendering metadata.
#[derive(Debug, Clone)]
pub struct TileInfo {
    pub image: Handle<Image>,
    /// Sprite pixel dimensions (may be larger than the 32×32 grid cell).
    pub sprite_width: u32,
    pub sprite_height: u32,
    /// CDDA-convention offset from the tile's top-left corner, in pixels.
    /// Negative values extend the sprite beyond the tile boundary (e.g. tall/large tiles).
    cdda_offset_x: i32,
    cdda_offset_y: i32,
}

impl TileInfo {
    /// Side length of the canonical grid cell in pixels.
    pub const CELL: f32 = 32.0;

    /// Sprite dimensions as a `Vec2` for `Sprite::custom_size`.
    pub fn sprite_size(&self) -> Vec2 {
        Vec2::new(self.sprite_width as f32, self.sprite_height as f32)
    }

    /// Bevy-space positional offset to add to the tile's grid-center translation.
    ///
    /// Converts from CDDA's top-left / y-down convention to Bevy's
    /// center-anchor / y-up convention:
    ///   x = cdda_offset_x + sprite_w/2 − cell/2
    ///   y = −(cdda_offset_y + sprite_h/2 − cell/2)
    pub fn bevy_offset(&self) -> Vec2 {
        let w = self.sprite_width as f32;
        let h = self.sprite_height as f32;
        let ox = self.cdda_offset_x as f32;
        let oy = self.cdda_offset_y as f32;
        Vec2::new(
            ox + w / 2.0 - Self::CELL / 2.0,
            -(oy + h / 2.0 - Self::CELL / 2.0),
        )
    }
}

/// Maps CDDA entity IDs → `TileInfo`. Inserted as a Bevy resource at startup.
#[derive(Resource, Debug, Clone)]
pub struct TileRegistry {
    tiles: HashMap<String, TileInfo>,
    fallback: TileInfo,
}

impl TileRegistry {
    /// Full metadata for a CDDA entity ID, with fallback.
    ///
    /// Tries the exact ID first, then strips OMT variant suffixes
    /// (e.g. `"barn_0_south"` → `"barn"`).
    pub fn tile_info(&self, id: &str) -> &TileInfo {
        self.tiles
            .get(id)
            .or_else(|| strip_omt_suffix(id).and_then(|b| self.tiles.get(b)))
            .unwrap_or(&self.fallback)
    }

    /// Image handle for a CDDA entity ID, with fallback.
    pub fn tile_for(&self, id: &str) -> Handle<Image> {
        self.tile_info(id).image.clone()
    }

    /// Overmap tile by building or OMT ID (alias for `tile_for`).
    pub fn overmap_tile(&self, id: &str) -> Handle<Image> {
        self.tile_info(id).image.clone()
    }

    /// Returns `true` if a real (non-fallback) tile exists for `id`.
    pub fn has_tile(&self, id: &str) -> bool {
        self.tiles.contains_key(id)
            || strip_omt_suffix(id).map_or(false, |b| self.tiles.contains_key(b))
    }
}

// ---------------------------------------------------------------------------
// Startup system
// ---------------------------------------------------------------------------

pub fn load_tiles(mut commands: Commands, asset_server: Res<AssetServer>) {
    let base: PathBuf = concat!(env!("CARGO_MANIFEST_DIR"), "/../../gfx/UltimateCataclysm").into();
    let base = std::path::absolute(&base).unwrap_or(base);

    let fallback_handle: Handle<Image> = asset_server.load("gfx/UltimateCataclysm/fallback.png");
    let fallback = TileInfo {
        image: fallback_handle,
        sprite_width: 32,
        sprite_height: 32,
        cdda_offset_x: 0,
        cdda_offset_y: 0,
    };

    let sheet_defs = parse_sheet_defs(&base.join("tile_info.json"));
    let mut tiles: HashMap<String, TileInfo> = HashMap::new();
    let mut total_jsons = 0usize;

    for sheet in &sheet_defs {
        if sheet.is_filler {
            continue;
        }
        let sheet_path = base.join(&sheet.dir_name);
        if !sheet_path.exists() {
            continue;
        }

        // One pass: collect PNG paths + JSON paths simultaneously
        let mut png_index: HashMap<String, PathBuf> = HashMap::new();
        let mut json_paths: Vec<PathBuf> = Vec::new();
        collect_files(&sheet_path, &mut png_index, &mut json_paths);
        total_jsons += json_paths.len();

        for json_path in &json_paths {
            ingest_json(json_path, sheet, &png_index, &asset_server, &mut tiles);
        }
    }

    info!(
        "Tile registry: {} entity IDs from {} JSON manifests across {} sheets",
        tiles.len(),
        total_jsons,
        sheet_defs.iter().filter(|s| !s.is_filler).count(),
    );

    commands.insert_resource(TileRegistry { tiles, fallback });
}

// ---------------------------------------------------------------------------
// tile_info.json parsing
// ---------------------------------------------------------------------------

/// Per-sheet metadata derived from `tile_info.json`.
#[derive(Debug)]
struct SheetDef {
    /// Directory name under the tileset base, e.g. `"pngs_normal_32x32"`.
    dir_name: String,
    sprite_width: u32,
    sprite_height: u32,
    offset_x: i32,
    offset_y: i32,
    is_filler: bool,
}

/// Map from the `tile_info.json` filename key to the matching `pngs_*` directory.
fn sheet_dir(filename: &str) -> Option<&'static str> {
    match filename {
        "normal.png" => Some("pngs_normal_32x32"),
        "small.png" => Some("pngs_small_20x20"),
        "tall.png" => Some("pngs_tall_32x64"),
        "human_body.png" => Some("pngs_human_body_32x36"),
        "human_body_plus.png" => Some("pngs_human_body_plus_32x48"),
        "centered.png" => Some("pngs_centered_64x64"),
        "large.png" => Some("pngs_large_64x64"),
        "large_ridden.png" => Some("pngs_large_ridden_64x64"),
        "huge.png" => Some("pngs_huge_64x96"),
        "giant.png" => Some("pngs_giant_96x96"),
        "incomplete_small.png" => Some("pngs_incomplete_small_20x20"),
        "incomplete.png" => Some("pngs_incomplete_32x32"),
        "incomplete_tall.png" => Some("pngs_incomplete_tall_32x64"),
        "incomplete_body_plus.png" => Some("pngs_incomplete_body_plus_32x48"),
        "incomplete_large.png" => Some("pngs_incomplete_large_64x64"),
        "incomplete_giant.png" => Some("pngs_incomplete_giant_96x96"),
        "fillerhoder.png" => Some("pngs_fillerhoder_32x32"),
        "filler.png" => Some("pngs_filler_32x32"),
        "filler_tall.png" => Some("pngs_filler_tall_32x64"),
        "fillergiant.png" => Some("pngs_fillergiant_96x96"),
        _ => None,
    }
}

fn parse_sheet_defs(path: &Path) -> Vec<SheetDef> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            warn!("Cannot read tile_info.json: {e}");
            return vec![];
        }
    };
    let arr: Vec<Value> = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            warn!("Cannot parse tile_info.json: {e}");
            return vec![];
        }
    };

    // First entry carries global defaults.
    let first = arr.first().and_then(|v| v.as_object());
    let gw = first.and_then(|o| o.get("width")).and_then(|v| v.as_u64()).unwrap_or(32) as u32;
    let gh = first.and_then(|o| o.get("height")).and_then(|v| v.as_u64()).unwrap_or(32) as u32;

    let mut defs = Vec::new();
    for entry in arr.iter().skip(1) {
        let obj = match entry.as_object() {
            Some(o) => o,
            None => continue,
        };
        for (filename, props) in obj {
            if filename == "fallback.png" {
                continue; // handled separately as the registry fallback
            }
            let dir_name = match sheet_dir(filename) {
                Some(d) => d.to_string(),
                None => continue,
            };
            let p = props.as_object();
            let sw = p.and_then(|o| o.get("sprite_width")).and_then(|v| v.as_u64()).unwrap_or(gw as u64) as u32;
            let sh = p.and_then(|o| o.get("sprite_height")).and_then(|v| v.as_u64()).unwrap_or(gh as u64) as u32;
            let ox = p.and_then(|o| o.get("sprite_offset_x")).and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let oy = p.and_then(|o| o.get("sprite_offset_y")).and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let filler = p.and_then(|o| o.get("filler")).and_then(|v| v.as_bool()).unwrap_or(false);
            defs.push(SheetDef { dir_name, sprite_width: sw, sprite_height: sh, offset_x: ox, offset_y: oy, is_filler: filler });
        }
    }
    defs
}

// ---------------------------------------------------------------------------
// File collection (single recursive walk per sheet)
// ---------------------------------------------------------------------------

/// Walk `dir` recursively, indexing PNGs by stem and collecting JSON paths.
fn collect_files(dir: &Path, png_index: &mut HashMap<String, PathBuf>, json_paths: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            collect_files(&p, png_index, json_paths);
        } else {
            match p.extension().and_then(|e| e.to_str()) {
                Some("png") => {
                    if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                        // Earlier (smaller/less-detailed) entries win;
                        // sheets are processed in tile_info.json order so
                        // larger tiles from later sheets can overwrite them
                        // via `ingest_json` using a plain `insert`.
                        png_index.entry(stem.to_string()).or_insert(p);
                    }
                }
                Some("json") => json_paths.push(p),
                _ => {}
            }
        }
    }
}

// ---------------------------------------------------------------------------
// JSON manifest parsing
// ---------------------------------------------------------------------------

/// Parse one tile manifest JSON and insert discovered `TileInfo` entries.
///
/// Later sheets overwrite earlier ones (tile_info.json order: small → normal →
/// large → …), so the highest-quality available tile ends up in the registry.
fn ingest_json(
    json_path: &Path,
    sheet: &SheetDef,
    png_index: &HashMap<String, PathBuf>,
    asset_server: &AssetServer,
    out: &mut HashMap<String, TileInfo>,
) {
    let content = match std::fs::read_to_string(json_path) {
        Ok(c) => c,
        Err(_) => return,
    };

    // Manifests are either a single object `{…}` or an array `[{…}, …]`.
    let entries: Vec<Value> = if content.trim_start().starts_with('[') {
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        serde_json::from_str::<Value>(&content)
            .ok()
            .map(|v| vec![v])
            .unwrap_or_default()
    };

    for entry in &entries {
        let obj = match entry.as_object() {
            Some(o) => o,
            None => continue,
        };
        let id_val = match obj.get("id") {
            Some(v) => v,
            None => continue,
        };
        let fg_val = match obj.get("fg") {
            Some(v) => v,
            None => continue,
        };

        let sprite_name = match primary_sprite_name(fg_val) {
            Some(s) => s,
            None => continue,
        };

        let abs_png = match png_index.get(sprite_name) {
            Some(p) => p,
            None => continue,
        };

        let asset_path = match abs_png.to_string_lossy().find("gfx/") {
            Some(idx) => abs_png.to_string_lossy()[idx..].to_string(),
            None => continue,
        };

        let handle: Handle<Image> = asset_server.load(&asset_path);
        let info = TileInfo {
            image: handle,
            sprite_width: sheet.sprite_width,
            sprite_height: sheet.sprite_height,
            cdda_offset_x: sheet.offset_x,
            cdda_offset_y: sheet.offset_y,
        };

        for id in extract_ids(id_val) {
            out.insert(id, info.clone());
        }
    }
}

// ---------------------------------------------------------------------------
// JSON helpers
// ---------------------------------------------------------------------------

/// Extract the primary foreground sprite name from a `fg` value.
///
/// Handles:
/// - `"sprite_name"` — plain string
/// - `["s1","s2",…]` — rotation array; take index 0
/// - `[{"weight":N,"sprite":"s"},…]` — weighted variants; take highest weight
fn primary_sprite_name(fg: &Value) -> Option<&str> {
    match fg {
        Value::String(s) if !s.is_empty() => Some(s.as_str()),
        Value::Array(arr) if !arr.is_empty() => match &arr[0] {
            Value::String(s) => Some(s.as_str()),
            Value::Object(_) => arr
                .iter()
                .filter_map(|v| v.as_object())
                .max_by_key(|o| o.get("weight").and_then(|w| w.as_i64()).unwrap_or(0))
                .and_then(|o| o.get("sprite"))
                .and_then(|s| s.as_str()),
            _ => None,
        },
        _ => None,
    }
}

/// Collect all CDDA entity IDs from an `id` value (string or string array).
fn extract_ids(id_val: &Value) -> Vec<String> {
    match id_val {
        Value::String(s) => vec![s.clone()],
        Value::Array(arr) => arr.iter().filter_map(|v| v.as_str().map(String::from)).collect(),
        _ => vec![],
    }
}

// ---------------------------------------------------------------------------
// OMT suffix stripping
// ---------------------------------------------------------------------------

/// Strip OMT variant suffixes to get the base tile ID.
///
/// OMT IDs follow `{base}[_{index}][_{direction}]`, e.g.:
/// - `"barn_0_south"` → `"barn"`
/// - `"abstorefront_1"` → `"abstorefront"`
/// - `"2storyModern01_1_north"` → `"2storyModern01"`
///
/// Returns `None` if no strippable suffix is found.
fn strip_omt_suffix(id: &str) -> Option<&str> {
    const DIRECTIONS: &[&str] = &[
        "north", "south", "east", "west",
        "ne", "nw", "se", "sw",
        "n", "s", "e", "w",
    ];

    let mut s = id;
    let mut stripped = false;

    // Strip a trailing direction component first.
    for dir in DIRECTIONS {
        let pat = format!("_{dir}");
        if let Some(rest) = s.strip_suffix(pat.as_str()) {
            s = rest;
            stripped = true;
            break;
        }
    }

    // Strip trailing `_<digits>` components.
    loop {
        let Some(pos) = s.rfind('_') else { break };
        let after = &s[pos + 1..];
        if !after.is_empty() && after.bytes().all(|b| b.is_ascii_digit()) {
            s = &s[..pos];
            stripped = true;
        } else {
            break;
        }
    }

    if stripped { Some(s) } else { None }
}
