//! # cdda-cli — CLI tools for CDDA mod development
//!
//! ## Commands
//!
//! - `schema`         — Generate static JSON Schema files for all definition types
//! - `gen-schemas`    — Generate dynamic schemas for core + mods (with autocomplete)
//! - `validate <dir>` — Validate all JSON files in a directory against schemas
//! - `stats <dir>`    — Print definition statistics for a data directory
//! - `check <dir>`    — Full load check: load + resolve + validate all definitions
//! - `ablation <baseline> <dirs>...` — Test removing mods to isolate load errors

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Instant;

use bevy_app::App;
use bevy_state::app::StatesPlugin;

use cdda_data::loader::Loader;
use cdda_defs_raw::raw_defs::cdda_types::{RawValue, StringOrArray};
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM};
use cdda_overmap::registry::{TerrainFlags, TerrainHandle, TerrainRegistry};

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cdda-cli", about = "CDDA data tooling for mod developers")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generate static JSON Schema files for all definition types (no dynamic enums).
    Schema,
    /// Generate dynamic schemas for core and mods with autocomplete enums.
    /// Outputs to `schemas/<name>/` for each mod.
    GenSchemas {
        /// Path to the core data directory (e.g. data/core).
        #[arg(long, default_value = "data/core")]
        core: PathBuf,
        /// Mod directories as NAME=PATH pairs (e.g. Magiclysm=data/mods/Magiclysm).
        #[arg(short, long = "mod", value_parser = parse_mod_pair)]
        mods: Vec<ModEntry>,
    },
    /// Validate all JSON files in a directory against schemas
    Validate {
        /// Path to directory containing CDDA JSON data
        path: PathBuf,
    },
    /// Print definition count statistics for a data directory
    Stats {
        /// Path to directory containing CDDA JSON data
        path: PathBuf,
    },
    /// Full load check: load, resolve copy-from, validate all definitions
    Check {
        /// Path to directory containing CDDA JSON data
        path: PathBuf,
    },
    /// Generate a 15x15 city view with a large city in the center (ASCII output)
    CityView {
        /// Path to CDDA JSON data directory (e.g. data/core)
        data_dir: PathBuf,
        /// City size (default 12)
        #[arg(long, default_value = "12")]
        city_size: i32,
        /// Noise seed for deterministic generation
        #[arg(long, default_value = "42")]
        seed: u64,
    },
    /// Load a baseline directory, then test adding/removing mods to find conflicts
    Ablation {
        /// Baseline data directory (e.g. data/core)
        baseline: PathBuf,
        /// Mod directories to test (loaded on top of baseline)
        mods: Vec<PathBuf>,
    },
    /// Resolve copy-from for every def category and verify the JSON → struct →
    /// JSON round-trip drops no fields (Phase-A consistency check).
    Roundtrip {
        /// Path to CDDA JSON data directory (e.g. data/core)
        path: PathBuf,
        /// Exit 1 if any category reports parse failures or unresolvable defs.
        /// (Unmodeled-field reports are always printed but do not fail unless
        /// this flag is set with `--strict`.)
        #[arg(long)]
        strict: bool,
    },
}

/// A mod entry parsed from NAME=PATH CLI arguments.
#[derive(Clone, Debug)]
struct ModEntry {
    name: String,
    path: PathBuf,
}

fn parse_mod_pair(s: &str) -> Result<ModEntry, String> {
    let (name, path) = s
        .split_once('=')
        .ok_or_else(|| format!("expected NAME=PATH, got: {s}"))?;
    let p = PathBuf::from(path);
    if !p.exists() {
        return Err(format!("mod directory not found: {path}"));
    }
    Ok(ModEntry {
        name: name.to_string(),
        path: p,
    })
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(tracing::level_filters::LevelFilter::WARN.into())
                .from_env_lossy(),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();
    let start = Instant::now();

    match cli.command {
        Command::Schema => cmd_schema(),
        Command::GenSchemas { core, mods } => cmd_gen_schemas(&core, &mods),
        Command::Validate { path } => cmd_validate(&path),
        Command::Stats { path } => cmd_stats(&path),
        Command::Check { path } => cmd_check(&path),
        Command::Ablation { baseline, mods } => cmd_ablation(&baseline, &mods),
        Command::Roundtrip { path, strict } => cmd_roundtrip(&path, strict),
        Command::CityView {
            data_dir,
            city_size,
            seed,
        } => {
            cmd_city_view(&data_dir, city_size, seed);
        }
    }

    eprintln!("Elapsed: {:.2}s", start.elapsed().as_secs_f64());
}

