use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use bevy_app::App;
use bevy_ecs::prelude::*;
use bevy_state::prelude::*;
use bevy_state::app::StatesPlugin;
use cdda_data::loader::Loader;
use cdda_data::DefRegistry;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM, OMAP_DIM};
use cdda_overmap::registry::{TerrainFlags, TerrainHandle, TerrainRegistry};
use cdda_overmap_gen::pipeline::{OvermapGenConfig, OvermapGenPhase, OvermapGenPlugin};
use cdda_overmap_gen::region_settings::OvermapRegionSettings;
use cdda_overmap_gen::special_catalog::SpecialCatalog;
use cdda_overmap_gen::connection_catalog::ConnectionCatalog;
use cdda_overmap_gen::mongroup_catalog::MongroupCatalog;
use cdda_overmap_gen::steps::city_buildings::CityBuildingCatalog;
use cdda_overmap_gen::steps::cities::City;

fn main() {
    tracing_subscriber::fmt().with_env_filter(
        tracing_subscriber::EnvFilter::builder()
            .with_default_directive(tracing::level_filters::LevelFilter::WARN.into())
            .from_env_lossy(),
    ).with_writer(std::io::stderr).init();
    let data_dir = std::env::args().nth(1).unwrap_or_else(|| "data/core".to_string());
    let full_gen = std::env::args().any(|a| a == "--full");
    let path = PathBuf::from(&data_dir);
    if !path.exists() { eprintln!("Error: {data_dir} not found"); std::process::exit(1); }
    eprintln!("Loading JSON...");
    let mut loader = Loader::new(vec![path]); loader.ingest_all();
    let registry = match loader.load() { Ok(r) => r, Err(e) => { eprintln!("Load errors: {}", e.len()); std::process::exit(1); }};
    eprintln!("Loaded: {} items, {} terrain", registry.items.len(), registry.terrain.len());
    let mut treg = build_terrain_registry(&registry);
    let mut region_settings = build_region_settings(&registry);
    if full_gen { eprintln!("Full gen mode"); region_settings.city_size = 8; region_settings.place_roads = true; }
    // Set field_index from region default_oter
    if treg.field_index == 0 { if let Some(rs) = registry.region_settings.values().next() { for z_idx in [10,11,9] { if let Some(d) = rs.default_oter.get(z_idx) { if let Some(i) = treg.index_by_id(d) { treg.field_index = i; break; }}}}}}
    if treg.field_index == 0 { for idx in 1..treg.len() as u32 { let h=TerrainHandle::new(idx,0); let f=treg.flags_for(h); if !f.contains(TerrainFlags::ROAD|TerrainFlags::RIVER|TerrainFlags::LAKE|TerrainFlags::OCEAN|TerrainFlags::FOREST|TerrainFlags::IMPASSABLE|TerrainFlags::UNDERGROUND|TerrainFlags::BRIDGE|TerrainFlags::LINE_DRAWING) { treg.field_index = idx; eprintln!("Field fallback: {} (idx {})", treg.string_id_for(h).unwrap_or("?"), idx); break; }}}
    eprintln!("Registry: {} types, field={}", treg.len(), treg.string_id_for(TerrainHandle::new(treg.field_index,0)).unwrap_or("NULL"));
    let mut app = App::new(); app.add_plugins((StatesPlugin, OvermapGenPlugin));
    app.insert_resource(OvermapGenConfig { noise_seed: 1920237457, om_x: 0, om_y: 0, region_id: "default".into() });
    app.insert_resource(treg); app.insert_resource(region_settings);
    app.insert_resource(SpecialCatalog::from_registry(&registry));
    app.insert_resource(CityBuildingCatalog { buildings: registry.city_buildings.values().cloned().collect() });
    app.insert_resource(cdda_overmap::index::ChunkIndex::default());
    app.world_mut().resource_mut::<NextState<OvermapGenPhase>>().set(OvermapGenPhase::Generating);
    eprintln!("Generating...");
    for _ in 0..5 { app.update(); if app.world().resource::<State<OvermapGenPhase>>().get().clone() == OvermapGenPhase::Complete { break; }}
    let world = app.world(); let treg2 = world.resource::<TerrainRegistry>().clone();
    let mut counts: HashMap<String, usize> = HashMap::new(); let mut nulls = 0; let mut roads: HashSet<(i32,i32)> = HashSet::new(); let mut city_pos = Vec::new(); let mut total = 0usize;
    for city in world.query::<&City>().iter(world) { city_pos.push((city.omt_x, city.omt_y)); }
    for (cpos, chunk) in world.query::<(&ChunkPosition, &OvermapChunk)>().iter(world) { if cpos.z.0 != 0 { continue; } let (ox,oy) = cpos.omt_origin();
        for ly in 0u8..CHUNK_DIM as u8 { for lx in 0u8..CHUNK_DIM as u8 { total+=1; let h=chunk.get(lx,ly); if h==TerrainHandle::NULL { nulls+=1; continue; }
            *counts.entry(treg2.string_id_for(h).unwrap_or("?").to_string()).or_default()+=1;
            if treg2.flags_for(h).contains(TerrainFlags::ROAD) { roads.insert((ox+lx as i32, oy+ly as i32)); }}}}}
    println!("\n══════════════════════════════════"); println!("  OVERMAP REPORT"); println!("══════════════════════════════════");
    println!("Total tiles: {total}"); println!("NULL: {nulls}"); println!("Cities: {}", city_pos.len());
    println!("Road tiles: {} ({:.1}%)", roads.len(), roads.len() as f64/total as f64*100.0);
    if city_pos.len() >= 2 { println!("Connected: {}", if all_connected(&city_pos, &roads) { "YES" } else { "NO" }); }
    let mut sorted: Vec<_> = counts.into_iter().collect(); sorted.sort_by(|a,b| b.1.cmp(&a.1));
    println!("\nTop terrains:"); for (n,c) in sorted.iter().take(15) { println!("  {n:<40} {c:>8} ({:.1}%)", *c as f64/total as f64*100.0); }
    println!("══════════════════════════════════");
}

