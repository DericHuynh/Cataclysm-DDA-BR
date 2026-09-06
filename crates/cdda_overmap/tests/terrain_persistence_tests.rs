use cdda_core_types::core::coords::ZLevel;
use cdda_overmap::registry::TerrainRegistryReloadError;
use cdda_overmap::serial::{
    deserialize_chunk, deserialize_chunks, serialize_chunk, serialize_chunks,
    TERRAIN_FORMAT_VERSION,
};
use cdda_overmap::{
    ChunkPosition, OvermapChunk, TerrainFlags, TerrainHandle, TerrainRegistry, CHUNK_SIZE,
};
use std::io::ErrorKind;

fn registry(ids: &[&str]) -> TerrainRegistry {
    let mut registry = TerrainRegistry::empty();
    for id in ids {
        registry.register_no_entity(id, TerrainFlags::empty(), 2, format!("{id}_mapgen"), 0);
    }
    registry
}

fn pos() -> ChunkPosition {
    ChunkPosition {
        om_x: -3,
        om_y: 7,
        z: ZLevel::new(-10),
        chunk_x: 5,
        chunk_y: 2,
    }
}

fn save(chunk: &OvermapChunk, registry: &TerrainRegistry) -> Vec<u8> {
    let mut bytes = Vec::new();
    serialize_chunk(chunk, &pos(), registry, &mut bytes).unwrap();
    bytes
}

#[test]
fn load_reordered_registry_preserves_ids_and_all_rotation_bytes() {
    let a = registry(&["forest", "road"]);
    let b = registry(&["road", "forest"]);
    assert_ne!(a.handle_by_id("forest"), b.handle_by_id("forest"));
    let forest_a = a.handle_by_id("forest").unwrap();
    let mut chunk = OvermapChunk::new_filled(forest_a);
    for rotation in 0..=255u8 {
        chunk.terrain[rotation as usize] = TerrainHandle::new(forest_a.type_index(), rotation);
    }
    chunk.terrain[256] = a.handle_by_id("road").unwrap();
    chunk.terrain[257] = TerrainHandle::NULL;
    let bytes = save(&chunk, &a);
    let (loaded_pos, loaded) = deserialize_chunk(&mut bytes.as_slice(), &b).unwrap();
    assert_eq!(loaded_pos, pos());
    for (&old, &new) in chunk.terrain.iter().zip(loaded.terrain.iter()) {
        assert_eq!(a.string_id_for(old), b.string_id_for(new));
        assert_eq!(old.rotation(), new.rotation());
    }
    assert_eq!(loaded.terrain[257], TerrainHandle::NULL);
    // Stable file bytes are independent of the owning runtime's slot assignment.
    assert_eq!(bytes, save(&loaded, &b));
}

#[test]
fn multi_chunk_roundtrip_resolves_each_palette() {
    let a = registry(&["forest", "road"]);
    let b = registry(&["road", "forest"]);
    let forest = OvermapChunk::new_filled(a.handle_by_id("forest").unwrap());
    let road = OvermapChunk::new_filled(TerrainHandle::new(a.index_by_id("road").unwrap(), 7));
    let mut second = pos();
    second.z = ZLevel::new(10);
    let mut bytes = Vec::new();
    serialize_chunks(&[(pos(), &forest), (second, &road)], &a, &mut bytes).unwrap();
    let loaded = deserialize_chunks(&mut bytes.as_slice(), &b).unwrap();
    assert_eq!(loaded.len(), 2);
    assert_eq!(loaded[0].0, pos());
    assert_eq!(loaded[1].0, second);
    assert_eq!(loaded[0].1.terrain[0], b.handle_by_id("forest").unwrap());
    assert_eq!(
        loaded[1].1.terrain[899],
        TerrainHandle::new(b.index_by_id("road").unwrap(), 7)
    );

    // Failure in a later chunk rejects the whole batch.
    let missing = registry(&["forest"]);
    assert_eq!(
        deserialize_chunks(&mut bytes.as_slice(), &missing)
            .err()
            .unwrap()
            .kind(),
        ErrorKind::InvalidData
    );
}

#[test]
fn null_and_empty_streams_need_no_registered_terrain() {
    let registry = TerrainRegistry::empty();
    let bytes = save(&OvermapChunk::new_filled(TerrainHandle::NULL), &registry);
    // Prefix, position, empty palette, 900 (u16, u8) cells.
    assert_eq!(bytes.len(), 6 + 11 + 2 + CHUNK_SIZE * 3);
    let (_, chunk) = deserialize_chunk(&mut bytes.as_slice(), &registry).unwrap();
    assert!(chunk.terrain.iter().all(|handle| handle.is_null()));
    let mut bytes = Vec::new();
    serialize_chunks(&[], &registry, &mut bytes).unwrap();
    assert!(deserialize_chunks(&mut bytes.as_slice(), &registry)
        .unwrap()
        .is_empty());
}

