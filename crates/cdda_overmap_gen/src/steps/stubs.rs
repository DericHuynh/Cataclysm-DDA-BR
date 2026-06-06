//! **Stubs** — shore variant generation for line-drawing terrains.
//!
//! After the data-loading phase registers base terrains and their
//! `_ns` / `_ew` / `_nesw` rotation variants (the "line-drawing" set),
//! this module generates the remaining directional variants:
//!
//! - **Single-edge variants**: `_north`, `_east`, `_south`, `_west`
//! - **Corner variants**: `_ne`, `_nw`, `_se`, `_sw`
//! - **Center variant**: `_center`
//!
//! Each variant is a new terrain entry in the [`TerrainRegistry`] with the same
//! flags and travel cost as the base terrain, and a rotation mapping is
//! registered so that [`TerrainRegistry::rotate`] returns the correct variant.
//!
//! ## Rotation conventions (CDDA line-drawing)
//!
//! | Rotation | Variant  | Visual             |
//! |----------|----------|--------------------|
//! | 0        | `_north` | edge to the north  |
//! | 1        | `_east`  | edge to the east   |
//! | 2        | `_south` | edge to the south  |
//! | 3        | `_west`  | edge to the west   |
//! | 4        | `_ne`    | corner north-east  |
//! | 5        | `_se`    | corner south-east  |
//! | 6        | `_sw`    | corner south-west  |
//! | 7        | `_nw`    | corner north-west  |
//! | —        | `_center`| isolated (no edge) |

use cdda_overmap::registry::{TerrainFlags, TerrainHandle, TerrainRegistry};
use tracing::{debug, info};

// ---------------------------------------------------------------------------
// Variant definitions
// ---------------------------------------------------------------------------

/// A single directional variant to register.
struct VariantDef {
    /// Suffix appended to the base terrain id (e.g. `"north"` → `"t_shore_north"`).
    suffix: &'static str,
    /// Rotation index for [`TerrainRegistry::register_rotation`].
    /// `None` for the center variant (no rotation mapping).
    rotation: Option<u8>,
}