fn all_connected(cities: &[(i32,i32)], roads: &HashSet<(i32,i32)>) -> bool {
    if cities.len()<2 { return true; }
    let mut v = HashSet::new(); let mut q = VecDeque::new(); q.push_back(cities[0]); v.insert(cities[0]);
    while let Some((cx,cy)) = q.pop_front() { for (nx,ny) in [(cx-1,cy),(cx+1,cy),(cx,cy-1),(cx,cy+1)] { if nx<0||nx>=OMAP_DIM||ny<0||ny>=OMAP_DIM { continue; } if !roads.contains(&(nx,ny)) { continue; } if v.contains(&(nx,ny)) { continue; } v.insert((nx,ny)); q.push_back((nx,ny)); }}
    cities.iter().all(|c| v.contains(c))
}

fn build_region_settings(registry: &DefRegistry) -> OvermapRegionSettings {
    let mut s = OvermapRegionSettings::default();
    if let Some(rs) = registry.region_settings.values().next() {
        if let Some(c) = &rs.cities { if c == "no_cities" { s.city_size = 0; s.place_roads = false; }}
        if rs.rivers.is_none() { s.river_scale = 0; }
        if rs.lakes.is_none() { s.lake_size_min = usize::MAX; }
        if rs.ocean.is_none() { s.ocean_start = [None,None,None,None]; }
        if rs.forests.is_none() { s.forest_noise_threshold = 1.0; }
        if rs.ravines.is_none() { s.ravine_num = 0; }
    }
    s
}

fn build_terrain_registry(registry: &DefRegistry) -> TerrainRegistry {
    use cdda_core_types::core::raw_defs::cdda_types::StringOrArray;
    let mut t = TerrainRegistry::empty();
    for (did, ter) in &registry.overmap_terrains {
        let mut f = TerrainFlags::empty();
        for s in match &ter.flags { StringOrArray::Single(s)=>vec![s.clone()], StringOrArray::Multi(v)=>v.clone() } {
            match s.to_uppercase().as_str() {
                "RIVER"=>f.set(TerrainFlags::RIVER),"LAKE"=>f.set(TerrainFlags::LAKE),"LAKE_SHORE"=>f.set(TerrainFlags::LAKE),
                "OCEAN"=>f.set(TerrainFlags::OCEAN),"OCEAN_SHORE"=>f.set(TerrainFlags::OCEAN),"ROAD"=>f.set(TerrainFlags::ROAD),
                "HIGHWAY"=>f.set(TerrainFlags::HIGHWAY),"LINE_DRAWING"=>f.set(TerrainFlags::LINE_DRAWING),
                "IMPASSABLE"=>f.set(TerrainFlags::IMPASSABLE),"UNDERGROUND"=>f.set(TerrainFlags::UNDERGROUND),
                "BRIDGE"=>f.set(TerrainFlags::BRIDGE),"SEWER"=>f.set(TerrainFlags::SEWER),"SUBWAY"=>f.set(TerrainFlags::SUBWAY),
                "RAILROAD"=>f.set(TerrainFlags::RAILROAD),"MANHOLE"=>f.set(TerrainFlags::MANHOLE),"FOREST"=>f.set(TerrainFlags::FOREST),_=>{}
            }
        }
        let lo = did.as_str().to_lowercase();
        if lo.starts_with("forest")||lo.contains("_forest_") { f.set(TerrainFlags::FOREST); }
        if lo.starts_with("road_")||lo=="road" { f.set(TerrainFlags::ROAD); f.set(TerrainFlags::LINE_DRAWING); }
        if lo.starts_with("highway_")||lo.starts_with("hiway_") { f.set(TerrainFlags::HIGHWAY); f.set(TerrainFlags::LINE_DRAWING); }
        if lo.starts_with("railroad_")||lo.starts_with("rail_") { f.set(TerrainFlags::RAILROAD); f.set(TerrainFlags::LINE_DRAWING); }
        if lo.starts_with("river_")||lo=="river_center" { f.set(TerrainFlags::RIVER); f.set(TerrainFlags::LINE_DRAWING); }
        if lo.starts_with("lake_")||lo.contains("_lake_") { f.set(TerrainFlags::LAKE); }
        if lo.starts_with("ocean_")||lo.contains("_ocean_") { f.set(TerrainFlags::OCEAN); }
        if lo.starts_with("sewer_")||lo.contains("_sewer_") { f.set(TerrainFlags::SEWER); }
        if lo.starts_with("subway_")||lo.contains("_subway_") { f.set(TerrainFlags::SUBWAY); }
        if lo.contains("_bridge")||lo.ends_with("_bridge") { f.set(TerrainFlags::BRIDGE); }
        if lo.contains("manhole") { f.set(TerrainFlags::MANHOLE); }
        let idx = t.register_no_entity(did.as_str(), f, 2, did.as_str().to_string());
        match did.as_str() { "field"=>t.field_index=idx,"forest"=>t.forest_index=idx,"forest_thick"=>t.forest_thick_index=idx,"forest_water"=>t.forest_water_index=idx,"road_ns"=>t.road_ns_index=idx,"road_ew"=>t.road_ew_index=idx,"road_nesw"=>t.road_nesw_index=idx,"lake_surface"=>t.lake_surface_index=idx,"lake_shore"=>t.lake_shore_index=idx,"ocean"=>t.ocean_index=idx,"river_center"=>t.river_center_index=idx,_=>{}}
    }
    t
}