#[test]
fn unknown_id_is_not_reinterpreted_as_the_same_numeric_slot() {
    let a = registry(&["forest"]);
    let b = registry(&["road"]);
    assert_eq!(a.handle_by_id("forest"), b.handle_by_id("road"));
    let bytes = save(
        &OvermapChunk::new_filled(a.handle_by_id("forest").unwrap()),
        &a,
    );
    let error = deserialize_chunk(&mut bytes.as_slice(), &b).err().unwrap();
    assert_eq!(error.kind(), ErrorKind::InvalidData);
    assert!(error.to_string().contains("forest"));
}

#[test]
fn legacy_and_unknown_versions_are_rejected() {
    let registry = registry(&["forest"]);
    let bytes = save(
        &OvermapChunk::new_filled(registry.handle_by_id("forest").unwrap()),
        &registry,
    );
    for version in [0, TERRAIN_FORMAT_VERSION + 1, u16::MAX] {
        let mut corrupt = bytes.clone();
        corrupt[4..6].copy_from_slice(&version.to_le_bytes());
        assert_eq!(
            deserialize_chunk(&mut corrupt.as_slice(), &registry)
                .err()
                .unwrap()
                .kind(),
            ErrorKind::InvalidData
        );
    }
    let legacy = vec![0u8; 11 + CHUNK_SIZE * 4];
    assert_eq!(
        deserialize_chunk(&mut legacy.as_slice(), &registry)
            .err()
            .unwrap()
            .kind(),
        ErrorKind::InvalidData
    );
    let mut multiple = Vec::new();
    serialize_chunks(&[], &registry, &mut multiple).unwrap();
    multiple[4..6].copy_from_slice(&2u16.to_le_bytes());
    assert_eq!(
        deserialize_chunks(&mut multiple.as_slice(), &registry)
            .err()
            .unwrap()
            .kind(),
        ErrorKind::InvalidData
    );
}

#[test]
fn malformed_palette_coordinates_and_cells_are_rejected() {
    let registry = registry(&["forest"]);
    let bytes = save(
        &OvermapChunk::new_filled(registry.handle_by_id("forest").unwrap()),
        &registry,
    );
    // Prefix (6), position (11), palette count (2), ID length (2), "forest" (6).
    let cell_offset = 27;
    for (offset, replacement) in [
        (0, vec![b'X']),                            // magic
        (6, vec![6]),                               // chunk x
        (7, vec![255]),                             // chunk y
        (8, vec![21]),                              // z index
        (17, 901u16.to_le_bytes().to_vec()),        // oversized palette
        (19, 0u16.to_le_bytes().to_vec()),          // empty ID
        (19, 4097u16.to_le_bytes().to_vec()),       // oversized ID
        (21, vec![255]),                            // invalid UTF-8
        (cell_offset, 2u16.to_le_bytes().to_vec()), // palette index past end
        (cell_offset, vec![0, 0, 1]),               // rotated NULL
    ] {
        let mut corrupt = bytes.clone();
        corrupt[offset..offset + replacement.len()].copy_from_slice(&replacement);
        assert_eq!(
            deserialize_chunk(&mut corrupt.as_slice(), &registry)
                .err()
                .unwrap()
                .kind(),
            ErrorKind::InvalidData,
            "offset {offset}"
        );
    }
    let mut duplicate = bytes.clone();
    duplicate[17..19].copy_from_slice(&2u16.to_le_bytes());
    duplicate.splice(
        cell_offset..cell_offset,
        bytes[19..cell_offset].iter().copied(),
    );
    assert_eq!(
        deserialize_chunk(&mut duplicate.as_slice(), &registry)
            .err()
            .unwrap()
            .kind(),
        ErrorKind::InvalidData
    );
    for length in [0, 5, 18, 22, bytes.len() - 1] {
        assert_eq!(
            deserialize_chunk(&mut &bytes[..length], &registry)
                .err()
                .unwrap()
                .kind(),
            ErrorKind::UnexpectedEof
        );
    }
    let mut absurd_count = b"OMTS".to_vec();
    absurd_count.extend(TERRAIN_FORMAT_VERSION.to_le_bytes());
    absurd_count.extend(u32::MAX.to_le_bytes());
    assert_eq!(
        deserialize_chunks(&mut absurd_count.as_slice(), &registry)
            .err()
            .unwrap()
            .kind(),
        ErrorKind::UnexpectedEof
    );
}

#[test]
fn invalid_runtime_handles_and_coordinates_cannot_be_saved() {
    let registry = registry(&["forest"]);
    for handle in [TerrainHandle::new(99, 0), TerrainHandle::new(0, 1)] {
        let mut bytes = Vec::new();
        assert_eq!(
            serialize_chunk(
                &OvermapChunk::new_filled(handle),
                &pos(),
                &registry,
                &mut bytes
            )
            .err()
            .unwrap()
            .kind(),
            ErrorKind::InvalidData
        );
        assert!(bytes.is_empty());
    }
    let mut invalid_pos = pos();
    invalid_pos.chunk_x = 6;
    assert!(serialize_chunk(
        &OvermapChunk::new_filled(TerrainHandle::NULL),
        &invalid_pos,
        &registry,
        &mut Vec::new()
    )
    .is_err());
}