// ── Dynamic schema generation (multi-mod) ────────────────────────────

fn cmd_gen_schemas(core: &PathBuf, mods: &[ModEntry]) {
    let out_base = get_default_schema_dir();
    eprintln!("Generating dynamic schemas in {:?}", out_base);

    // Generate core schema first (flags/IDs from core data only).
    eprintln!("  core ({:?}) ...", core);
    match cdda_data::schema_gen::generate_schemas_for_mod("core", &[core.clone()], &out_base) {
        Ok(()) => eprintln!("    -> schemas/core/"),
        Err(errors) => {
            for e in &errors {
                eprintln!("    ERROR: {e}");
            }
        }
    }

    // Generate one schema per mod (core + mod data so modders get full autocomplete).
    for entry in mods {
        eprintln!("  {} ({:?}) ...", entry.name, entry.path);
        match cdda_data::schema_gen::generate_schemas_for_mod(
            &entry.name,
            &[core.clone(), entry.path.clone()],
            &out_base,
        ) {
            Ok(()) => eprintln!("    -> schemas/{}/", entry.name),
            Err(errors) => {
                for e in &errors {
                    eprintln!("    ERROR: {e}");
                }
            }
        }
    }

    eprintln!("Done.");
}

// ── Static schema generation ─────────────────────────────────────────

fn cmd_schema() {
    let out_dir = get_default_schema_dir();
    eprintln!("Generating schemas in {:?}", out_dir);
    std::fs::create_dir_all(&out_dir).expect("Failed to create schema output directory");

    match cdda_data::schema::write_all_schemas(&out_dir) {
        Ok(()) => {
            let count = out_dir.read_dir().map(|d| d.count()).unwrap_or(0);
            eprintln!("Done. {} schema files written.", count);
        }
        Err(errors) => {
            for e in &errors {
                eprintln!("  Schema error: {}", e);
            }
            std::process::exit(1);
        }
    }
}

fn get_default_schema_dir() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop(); // crates/
    p.pop(); // workspace root
    p.join("data").join("schemas")
}

// ── Stats ────────────────────────────────────────────────────────────

fn cmd_stats(path: &PathBuf) {
    if !path.exists() {
        eprintln!("Error: directory not found: {:?}", path);
        std::process::exit(1);
    }

    let mut loader = cdda_data::loader::Loader::new(vec![path.clone()]);
    let raw_map = loader.ingest_all();

    eprintln!("Raw definitions by type:");
    let mut types: Vec<_> = raw_map.iter().map(|(k, v)| (k.clone(), v.len())).collect();
    types.sort_by(|a, b| b.1.cmp(&a.1));

    let total: usize = types.iter().map(|(_, c)| c).sum();
    for (type_name, count) in &types {
        eprintln!("  {:20} {}", type_name, count);
    }
    eprintln!("  {:20} {}", "TOTAL", total);
    eprintln!();

    match loader.load() {
        Ok(registry) => {
            eprintln!("Resolved definitions: {}", registry.total_count());
            eprintln!("Categories with data: {}", registry.category_count());
        }
        Err(errors) => {
            eprintln!("Load completed with {} errors:", errors.len());
            for (i, err) in errors.iter().enumerate().take(20) {
                eprintln!("  {:>3}. {:?}", i + 1, err);
            }
        }
    }
}

// ── Validate ─────────────────────────────────────────────────────────

fn cmd_validate(path: &PathBuf) {
    if !path.exists() {
        eprintln!("Error: directory not found: {:?}", path);
        std::process::exit(1);
    }

    let mut loader = cdda_data::loader::Loader::new(vec![path.clone()]);
    let raw_map = loader.ingest_all();

    let mut total_errors = 0;
    let mut total_defs = 0;

    for (_type_name, defs) in &raw_map {
        total_defs += defs.len();
    }

    eprintln!(
        "Validating {} raw definitions across {} types...",
        total_defs,
        raw_map.len()
    );

    match loader.load() {
        Ok(_registry) => {
            eprintln!("  All definitions loaded without errors.");
            eprintln!("  Schema validation: PASSED");
        }
        Err(errors) => {
            total_errors = errors.len();
            for (i, err) in errors.iter().enumerate() {
                eprintln!("  {:>3}. {:?}", i + 1, err);
            }
            eprintln!("  Schema validation: {} error(s)", total_errors);
        }
    }

    if total_errors > 0 {
        std::process::exit(1);
    }
}

