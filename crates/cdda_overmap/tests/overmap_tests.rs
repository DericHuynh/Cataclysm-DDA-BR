//! Integration tests for cdda_overmap.
//!
//! Ports CDDA master's overmap test patterns to the Rust ECS architecture.

use cdda_core_types::core::coords::ZLevel;
use cdda_overmap::chunk::{ChunkPosition, OvermapChunk, CHUNK_DIM, CHUNK_SIZE};
use cdda_overmap::registry::{TerrainFlags, TerrainHandle, TerrainRegistry};

// ===========================================================================
// TerrainHandle tests
// ===========================================================================

#[test]
fn terrain_handle_null_is_zero() {
    assert_eq!(TerrainHandle::NULL, TerrainHandle(0));
}

#[test]
fn terrain_handle_encode_decode() {
    let h = TerrainHandle::new(42, 3);
    assert_eq!(h.type_index(), 42);
    assert_eq!(h.rotation(), 3);
    assert_eq!(h.base(), TerrainHandle::new(42, 0));
}

#[test]
fn terrain_handle_round_trip_all_rotations() {
    for idx in 0..100 {
        for rot in 0..8 {
            let h = TerrainHandle::new(idx, rot);
            assert_eq!(h.type_index(), idx);
            assert_eq!(h.rotation(), rot);
        }
    }
}

#[test]
fn terrain_handle_default_is_null() {
    let h = TerrainHandle::default();
    assert_eq!(h, TerrainHandle::NULL);
}

#[test]
fn terrain_handle_base_clears_rotation() {
    let h = TerrainHandle::new(99, 7);
    let b = h.base();
    assert_eq!(b.type_index(), 99);
    assert_eq!(b.rotation(), 0);
    assert_ne!(b, h);
}

// ===========================================================================
// TerrainRegistry tests
// ===========================================================================

#[test]
fn registry_empty_has_only_null() {
    let reg = TerrainRegistry::empty();
    assert_eq!(reg.len(), 1);
    assert!(reg.handle_by_id("forest").is_none());
}

#[test]
fn registry_register_and_lookup() {
    let mut reg = TerrainRegistry::empty();
    let mut flags = TerrainFlags::empty();
    flags.set(TerrainFlags::FOREST);

    let idx = reg.register_no_entity("forest", flags, 3, "forest_mapgen".to_string(), 0);
    assert!(idx > 0);

    let handle = reg.handle_by_id("forest").unwrap();
    assert_eq!(handle.type_index(), idx);

    let retrieved = reg.flags_for(handle);
    assert!(retrieved.contains(TerrainFlags::FOREST));

    assert_eq!(reg.travel_cost(handle), 3);
    assert_eq!(reg.mapgen_id(handle), "forest_mapgen");
}

#[test]
fn registry_flags_intersect() {
    let mut flags = TerrainFlags::empty();
    flags.set(TerrainFlags::RIVER);
    assert!(flags.contains(TerrainFlags::RIVER));
    assert!(!flags.contains(TerrainFlags::ROAD));

    let mut flags2 = TerrainFlags::empty();
    flags2.set(TerrainFlags::RIVER);
    flags2.set(TerrainFlags::LAKE);

    assert!(flags.intersects(flags2));
}

#[test]
fn registry_handle_by_id_when_not_found() {
    let reg = TerrainRegistry::empty();
    assert!(reg.handle_by_id("nonexistent_terrain").is_none());
    assert!(reg.index_by_id("nonexistent_terrain").is_none());
}

#[test]
fn registry_rotate_unregistered_is_noop() {
    let reg = TerrainRegistry::empty();
    let h = reg.rotate(TerrainHandle::NULL, 3);
    assert_eq!(h, TerrainHandle::NULL);
}

// ===========================================================================
// Chunk tests
// ===========================================================================

#[test]
fn chunk_new_filled_all_same() {
    let h = TerrainHandle::new(5, 0);
    let chunk = OvermapChunk::new_filled(h);
    for ly in 0u8..CHUNK_DIM as u8 {
        for lx in 0u8..CHUNK_DIM as u8 {
            assert_eq!(chunk.get(lx, ly), h);
        }
    }
}

#[test]
fn chunk_set_and_get() {
    let mut chunk = OvermapChunk::new_filled(TerrainHandle::NULL);
    let h = TerrainHandle::new(42, 1);
    chunk.set(10, 15, h);
    assert_eq!(chunk.get(10, 15), h);
    assert_eq!(chunk.get(0, 0), TerrainHandle::NULL);
}

#[test]
fn chunk_fill_overwrites() {
    let mut chunk = OvermapChunk::new_filled(TerrainHandle::new(1, 0));
    let h2 = TerrainHandle::new(2, 0);
    chunk.fill(h2);
    assert_eq!(chunk.get(29, 29), h2);
    assert_eq!(chunk.get(0, 0), h2);
}

#[test]
fn chunk_iter_tiles_count() {
    let chunk = OvermapChunk::new_filled(TerrainHandle::NULL);
    let count = chunk.iter_tiles().count();
    assert_eq!(count, CHUNK_SIZE);
}

#[test]
fn chunk_set_only_changes_one_tile() {
    let mut chunk = OvermapChunk::new_filled(TerrainHandle::new(1, 0));
    chunk.set(15, 15, TerrainHandle::new(99, 0));
    assert_eq!(chunk.get(15, 15), TerrainHandle::new(99, 0));
    assert_eq!(chunk.get(14, 15), TerrainHandle::new(1, 0));
    assert_eq!(chunk.get(15, 14), TerrainHandle::new(1, 0));
}

