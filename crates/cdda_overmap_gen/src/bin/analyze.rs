//! # overmap-analyze — headless overmap generation diagnostics
use bevy_app::App;
use bevy_ecs::world::World;
use bevy_state::app::StatesPlugin;
use bevy_state::prelude::*;
use cdda_core_types::core::raw_defs::cdda_types::{RawValue, StringOrArray};
use cdda_data::loader::Loader;
use cdda_data::DefRegistry;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM, OMAP_DIM};
use cdda_overmap::registry::{TerrainFlags, TerrainHandle, TerrainRegistry};
use cdda_overmap_gen::connection_catalog::ConnectionCatalog;
use cdda_overmap_gen::mongroup_catalog::MongroupCatalog;
use cdda_overmap_gen::pipeline::{
    OvermapGenConfig, OvermapGenPhase, OvermapGenPlugin, DEFAULT_NOISE_SEED,
};
use cdda_overmap_gen::region_settings::OvermapRegionSettings;
use cdda_overmap_gen::special_catalog::SpecialCatalog;
use cdda_overmap_gen::steps::cities::City;

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(tracing::level_filters::LevelFilter::WARN.into())
                .from_env_lossy(),
        )
        .with_writer(std::io::stderr)
        .init();
    let args: Vec<String> = std::env::args().collect();
    let data_dir = args
        .get(1)
        .cloned()
        .unwrap_or_else(|| "data/core".to_string());
    let ascii_mode = args.iter().any(|a| a == "--ascii");
    let path = PathBuf::from(&data_dir);
    if !path.exists() {
        eprintln!("Error: {data_dir} not found");
        std::process::exit(1);
    }
    eprintln!("Loading JSON from {data_dir}...");
    let mut loader = Loader::new(vec![path]);
    loader.ingest_all();
    let registry = match loader.load() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Load errors: {}", e.len());
            std::process::exit(1);
        }
    };
    eprintln!(
        "Loaded: {} terrain, {} regions, {} connections, {} specials",
        registry.overmap_terrains.len(),
        registry.region_settings.len(),
        registry.overmap_connections.len(),
        registry.overmap_specials.len()
    );

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

        let idx = treg.register_no_entity(key, flags, tc, mg);
        match key {
            "forest" => treg.forest_index = idx,
            "forest_thick" => treg.forest_thick_index = idx,
            "forest_water" => treg.forest_water_index = idx,
            "road_ns" => treg.road_ns_index = idx,
            "road_ew" => treg.road_ew_index = idx,
            "road_nesw" => treg.road_nesw_index = idx,
            "lake_surface" => treg.lake_surface_index = idx,
            "lake_shore" => treg.lake_shore_index = idx,
            "ocean" => treg.ocean_index = idx,
            "river_center" => treg.river_center_index = idx,
            _ => {}
        }
    }
    // LINE_DRAWING directional variants
    let before = treg.len();
    let mut vc: Vec<(u32, String)> = Vec::new();
    for idx in 1..before as u32 {
        let h = TerrainHandle::new(idx, 0);
        let f = treg.flags_for(h);
        if !f.contains(TerrainFlags::LINE_DRAWING) {
            continue;
        }
        let base = treg.string_id_for(h).unwrap_or("").to_string();
        let tc = treg.travel_cost(h);
        let mg = treg.mapgen_id(h).to_string();
        for s in &["_ns", "_ew", "_nesw"] {
            let vid = format!("{}{}", base, s);
            if treg.index_by_id(&vid).is_some() {
                continue;
            }
            treg.register_no_entity(&vid, f, tc, mg.clone());
            vc.push((idx, vid));
        }
    }
    for (bi, vid) in &vc {
        let vi = treg.index_by_id(vid).unwrap();
        match vid.rsplit('_').next().unwrap_or("") {
            "ns" => {
                treg.register_rotation(*bi, 0, vi);
                treg.register_rotation(*bi, 2, vi);
            }
            "ew" => {
                treg.register_rotation(*bi, 1, vi);
                treg.register_rotation(*bi, 3, vi);
            }
            _ => {}
        }
    }
    if let Some(i) = treg.index_by_id("road_ns") {
        treg.road_ns_index = i;
    }
    if let Some(i) = treg.index_by_id("road_ew") {
        treg.road_ew_index = i;
    }
    if let Some(i) = treg.index_by_id("road_nesw") {
        treg.road_nesw_index = i;
    }
    // Find field: prefer terrain with "field" name and no flags, else first flagless terrain.
    if treg.field_index == 0 {
        if let Some(i) = treg.index_by_id("field") {
            treg.field_index = i;
        } else {
            let mut best = 0u32;
            for idx in 1..treg.len() as u32 {
                let f = treg.flags_for(TerrainHandle::new(idx, 0));
                let n = treg.string_id_for(TerrainHandle::new(idx, 0)).unwrap_or("");
                if !f.contains(
                    TerrainFlags::ROAD
                        | TerrainFlags::RIVER
                        | TerrainFlags::LAKE
                        | TerrainFlags::OCEAN
                        | TerrainFlags::FOREST
                        | TerrainFlags::IMPASSABLE
                        | TerrainFlags::UNDERGROUND
                        | TerrainFlags::HIGHWAY
                        | TerrainFlags::RAILROAD
                        | TerrainFlags::SEWER
                        | TerrainFlags::SUBWAY
                        | TerrainFlags::MANHOLE
                        | TerrainFlags::BRIDGE,
                ) {
                    best = idx;
                    if n.contains("field")
                        || n.contains("meadow")
                        || n.contains("grass")
                        || n == "."
                        || n == ","
                    {
                        break;
                    }
                }
            }
            if best > 0 {
                treg.field_index = best;
                eprintln!(
                    "Field: {} (idx {})",
                    treg.string_id_for(TerrainHandle::new(best, 0))
                        .unwrap_or("?"),
                    best
                );
            }
        }
    }
    // Enable river generation in region settings (default disables rivers).
    let mut region_settings = OvermapRegionSettings::default();
    region_settings.river_scale = 2;
    let connection_catalog = ConnectionCatalog::from_registry(&registry);
    eprintln!(
        "Registry: {} types, field={} (idx {})",
        treg.len(),
        treg.string_id_for(TerrainHandle::new(treg.field_index, 0))
            .unwrap_or("?"),
        treg.field_index
    );
    // Debug: check critical road/building terrain handles
    for id in &[
        "road_ns",
        "road_ew",
        "road_nesw",
        "road_nesw_manhole",
        "2storyModern01_first",
        "2storyModern01_first_north",
    ] {
        let found = treg.handle_by_id(id);
        eprintln!(
            "  terrain {:40} => {}",
            id,
            if found.is_some() { "FOUND" } else { "MISSING" }
        );
    }

    let mut app = App::new();
    app.add_plugins((StatesPlugin, OvermapGenPlugin));
    app.insert_resource(OvermapGenConfig {
        noise_seed: DEFAULT_NOISE_SEED,
        om_x: 0,
        om_y: 0,
        region_id: "default".into(),
    });
    app.insert_resource(SpecialCatalog::from_registry(&registry));
    app.insert_resource(connection_catalog.clone());
    app.insert_resource(MongroupCatalog::from_registry(&registry));

    app.insert_resource(cdda_overmap::index::ChunkIndex::default());
    let treg_for_stats = treg.clone();
    app.insert_resource(treg);
    app.insert_resource(region_settings);

    app.world_mut()
        .resource_mut::<NextState<OvermapGenPhase>>()
        .set(OvermapGenPhase::Generating);
    eprintln!("Generating overmap...");
    for _ in 0..10 {
        app.update();
        if *app.world().resource::<State<OvermapGenPhase>>().get() == OvermapGenPhase::Complete {
            break;
        }
    }
    eprintln!(
        "Phase: {:?}",
        app.world().resource::<State<OvermapGenPhase>>().get()
    );

    let w = app.world_mut();
    let special_count = w
        .query::<&cdda_overmap_gen::steps::specials::PlacedSpecial>()
        .iter(&*w)
        .count();
    let city_pos: Vec<(i32, i32)> = w
        .query::<&City>()
        .iter(&*w)
        .map(|c| (c.omt_x, c.omt_y))
        .collect();
    let treg_actual = w.resource::<TerrainRegistry>().clone();

    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut nulls = 0usize;
    let mut total = 0usize;
    let mut road_set: HashSet<(i32, i32)> = HashSet::new();
    let mut river_set: HashSet<(i32, i32)> = HashSet::new();
    let mut lake_set: HashSet<(i32, i32)> = HashSet::new();
    let mut ocean_set: HashSet<(i32, i32)> = HashSet::new();
    let mut forest_set: HashSet<(i32, i32)> = HashSet::new();
    let mut highway_set: HashSet<(i32, i32)> = HashSet::new();

    for (cpos, chunk) in w.query::<(&ChunkPosition, &OvermapChunk)>().iter(&*w) {
        if cpos.z.0 != 0 {
            continue;
        }
        let (ox, oy) = cpos.omt_origin();
        for ly in 0u8..CHUNK_DIM as u8 {
            for lx in 0u8..CHUNK_DIM as u8 {
                total += 1;
                let h = chunk.get(lx, ly);
                if h == TerrainHandle::NULL {
                    nulls += 1;
                    continue;
                }
                *counts
                    .entry(treg_actual.string_id_for(h).unwrap_or("?").to_string())
                    .or_default() += 1;
                let flags = treg_actual.flags_for(h);
                let wx = ox + lx as i32;
                let wy = oy + ly as i32;
                if flags.contains(TerrainFlags::ROAD) {
                    road_set.insert((wx, wy));
                }
                if flags.contains(TerrainFlags::RIVER) {
                    river_set.insert((wx, wy));
                }
                if flags.contains(TerrainFlags::LAKE) {
                    lake_set.insert((wx, wy));
                }
                if flags.contains(TerrainFlags::OCEAN) {
                    ocean_set.insert((wx, wy));
                }
                if flags.contains(TerrainFlags::FOREST) {
                    forest_set.insert((wx, wy));
                }
                if flags.contains(TerrainFlags::HIGHWAY) {
                    highway_set.insert((wx, wy));
                }
            }
        }
    }

    println!("\n═══ OVERMAP GENERATION REPORT ═══");
    println!("Total tiles:      {total}");
    println!("NULL tiles:       {nulls}");
    println!(
        "Cities:           {} ({})",
        city_pos.len(),
        if city_pos.is_empty() { "NONE!" } else { "OK" }
    );
    println!("Specials placed:  {special_count}");
    println!(
        "Road tiles:       {} ({:.1}%)",
        road_set.len(),
        road_set.len() as f64 / total as f64 * 100.0
    );
    println!(
        "Highway tiles:    {} ({:.1}%)",
        highway_set.len(),
        highway_set.len() as f64 / total as f64 * 100.0
    );
    println!(
        "River tiles:      {} ({:.1}%)",
        river_set.len(),
        river_set.len() as f64 / total as f64 * 100.0
    );
    println!(
        "Lake tiles:       {} ({:.1}%)",
        lake_set.len(),
        lake_set.len() as f64 / total as f64 * 100.0
    );
    println!(
        "Ocean tiles:      {} ({:.1}%)",
        ocean_set.len(),
        ocean_set.len() as f64 / total as f64 * 100.0
    );
    println!(
        "Forest tiles:     {} ({:.1}%)",
        forest_set.len(),
        forest_set.len() as f64 / total as f64 * 100.0
    );

    if city_pos.len() >= 2 {
        let ok = all_connected(&city_pos, &road_set);
        println!(
            "City connectivity: {}",
            if ok { "YES" } else { "NO — ORPHAN CITIES" }
        );
        if !ok {
            for &c in &city_pos {
                if !flood_fill(city_pos[0], &road_set).contains(&c) {
                    println!("  Orphan at ({}, {})", c.0, c.1);
                }
            }
        }
    } else {
        println!(
            "City connectivity: N/A ({})",
            if city_pos.len() == 1 {
                "single"
            } else {
                "none"
            }
        );
    }

    let mut issues: Vec<String> = Vec::new();
    if nulls > 0 {
        issues.push(format!("{nulls} NULL tiles"));
    }
    if river_set.is_empty() {
        issues.push("No rivers".into());
    }
    if forest_set.is_empty() {
        issues.push("No forests".into());
    }

    if !issues.is_empty() {
        println!("\n⚠ Issues:");
        for i in &issues {
            println!("  {i}");
        }
    } else {
        println!("\nNo issues.");
    }

    let mut sorted: Vec<_> = counts.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));
    println!("\nTop 20:");
    for (n, c) in sorted.iter().take(20) {
        println!(
            "  {n:<40} {c:>6} ({:.1}%)",
            *c as f64 / total as f64 * 100.0
        );
    }
    println!("═══════════════════════════════════\n");

    // ── ASCII map ────────────────────────────────────────────────────────
    if ascii_mode {
        // Build a grid of terrain types keyed by (x, y)
        let mut grid = vec![vec![' '; OMAP_DIM as usize]; OMAP_DIM as usize];
        for (cpos, chunk) in w.query::<(&ChunkPosition, &OvermapChunk)>().iter(&*w) {
            if cpos.z.0 != 0 {
                continue;
            }
            let (ox, oy) = cpos.omt_origin();
            for ly in 0u8..CHUNK_DIM as u8 {
                for lx in 0u8..CHUNK_DIM as u8 {
                    let h = chunk.get(lx, ly);
                    if h == TerrainHandle::NULL {
                        continue;
                    }
                    let id = treg_actual.string_id_for(h).unwrap_or("?");
                    let wx = (ox + lx as i32) as usize;
                    let wy = (oy + ly as i32) as usize;
                    if wx >= OMAP_DIM as usize || wy >= OMAP_DIM as usize {
                        continue;
                    }
                    let flags = treg_actual.flags_for(h);
                    let ch = if flags.contains(TerrainFlags::ROAD) {
                        if city_pos.iter().any(|&(cx, cy)| {
                            (cx as i32 - wx as i32).abs() <= 1 && (cy as i32 - wy as i32).abs() <= 1
                        }) {
                            '#'
                        } else {
                            'R'
                        }
                    } else if flags.contains(TerrainFlags::HIGHWAY) {
                        'H'
                    } else if flags.contains(TerrainFlags::RIVER) {
                        '~'
                    } else if flags.contains(TerrainFlags::LAKE) {
                        'l'
                    } else if flags.contains(TerrainFlags::OCEAN) {
                        'o'
                    } else if let Some(first) = id.chars().next() {
                        match first {
                            'f' => {
                                if id.contains("_thick") {
                                    'F'
                                } else if id.contains("_water") {
                                    'w'
                                } else {
                                    'f'
                                }
                            }
                            'r' => 'r',       // ravine
                            's' | 'S' => '■', // special/building
                            _ => '·',
                        }
                    } else {
                        '·'
                    };
                    grid[wy][wx] = ch;
                }
            }
        }

        // Mark city centers
        for &(cx, cy) in &city_pos {
            if cx >= 0 && cx < OMAP_DIM as i32 && cy >= 0 && cy < OMAP_DIM as i32 {
                grid[cy as usize][cx as usize] = '@';
            }
        }

        println!("\n═══ ASCII OVERMAP (z=0) ═══");
        println!("Legend: @=city  #=urban  R=road  f=forest  F=thick  w=water  ~=river  l=lake  o=ocean  r=ravine  ·=field");
        // Print row by row, skipping the first and last 5 border columns
        for y in 5..(OMAP_DIM as usize - 5) {
            let row: String = grid[y][5..(OMAP_DIM as usize - 5)].iter().collect();
            println!("{}", row);
        }
        println!("═══════════════════════════════════\n");
    }
}

fn flood_fill(start: (i32, i32), roads: &HashSet<(i32, i32)>) -> HashSet<(i32, i32)> {
    let mut v = HashSet::new();
    let mut q = VecDeque::new();
    q.push_back(start);
    v.insert(start);
    while let Some((cx, cy)) = q.pop_front() {
        for (nx, ny) in [(cx - 1, cy), (cx + 1, cy), (cx, cy - 1), (cx, cy + 1)] {
            if nx < 0
                || nx >= OMAP_DIM
                || ny < 0
                || ny >= OMAP_DIM
                || !roads.contains(&(nx, ny))
                || v.contains(&(nx, ny))
            {
                continue;
            }
            v.insert((nx, ny));
            q.push_back((nx, ny));
        }
    }
    v
}

fn all_connected(cities: &[(i32, i32)], roads: &HashSet<(i32, i32)>) -> bool {
    if cities.len() < 2 {
        return true;
    }
    let r = flood_fill(cities[0], roads);
    cities.iter().all(|c| r.contains(c))
}