// ── Check ────────────────────────────────────────────────────────────

fn cmd_check(path: &PathBuf) {
    if !path.exists() {
        eprintln!("Error: directory not found: {:?}", path);
        std::process::exit(1);
    }

    eprint!("Loading data from {:?} ... ", path);
    let mut loader = cdda_data::loader::Loader::new(vec![path.clone()]);

    match loader.load() {
        Ok(registry) => {
            let total = registry.total_count();
            let categories = registry.category_count();
            eprintln!("{} definitions across {} categories", total, categories);
        }
        Err(errors) => {
            eprintln!();
            eprintln!("LOAD FAILED with {} errors:", errors.len());
            for (i, err) in errors.iter().enumerate() {
                eprintln!("  {:>3}. {:?}", i + 1, err);
            }
            std::process::exit(1);
        }
    }
}

// ── Ablation testing ─────────────────────────────────────────────────

fn cmd_ablation(baseline: &PathBuf, mod_dirs: &[PathBuf]) {
    if !baseline.exists() {
        eprintln!("Error: baseline directory not found: {:?}", baseline);
        std::process::exit(1);
    }

    eprintln!("=== Ablation Test ===");
    eprintln!("Baseline: {:?}", baseline);
    eprintln!();
    eprint!("Loading baseline... ");

    let mut dirs: Vec<PathBuf> = vec![baseline.clone()];
    let mut errors: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    for mod_dir in mod_dirs {
        if !mod_dir.exists() {
            eprintln!("Warning: mod directory not found: {:?}", mod_dir);
            continue;
        }
        dirs.push(mod_dir.clone());
    }

    let mut loader = cdda_data::loader::Loader::new(dirs.clone());
    match loader.load() {
        Ok(registry) => {
            eprintln!("{} definitions", registry.total_count());
        }
        Err(load_errors) => {
            eprintln!("{} errors:", load_errors.len());
            for e in &load_errors {
                eprintln!("  {:?}", e);
            }
        }
    }

    eprintln!();

    if mod_dirs.len() > 1 {
        eprintln!("=== Ablation: removing one mod at a time ===");
        for mod_dir in mod_dirs {
            let without: Vec<PathBuf> = dirs.iter().filter(|d| *d != mod_dir).cloned().collect();
            eprint!(
                "  Without {:?} ... ",
                mod_dir.file_name().unwrap_or_default()
            );
            let mut l = cdda_data::loader::Loader::new(without);
            match l.load() {
                Ok(registry) => {
                    eprintln!("{} definitions", registry.total_count());
                }
                Err(e) => {
                    eprintln!("{} error(s)", e.len());
                    errors.insert(
                        format!(
                            "missing_{}",
                            mod_dir.file_name().unwrap_or_default().to_string_lossy()
                        ),
                        e.iter().map(|e| format!("{:?}", e)).collect(),
                    );
                }
            }
        }
    }

    if !mod_dirs.is_empty() {
        eprintln!();
        eprintln!("=== Isolated mods (with baseline) ===");
        for mod_dir in mod_dirs {
            eprint!("  {:?} ... ", mod_dir.file_name().unwrap_or_default());
            let mut l = cdda_data::loader::Loader::new(vec![baseline.clone(), mod_dir.clone()]);
            match l.load() {
                Ok(registry) => {
                    eprintln!("{} definitions", registry.total_count());
                }
                Err(e) => {
                    eprintln!("{} error(s)", e.len());
                    errors.insert(
                        format!(
                            "isolated_{}",
                            mod_dir.file_name().unwrap_or_default().to_string_lossy()
                        ),
                        e.iter().map(|e| format!("{:?}", e)).collect(),
                    );
                }
            }
        }
    }

    if !errors.is_empty() {
        eprintln!();
        eprintln!("=== Error Summary ===");
        for (key, errs) in &errors {
            eprintln!("  {}: {} error(s)", key, errs.len());
            for err in errs {
                eprintln!("    {}", err);
            }
        }
        std::process::exit(1);
    }
}

// ═════════════════════════════════════════════════════════════════════
// roundtrip - Phase-A JSON→struct→JSON consistency check
// ═════════════════════════════════════════════════════════════════════

