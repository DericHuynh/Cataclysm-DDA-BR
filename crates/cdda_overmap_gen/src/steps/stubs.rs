//! Terrain registry utilities — directional variant generation.
//!
//! Contains `generate_shore_variants` which creates the river/highway/railroad
//! shore tiles needed by the overmap generation pipeline.
//!
//! Called by `startup.rs::build_terrain_registry` and the headless binary.

use cdda_overmap::registry::{TerrainFlags, TerrainHandle, TerrainRegistry};

/// After LINE_DRAWING _ns/_ew/_nesw variants are created, generate
/// the shore, center, and single-direction variants needed by the
/// river, highway, railroad, sewer, and subway systems.
///
/// CDDA naming convention:
/// - Single directions: `_north`, `_east`, `_south`, `_west`
/// - Corner tiles: `_ne`, `_nw`, `_se`, `_sw`
/// - Center/intersection: `_center` (synonym for the 4-way variant)
pub fn generate_shore_variants(treg: &mut TerrainRegistry) {
    let count = treg.len();
    let mut to_create: Vec<(u32, String, u8, String, TerrainFlags)> = Vec::new();

    for idx in 1..count as u32 {
        let base = TerrainHandle::new(idx, 0);
        let flags = treg.flags_for(base);
        if !flags.contains(TerrainFlags::LINE_DRAWING) {
            continue;
        }
        let base_id = treg.string_id_for(base).unwrap_or("").to_string();
        let tc = treg.travel_cost(base);
        let mg = treg.mapgen_id(base).to_string();

        // Skip already-generated variants
        if base_id.ends_with("_ns")
            || base_id.ends_with("_ew")
            || base_id.ends_with("_nesw")
            || base_id.ends_with("_north")
            || base_id.ends_with("_east")
            || base_id.ends_with("_south")
            || base_id.ends_with("_west")
            || base_id.ends_with("_ne")
            || base_id.ends_with("_nw")
            || base_id.ends_with("_se")
            || base_id.ends_with("_sw")
            || base_id.ends_with("_center")
        {
            continue;
        }

        let shore_names: &[&str] = &[
            "_north", "_east", "_south", "_west", "_ne", "_nw", "_se", "_sw", "_center",
        ];
        for suffix in shore_names {
            let vid = format!("{}{}", base_id, suffix);
            if treg.index_by_id(&vid).is_some() {
                continue;
            }
            treg.register_no_entity(&vid, flags, tc, mg.clone());
            to_create.push((idx, vid, tc, mg.clone(), flags));
        }
    }

    // Set up directional rotations
    for (base_idx, vid, _tc, _mg, _flags) in &to_create {
        if let Some(vi) = treg.index_by_id(vid) {
            match vid.rsplit('_').next().unwrap_or("") {
                "north" => treg.register_rotation(*base_idx, 0, vi),
                "east" => treg.register_rotation(*base_idx, 1, vi),
                "south" => treg.register_rotation(*base_idx, 2, vi),
                "west" => treg.register_rotation(*base_idx, 3, vi),
                _ => {}
            }
        }
    }
}