// ===========================================================================
// ChunkPosition tests
// ===========================================================================

#[test]
fn chunk_position_to_key() {
    let pos = ChunkPosition {
        om_x: 0,
        om_y: 0,
        z: ZLevel::new(0),
        chunk_x: 3,
        chunk_y: 4,
    };
    let pos2 = ChunkPosition {
        om_x: 0,
        om_y: 0,
        z: ZLevel::new(0),
        chunk_x: 3,
        chunk_y: 4,
    };
    assert_eq!(pos.to_key(), pos2.to_key());
}

#[test]
fn chunk_position_different_keys() {
    let a = ChunkPosition {
        om_x: 0,
        om_y: 0,
        z: ZLevel::new(0),
        chunk_x: 0,
        chunk_y: 0,
    };
    let b = ChunkPosition {
        om_x: 0,
        om_y: 0,
        z: ZLevel::new(0),
        chunk_x: 1,
        chunk_y: 0,
    };
    assert_ne!(a.to_key(), b.to_key());
}

#[test]
fn chunk_position_omt_origin() {
    let pos = ChunkPosition {
        om_x: 0,
        om_y: 0,
        z: ZLevel::new(0),
        chunk_x: 1,
        chunk_y: 2,
    };
    let (ox, oy) = pos.omt_origin();
    // chunk_x * CHUNK_DIM(30) = 30, chunk_y * 30 = 60
    assert_eq!(ox, 30);
    assert_eq!(oy, 60);
}

#[test]
fn chunk_position_omt_origin_nonzero_overmap() {
    let pos = ChunkPosition {
        om_x: 2,
        om_y: 3,
        z: ZLevel::new(0),
        chunk_x: 3,
        chunk_y: 4,
    };
    let (ox, oy) = pos.omt_origin();
    // om_x * 180 + chunk_x * 30 = 360 + 90 = 450
    assert_eq!(ox, 2 * 180 + 3 * 30);
    // om_y * 180 + chunk_y * 30 = 540 + 120 = 660
    assert_eq!(oy, 3 * 180 + 4 * 30);
}

#[test]
fn chunk_position_omt_origin_wraps_correctly() {
    // Chunk (5, 5) in overmap (0, 0) — at (150, 150) with CHUNK_DIM=30
    let pos = ChunkPosition {
        om_x: 0,
        om_y: 0,
        z: ZLevel::new(0),
        chunk_x: 5,
        chunk_y: 5,
    };
    let (ox, oy) = pos.omt_origin();
    assert_eq!(ox, 150);
    assert_eq!(oy, 150);
}

// ===========================================================================
// Scent trace tests (placeholder — ported from C++ set_and_get_overmap_scents)
// ===========================================================================

/// In C++, scent traces default to `calendar::before_time_starts`.
/// This test documents the expected behavior for the Rust port.
/// Scent data will be stored as a component on overmap entities in Bevy ECS.
#[test]
fn test_scent_trace_default_is_invalid() {
    // Placeholder: when scent traces are implemented, the default
    // creation_time should be equivalent to before_time_starts / invalid.
    //
    // In Bevy ECS, scent would be queried as:
    //   Query<&ScentTrace, With<OvermapPosition>>
    // with a default value indicating "no scent."
}

/// In C++, calling `set_scent` stores `creation_time` and `initial_strength`.
/// This test documents the expected API.
#[test]
fn test_set_and_get_scent() {
    // Placeholder: when scent traces are implemented:
    // - set_scent(pos, scent_trace { creation_time, initial_strength })
    // - scent_at(pos).creation_time == creation_time
    // - scent_at(pos).initial_strength == 90
}

// ===========================================================================
// OmDirection rotation tests (ported from C++ direction tests)
// ===========================================================================

/// Turning left 4 times should return to the starting direction.
#[test]
fn test_om_direction_rotation_composition() {
    use cdda_overmap::direction::OmDirection;

    // Turning left 4 times should return to start.
    for dir in &[
        OmDirection::North,
        OmDirection::East,
        OmDirection::South,
        OmDirection::West,
    ] {
        let mut d = *dir;
        for _ in 0..4 {
            d = d.turn_left();
        }
        assert_eq!(d, *dir, "turn_left x4 should return to start from {dir:?}");
    }

    // Turning right then left should return to start.
    for dir in &[
        OmDirection::North,
        OmDirection::East,
        OmDirection::South,
        OmDirection::West,
    ] {
        assert_eq!(
            dir.turn_right().turn_left(),
            *dir,
            "turn_right().turn_left() should be identity from {dir:?}"
        );
    }

    // Opposite of opposite is self.
    for dir in &[
        OmDirection::North,
        OmDirection::East,
        OmDirection::South,
        OmDirection::West,
    ] {
        assert_eq!(
            dir.opposite().opposite(),
            *dir,
            "opposite().opposite() should be identity from {dir:?}"
        );
    }

    // Invalid stays invalid under all rotations.
    assert_eq!(OmDirection::Invalid.turn_left(), OmDirection::Invalid);
    assert_eq!(OmDirection::Invalid.turn_right(), OmDirection::Invalid);
    assert_eq!(OmDirection::Invalid.opposite(), OmDirection::Invalid);
}