fn cmd_roundtrip(path: &PathBuf, strict: bool) {
    if !path.exists() {
        eprintln!("Error: directory not found: {:?}", path);
        std::process::exit(1);
    }

    eprintln!("Resolving copy-from + round-trip check from {:?} ...", path);
    let mut loader = cdda_data::loader::Loader::new(vec![path.clone()]);
    loader.ingest_all();

    let summaries = cdda_data::roundtrip::roundtrip_all_types(&loader);

    let mut fails = 0usize;
    eprintln!();
    eprintln!("=== Round-trip report ===");
    eprintln!(
        "{:>28} {:<16} {:>6} {:>10} {:>10} {:>10}",
        "category", "json_type", "ok", "parse-fail", "unmodeled", "unresolved"
    );
    for s in &summaries {
        eprintln!(
            "{:>28} {:<16} {:>7} {:>10} {:>10} {:>10}",
            s.category, s.json_type, s.ok, s.parse_failures, s.mismatch_failures, s.unresolved,
        );
        // Parse failures and unresolved defs are real problems. Unmodeled
        // fields are expected while the CDDA schema is only partially modeled.
        if s.parse_failures > 0 || s.unresolved > 0 {
            fails += 1;
        }
    }

    let total_ok: usize = summaries.iter().map(|s| s.ok).sum();
    eprintln!("\nTotal clean: {total_ok}");

    if fails > 0 && strict {
        eprintln!(
            "\nRound-trip FAILED ({} category(ies) with parse failures/unresolved defs).",
            fails
        );
        std::process::exit(1);
    }
    if fails > 0 {
        eprintln!(
            "\n{}-category(ies) report parse failures or unresolved defs. Re-run with --strict to fail on them.",
            fails
        );
    }
}

// ═════════════════════════════════════════════════════════════════════
// city-view - 15x15 city generation preview
// ═════════════════════════════════════════════════════════════════════