#[test]
fn duplicate_registration_updates_properties_without_reassigning_identity() {
    let mut registry = registry(&["forest", "road"]);
    let old = registry.handle_by_id("forest").unwrap();
    let index = registry.register_no_entity(
        "forest",
        TerrainFlags::from_bits(TerrainFlags::FOREST),
        9,
        "new_mapgen".into(),
        0,
    );
    assert_eq!(index, old.type_index());
    assert_eq!(registry.len(), 3);
    assert_eq!(registry.string_id_for(old), Some("forest"));
    assert_eq!(registry.travel_cost(old), 9);
    assert_eq!(registry.mapgen_id(old), "new_mapgen");
}

#[test]
fn rebuild_preserves_extant_handles_and_remaps_families_and_rotation_links() {
    let mut old = registry(&["forest", "road", "road_ew"]);
    let forest_family = old.get_or_create_family("forest");
    let road_family = old.get_or_create_family("road");
    let forest = old.handle_by_id("forest").unwrap();
    let road = old.handle_by_id("road").unwrap();
    let east = old.handle_by_id("road_ew").unwrap();
    let chunk = OvermapChunk::new_filled(TerrainHandle::new(forest.type_index(), 255));
    let mut fresh = registry(&["lake", "road_ew", "road", "forest"]);
    let fresh_road_family = fresh.get_or_create_family("road");
    let fresh_forest_family = fresh.get_or_create_family("forest");
    assert_ne!(forest_family, fresh_forest_family);
    fresh.register_no_entity(
        "forest",
        TerrainFlags::from_bits(TerrainFlags::FOREST),
        8,
        "updated".into(),
        fresh_forest_family,
    );
    fresh.register_no_entity(
        "road",
        TerrainFlags::from_bits(TerrainFlags::ROAD),
        1,
        "road".into(),
        fresh_road_family,
    );
    fresh.register_rotation(
        fresh.index_by_id("road").unwrap(),
        1,
        fresh.index_by_id("road_ew").unwrap(),
    );
    old.rebuild_from(&fresh).unwrap();
    assert_eq!(old.handle_by_id("forest"), Some(forest));
    assert_eq!(old.handle_by_id("road"), Some(road));
    assert_eq!(old.handle_by_id("road_ew"), Some(east));
    assert_eq!(old.index_by_id("lake"), Some(4));
    assert_eq!(old.family_id(forest), forest_family);
    assert_eq!(old.family_id(road), road_family);
    assert_eq!(old.rotate(road, 1), east);
    assert_eq!(old.travel_cost(chunk.terrain[0]), 8);
    assert_eq!(old.mapgen_id(forest), "updated");
    assert_eq!(old.string_id_for(chunk.terrain[0]), Some("forest"));
    assert_eq!(chunk.terrain[0].rotation(), 255);
    let bytes = save(&chunk, &old);
    let (_, loaded) = deserialize_chunk(&mut bytes.as_slice(), &fresh).unwrap();
    assert_eq!(fresh.string_id_for(loaded.terrain[0]), Some("forest"));
}

#[test]
fn invalid_rebuild_links_roll_back_properties_and_new_slots() {
    let mut old = registry(&["forest"]);
    let forest = old.handle_by_id("forest").unwrap();
    let mut fresh = registry(&["lake", "forest"]);
    fresh.register_no_entity("forest", TerrainFlags::empty(), 9, "changed".into(), 99);
    assert_eq!(
        old.rebuild_from(&fresh),
        Err(TerrainRegistryReloadError::InvalidFamily("forest".into()))
    );
    fresh.register_no_entity("forest", TerrainFlags::empty(), 9, "changed".into(), 0);
    fresh.register_rotation(fresh.index_by_id("forest").unwrap(), 1, 99);
    assert_eq!(
        old.rebuild_from(&fresh),
        Err(TerrainRegistryReloadError::InvalidRotation("forest".into()))
    );
    assert_eq!(old.len(), 2);
    assert_eq!(old.handle_by_id("forest"), Some(forest));
    assert_eq!(old.travel_cost(forest), 2);
    assert_eq!(old.mapgen_id(forest), "forest_mapgen");
    assert!(old.handle_by_id("lake").is_none());
}

#[test]
fn removal_rejects_rebuild_without_changing_any_existing_slots() {
    let mut old = registry(&["forest", "road"]);
    let forest = old.handle_by_id("forest").unwrap();
    let road = old.handle_by_id("road").unwrap();
    let fresh = registry(&["lake", "road"]);
    assert_eq!(
        old.rebuild_from(&fresh),
        Err(TerrainRegistryReloadError::RemovedId("forest".into()))
    );
    assert_eq!(old.len(), 3);
    assert_eq!(old.handle_by_id("forest"), Some(forest));
    assert_eq!(old.handle_by_id("road"), Some(road));
    assert!(old.handle_by_id("lake").is_none());
    assert_eq!(old.mapgen_id(forest), "forest_mapgen");
}