/// All directional variants generated for each line-drawing terrain.
const DIRECTIONAL_VARIANTS: &[VariantDef] = &[
    // Single-edge
    VariantDef {
        suffix: "north",
        rotation: Some(0),
    },
    VariantDef {
        suffix: "east",
        rotation: Some(1),
    },
    VariantDef {
        suffix: "south",
        rotation: Some(2),
    },
    VariantDef {
        suffix: "west",
        rotation: Some(3),
    },
    // Corners
    VariantDef {
        suffix: "ne",
        rotation: Some(4),
    },
    VariantDef {
        suffix: "se",
        rotation: Some(5),
    },
    VariantDef {
        suffix: "sw",
        rotation: Some(6),
    },
    VariantDef {
        suffix: "nw",
        rotation: Some(7),
    },
    // Center (isolated)
    VariantDef {
        suffix: "center",
        rotation: None,
    },
];

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Generate shore/center/single-direction rotation variants for every terrain
/// in the registry that has the [`TerrainFlags::LINE_DRAWING`] flag.
///
/// For each such terrain `t_X`, this creates:
/// - `t_X_north`, `t_X_east`, `t_X_south`, `t_X_west`
/// - `t_X_ne`, `t_X_nw`, `t_X_se`, `t_X_sw`
/// - `t_X_center`
///
/// Each variant shares the base terrain's flags and travel cost, and a
/// rotation mapping is registered so that `registry.rotate(base_handle, r)`
/// returns the correct variant handle.
///
/// # Idempotency
///
/// If a variant already exists (same string id), [`TerrainRegistry::register_no_entity`]
/// will create a duplicate entry. Callers should ensure this is invoked exactly
/// once during data loading, before generation begins.
pub fn generate_shore_variants(treg: &mut TerrainRegistry) {
    let total = treg.len();
    if total <= 1 {
        // Only index-0 (NULL) exists — nothing to do.
        debug!("generate_shore_variants: registry is empty (only NULL present), skipping");
        return;
    }

    // --- collect LINE_DRAWING base terrains ---------------------------------------
    // We scan indices first to avoid holding `&self` across `&mut self` calls.
    struct BaseInfo {
        index: u32,
        string_id: String,
        mapgen_id: String,
        flags: TerrainFlags,
        travel_cost: u8,
    }

    let mut bases: Vec<BaseInfo> = Vec::new();

    for i in 1..total {
        let handle = TerrainHandle::new(i as u32, 0);
        let flags = treg.flags_for(handle);
        if flags.contains(TerrainFlags::LINE_DRAWING) {
            if let Some(string_id) = treg.string_id_for(handle) {
                bases.push(BaseInfo {
                    index: i as u32,
                    string_id: string_id.to_string(),
                    mapgen_id: treg.mapgen_id(handle).to_string(),
                    flags,
                    travel_cost: treg.travel_cost(handle),
                });
            }
        }
    }

    if bases.is_empty() {
        debug!("generate_shore_variants: no LINE_DRAWING terrains found, skipping");
        return;
    }

    info!(
        count = bases.len(),
        "generate_shore_variants: found {} LINE_DRAWING base terrains",
        bases.len(),
    );

    // --- register variants for each base -----------------------------------------
    let variant_count = DIRECTIONAL_VARIANTS.len();
    let mut total_created: usize = 0;

    for base in &bases {
        for vdef in DIRECTIONAL_VARIANTS {
            let variant_string_id = format!("{}_{}", base.string_id, vdef.suffix);
            let variant_mapgen_id = format!("{}_{}", base.mapgen_id, vdef.suffix);

            let variant_index = treg.register_no_entity(
                &variant_string_id,
                base.flags,
                base.travel_cost,
                variant_mapgen_id,
                0,
            );

            if let Some(rotation) = vdef.rotation {
                treg.register_rotation(base.index, rotation, variant_index);
            }
            // The `_center` variant (rotation == None) is not rotation-mapped.
            // It is a standalone terrain reached by direct handle lookup, not
            // via `rotate()`.

            total_created += 1;
        }
    }

    info!(
        base_count = bases.len(),
        variants_per_base = variant_count,
        total_variants = total_created,
        "generate_shore_variants: registered {} directional variants",
        total_created,
    );
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Helper to create LINE_DRAWING flags (private field — must use `empty()` + `set()`).
    fn line_drawing_flags() -> TerrainFlags {
        let mut f = TerrainFlags::empty();
        f.set(TerrainFlags::LINE_DRAWING);
        f
    }

    /// Build a minimal registry with a few LINE_DRAWING terrains.
    fn build_test_registry() -> TerrainRegistry {
        let mut treg = TerrainRegistry::empty();

        // Register a line-drawing terrain (like "t_shore")
        treg.register_no_entity("t_shore", line_drawing_flags(), 4, "t_shore".to_string(), 0);

        // Register another (like "t_ocean_shore")
        treg.register_no_entity(
            "t_ocean_shore",
            line_drawing_flags(),
            4,
            "t_ocean_shore".to_string(),
            0,
        );

        // Register a non-line-drawing terrain (should be ignored)
        treg.register_no_entity(
            "t_grass",
            TerrainFlags::empty(),
            2,
            "t_grass".to_string(),
            0,
        );

        treg
    }

    #[test]
    fn creates_all_nine_variants_per_base() {
        let mut treg = build_test_registry();

        // Find the initial count of line-drawing terrains
        let pre_count = {
            let mut c = 0;
            for i in 1..treg.len() {
                let h = TerrainHandle::new(i as u32, 0);
                if treg.flags_for(h).contains(TerrainFlags::LINE_DRAWING) {
                    c += 1;
                }
            }
            c
        };
        assert_eq!(pre_count, 2, "should have 2 LINE_DRAWING bases");

        generate_shore_variants(&mut treg);

        // 2 bases × 9 variants = 18 new entries + 3 original = 21 total (excluding NULL)
        // But we also have t_grass, so: 1 (NULL) + 2 (bases) + 1 (grass) + 18 (variants) = 22
        assert_eq!(
            treg.len(),
            22,
            "1 NULL + 2 bases + 1 grass + 18 variants = 22"
        );

        // Verify each base got all 9 variants
        for base_id in &["t_shore", "t_ocean_shore"] {
            for vdef in DIRECTIONAL_VARIANTS {
                let variant_id = format!("{}_{}", base_id, vdef.suffix);
                let idx = treg.index_by_id(&variant_id);
                assert!(idx.is_some(), "missing variant: {variant_id}");
                // Variant should have same flags as base
                let variant_handle = treg.handle_by_id(&variant_id).unwrap();
                assert!(
                    treg.flags_for(variant_handle)
                        .contains(TerrainFlags::LINE_DRAWING),
                    "{variant_id} should have LINE_DRAWING flag"
                );
            }
        }
    }

    #[test]
    fn non_line_drawing_ignored() {
        let mut treg = build_test_registry();
        let grass_idx = treg.index_by_id("t_grass").unwrap();

        generate_shore_variants(&mut treg);

        // t_grass should NOT have any variant registered
        assert!(treg.index_by_id("t_grass_north").is_none());
        assert!(treg.index_by_id("t_grass_center").is_none());

        // t_grass should still be at the same index
        assert_eq!(treg.index_by_id("t_grass"), Some(grass_idx));
    }

    #[test]
    fn rotation_mappings_are_registered() {
        let mut treg = build_test_registry();
        generate_shore_variants(&mut treg);

        let shore_idx = treg.index_by_id("t_shore").unwrap();
        let shore_north_idx = treg.index_by_id("t_shore_north").unwrap();
        let shore_east_idx = treg.index_by_id("t_shore_east").unwrap();
        let shore_center_idx = treg.index_by_id("t_shore_center").unwrap();

        let shore_handle = TerrainHandle::new(shore_idx, 0);

        // Rotation 0 → _north
        let rotated = treg.rotate(shore_handle, 0);
        assert_eq!(rotated.type_index(), shore_north_idx);

        // Rotation 1 → _east
        let rotated = treg.rotate(shore_handle, 1);
        assert_eq!(rotated.type_index(), shore_east_idx);

        // The center variant should NOT be reachable via rotate()
        // (it's a standalone terrain)
        let rotated = treg.rotate(shore_handle, 2);
        assert_ne!(rotated.type_index(), shore_center_idx);
    }

    #[test]
    fn idempotent_on_empty_registry() {
        let mut treg = TerrainRegistry::empty();
        assert_eq!(treg.len(), 1); // only NULL at index 0
        generate_shore_variants(&mut treg);
        assert_eq!(treg.len(), 1, "empty registry should stay empty");
    }

    #[test]
    fn all_variant_ids_are_unique() {
        let mut treg = build_test_registry();
        generate_shore_variants(&mut treg);

        let mut seen = HashSet::new();
        for i in 1..treg.len() {
            let handle = TerrainHandle::new(i as u32, 0);
            if let Some(id) = treg.string_id_for(handle) {
                assert!(seen.insert(id.to_string()), "duplicate string id: {id}");
            }
        }
    }
}