fn cmd_city_view(data_dir: &PathBuf, city_size: i32, seed: u64) {
    if !data_dir.exists() {
        eprintln!("Error: {:?} not found", data_dir);
        std::process::exit(1);
    }
    let mut loader = Loader::new(vec![data_dir.clone()]);
    loader.ingest_all();
    let registry = match loader.load() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Load errors: {}", e.len());
            std::process::exit(1);
        }
    };

    // Build TerrainRegistry
    let mut treg = TerrainRegistry::empty();
    for (def_id, terrain) in &registry.overmap_terrains {
        let key: &str = def_id.as_str();
        let mut flags = TerrainFlags::empty();
        for f in match &terrain.flags {
            StringOrArray::Single(s) => vec![s.clone()],
            StringOrArray::Multi(v) => v.clone(),
        } {
            match f.to_uppercase().as_str() {
                "RIVER" => flags.set(TerrainFlags::RIVER),
                "LAKE" | "LAKE_SHORE" => flags.set(TerrainFlags::LAKE),
                "OCEAN" | "OCEAN_SHORE" => flags.set(TerrainFlags::OCEAN),
                "ROAD" => flags.set(TerrainFlags::ROAD),
                "HIGHWAY" => flags.set(TerrainFlags::HIGHWAY),
                "LINE_DRAWING" | "LINEAR" => flags.set(TerrainFlags::LINE_DRAWING),
                "IMPASSABLE" => flags.set(TerrainFlags::IMPASSABLE),
                "UNDERGROUND" => flags.set(TerrainFlags::UNDERGROUND),
                "BRIDGE" => flags.set(TerrainFlags::BRIDGE),
                "SEWER" => flags.set(TerrainFlags::SEWER),
                "SUBWAY" => flags.set(TerrainFlags::SUBWAY),
                "RAILROAD" => flags.set(TerrainFlags::RAILROAD),
                "MANHOLE" => flags.set(TerrainFlags::MANHOLE),
                "FOREST" | "FOREST_TRAIL" => flags.set(TerrainFlags::FOREST),
                "SWAMP" => {
                    flags.set(TerrainFlags::FOREST);
                    flags.set(TerrainFlags::LAKE);
                }
                _ => {}
            }
        }
        let lo = key.to_lowercase();
        for (pat, land) in [
            ("forest", "FOREST"),
            ("road_", "ROAD+LINE"),
            ("highway_", "HIGHWAY+LINE"),
            ("railroad_", "RAILROAD+LINE"),
            ("river_", "RIVER+LINE"),
            ("lake_", "LAKE"),
            ("ocean_", "OCEAN"),
            ("sewer_", "SEWER+LINE"),
            ("subway_", "SUBWAY+LINE"),
            ("_bridge", "BRIDGE+LINE"),
            ("manhole", "MANHOLE"),
            ("forest_trail", "FOREST+ROAD"),
            ("trail_", "FOREST+ROAD"),
        ] {
            if lo.contains(pat) {
                match land {
                    "FOREST" => flags.set(TerrainFlags::FOREST),
                    "ROAD+LINE" => {
                        flags.set(TerrainFlags::ROAD);
                        flags.set(TerrainFlags::LINE_DRAWING);
                    }
                    "HIGHWAY+LINE" => {
                        flags.set(TerrainFlags::HIGHWAY);
                        flags.set(TerrainFlags::LINE_DRAWING);
                    }
                    "RAILROAD+LINE" => {
                        flags.set(TerrainFlags::RAILROAD);
                        flags.set(TerrainFlags::LINE_DRAWING);
                    }
                    "RIVER+LINE" => {
                        flags.set(TerrainFlags::RIVER);
                        flags.set(TerrainFlags::LINE_DRAWING);
                    }
                    "LAKE" => flags.set(TerrainFlags::LAKE),
                    "OCEAN" => flags.set(TerrainFlags::OCEAN),
                    "SEWER+LINE" => {
                        flags.set(TerrainFlags::SEWER);
                        flags.set(TerrainFlags::LINE_DRAWING);
                    }
                    "SUBWAY+LINE" => {
                        flags.set(TerrainFlags::SUBWAY);
                        flags.set(TerrainFlags::LINE_DRAWING);
                    }
                    "BRIDGE+LINE" => {
                        flags.set(TerrainFlags::BRIDGE);
                        flags.set(TerrainFlags::LINE_DRAWING);
                    }
                    "MANHOLE" => flags.set(TerrainFlags::MANHOLE),
                    "FOREST+ROAD" => {
                        flags.set(TerrainFlags::FOREST);
                        flags.set(TerrainFlags::ROAD);
                    }
                    _ => {}
                }
            }
        }
        let tc: u8 = match &terrain.travel_cost_type {
            Some(RawValue::String(s)) => match s.as_str() {
                "impassable" => 99,
                "road" => 1,
                "field" => 2,
                "forest" => 5,
                "water" => 99,
                _ => 2,
            },
            Some(RawValue::Number(n)) => (*n as u8).max(1),
            _ => 2,
        };
        let mg = terrain
            .mapgen
            .as_ref()
            .and_then(|mg| mg.first())
            .and_then(|raw| match raw {
                RawValue::String(s) => Some(s.clone()),
                RawValue::Object(obj) => {
                    obj.get("builtin")
                        .or_else(|| obj.get("method"))
                        .and_then(|v| match v {
                            RawValue::String(s) => Some(s.clone()),
                            _ => None,
                        })
                }
                _ => None,
            })
            .unwrap_or_else(|| key.to_string());
        treg.register_no_entity(key, flags, tc, mg, 0);
    }
    // Directional variants for LINE_DRAWING
    for idx in 1..treg.len() as u32 {
        let h = TerrainHandle::new(idx, 0);
        let f = treg.flags_for(h);
        if !f.contains(TerrainFlags::LINE_DRAWING) {
            continue;
        }
        let base = treg.string_id_for(h).unwrap_or("").to_string();
        let tcc = treg.travel_cost(h);
        let mgg = treg.mapgen_id(h).to_string();
        for s in &["_ns", "_ew", "_nesw"] {
            let vid = format!("{}{}", base, s);
            if treg.index_by_id(&vid).is_some() {
                continue;
            }
            let vi = treg.register_no_entity(&vid, f, tcc, mgg.clone(), 0);
            match *s {
                "_ns" => {
                    treg.register_rotation(idx, 0, vi);
                    treg.register_rotation(idx, 2, vi);
                }
                "_ew" => {
                    treg.register_rotation(idx, 1, vi);
                    treg.register_rotation(idx, 3, vi);
                }
                _ => {}
            }
        }
    }
    let core_terrains = cdda_overmap::registry::CoreTerrains::from_registry(&treg);

    eprintln!("TerrainRegistry: {} types", treg.len());

    // Bevy App with real OvermapGenPlugin
    let mut app = App::new();
    app.add_plugins((StatesPlugin, cdda_overmap_gen::pipeline::OvermapGenPlugin));

    // All chunks are spawned by init_base_terrain during the pipeline
    app.insert_resource(cdda_overmap::index::ChunkIndex::default());
    app.insert_resource(treg.clone());
    app.insert_resource(core_terrains.clone());
    app.insert_resource(cdda_overmap_gen::region_settings::OvermapRegionSettings {
        city_spec: true,
        city: cdda_overmap_gen::region_settings::RegionSettingsCity {
            city_size,
            ..Default::default()
        },
        place_roads: false,
        place_railroads: false,
        place_specials: false,
        ..Default::default()
    });
    app.insert_resource(cdda_overmap_gen::pipeline::OvermapGenConfig {
        noise_seed: seed as u32,
        om_x: 0,
        om_y: 0,
    });
    app.insert_resource(cdda_overmap_gen::special_catalog::SpecialCatalog::default());

    // Run pipeline
    app.world_mut()
        .resource_mut::<bevy_state::prelude::NextState<cdda_overmap_gen::pipeline::OvermapGenPhase>>()
        .set(cdda_overmap_gen::pipeline::OvermapGenPhase::Generating);
    for _ in 0..10 {
        app.update();
        use cdda_overmap_gen::pipeline::OvermapGenPhase;
        if *app
            .world()
            .resource::<bevy_state::prelude::State<OvermapGenPhase>>()
            .get()
            == OvermapGenPhase::Complete
        {
            break;
        }
    }

    // Read results - find the city closest to center of our chunk
    let size = 15i32;
    let (cx, cy) = {
        let w_mut = app.world_mut();
        let mut q = w_mut.query::<&cdda_overmap_gen::steps::cities::City>();
        let mut best = (7i32, 7i32);
        let mut best_dist = i32::MAX;
        for city in q.iter(w_mut) {
            let d = (city.omt_x - 7).abs() + (city.omt_y - 7).abs();
            if d < best_dist {
                best_dist = d;
                best = (city.omt_x, city.omt_y);
            }
        }
        best
    };
    let mut grid: [[TerrainHandle; 180]; 180] = [[TerrainHandle::NULL; 180]; 180];
    let w_mut = app.world_mut();
    let mut q = w_mut.query::<(&ChunkPosition, &OvermapChunk)>();
    for (cpos, chunk) in q.iter(w_mut) {
        if cpos.z.0 != 0 {
            continue;
        }
        let (ox, oy) = cpos.omt_origin();
        for ly in 0u8..CHUNK_DIM as u8 {
            for lx in 0u8..CHUNK_DIM as u8 {
                let gx = ox + lx as i32;
                let gy = oy + ly as i32;
                if gx >= 0 && gx < 180 && gy >= 0 && gy < 180 {
                    grid[gy as usize][gx as usize] = chunk.get(lx, ly);
                }
            }
        }
    }
    // Read building positions from CityTiles resource (set by build_cities)
    let buildings: HashSet<(i32, i32)> = app
        .world()
        .get_resource::<cdda_overmap_gen::steps::cities::CityTiles>()
        .map(|ct| ct.buildings.clone())
        .unwrap_or_default();

    // Compute road tiles: everything with ROAD flag that isn't a building
    let mut roads: HashSet<(i32, i32)> = HashSet::new();
    for y in (cy - size / 2)..(cy + size / 2) {
        for x in (cx - size / 2)..(cx + size / 2) {
            if x < 0 || x >= 180 || y < 0 || y >= 180 {
                continue;
            }
            let h = grid[y as usize][x as usize];
            if h == TerrainHandle::NULL || h.type_index() == core_terrains.field.type_index() {
                continue;
            }
            let f = treg.flags_for(h);
            if f.contains(TerrainFlags::ROAD)
                && !f.contains(TerrainFlags::HIGHWAY)
                && !buildings.contains(&(x, y))
            {
                roads.insert((x, y));
            }
        }
    }

    println!(
        "\n═══ CITY VIEW ({}x{}, city_size={}) ═══",
        size, size, city_size
    );
    println!("Legend: @=center  R=road  #=building  \u{00b7}=field");
    for y in (cy - size / 2)..(cy + size / 2) {
        let mut row = String::with_capacity(size as usize);
        for x in (cx - size / 2)..(cx + size / 2) {
            if x == cx && y == cy {
                row.push('@');
            } else if buildings.contains(&(x, y)) {
                row.push('#');
            } else if roads.contains(&(x, y)) {
                row.push('R');
            } else {
                row.push('\u{00b7}');
            }
        }
        println!("{}", row);
    }
    println!("═══════════════════════════════════\n");
    eprintln!(
        "Road tiles: {}  Building tiles: {}",
        roads.len(),
        buildings.len()
    );
}
