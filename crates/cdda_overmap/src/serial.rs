//! Binary chunk serialization — compact, zero-allocation-per-row.
//!
//! # Wire format
//!
//! Each chunk serializes as:
//!
//! ```text
//! Header (11 bytes):
//!   chunk_x  : u8       — column within overmap (0..6)
//!   chunk_y  : u8       — row within overmap (0..6)
//!   z_index  : u8       — z-level encoded as index 0..21 (z=-10 → 0)
//!   om_x     : i32 LE   — overmap x in world grid
//!   om_y     : i32 LE   — overmap y in world grid
//!
//! Terrain (900 × u32 LE = 3600 bytes):
//!   One TerrainHandle per OMT slot, row-major, 30×30 = 900 slots.
//!
//! Total: 3611 bytes per chunk.
//! ```
//!
//! Multi-chunk files prepend a u32 LE count, then concatenate chunk records.
//!
//! # Submap coordinates
//!
//! The wire format stores OMT-level terrain handles only. Submap tile data
//! (the 12×12 map tiles within each submap) is stored separately and is not
//! part of this format. To recover the world submap origin of a deserialized
//! chunk, use `ChunkPosition::submap_origin()`.

use crate::chunk::{ChunkPosition, OvermapChunk, CHUNK_SIZE};
use crate::registry::TerrainHandle;
use cdda_core_types::core::coords::ZLevel;
use std::io;

/// Serialize a single chunk to a binary writer.
pub fn serialize_chunk(
    chunk: &OvermapChunk,
    pos: &ChunkPosition,
    writer: &mut impl io::Write,
) -> io::Result<()> {
    // Header: 11 bytes
    writer.write_all(&[pos.chunk_x, pos.chunk_y, z_to_index(pos.z)])?;
    writer.write_all(&pos.om_x.to_le_bytes())?;
    writer.write_all(&pos.om_y.to_le_bytes())?;

    // Terrain: 900 × u32 LE = 3600 bytes
    for &handle in chunk.terrain.as_ref() {
        writer.write_all(&handle.0.to_le_bytes())?;
    }
    Ok(())
}

/// Deserialize a single chunk from a binary reader.
pub fn deserialize_chunk(reader: &mut impl io::Read) -> io::Result<(ChunkPosition, OvermapChunk)> {
    let mut header = [0u8; 11];
    reader.read_exact(&mut header)?;

    let chunk_x = header[0];
    let chunk_y = header[1];
    let z       = z_from_index(header[2]);
    let om_x    = i32::from_le_bytes(header[3..7].try_into().unwrap());
    let om_y    = i32::from_le_bytes(header[7..11].try_into().unwrap());

    let mut terrain = Box::new([TerrainHandle::NULL; CHUNK_SIZE]);
    for slot in terrain.iter_mut() {
        let mut bytes = [0u8; 4];
        reader.read_exact(&mut bytes)?;
        slot.0 = u32::from_le_bytes(bytes);
    }

    Ok((
        ChunkPosition { om_x, om_y, z, chunk_x, chunk_y },
        OvermapChunk { terrain },
    ))
}

/// Serialize multiple chunks into a single buffer (overmap save file).
///
/// Prepends a u32 LE chunk count, then each chunk record in order.
pub fn serialize_chunks(
    chunks: &[(ChunkPosition, &OvermapChunk)],
    writer: &mut impl io::Write,
) -> io::Result<()> {
    writer.write_all(&(chunks.len() as u32).to_le_bytes())?;
    for (pos, chunk) in chunks {
        serialize_chunk(chunk, pos, writer)?;
    }
    Ok(())
}

/// Deserialize multiple chunks from a buffer.
pub fn deserialize_chunks(
    reader: &mut impl io::Read,
) -> io::Result<Vec<(ChunkPosition, OvermapChunk)>> {
    let mut count_bytes = [0u8; 4];
    reader.read_exact(&mut count_bytes)?;
    let count = u32::from_le_bytes(count_bytes) as usize;

    let mut chunks = Vec::with_capacity(count);
    for _ in 0..count {
        chunks.push(deserialize_chunk(reader)?);
    }
    Ok(chunks)
}

// ---------------------------------------------------------------------------
// Z-level encoding
// ---------------------------------------------------------------------------

/// Convert a storage index (0 = z=-10, 20 = z=+10) to a ZLevel.
#[inline]
pub fn z_from_index(idx: u8) -> ZLevel {
    ZLevel::new(idx as i8 - 10)
}

/// Convert a ZLevel to its storage index.
#[inline]
pub fn z_to_index(z: ZLevel) -> u8 {
    (z.0 + 10) as u8
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::CHUNK_DIM;
    use crate::registry::TerrainHandle;

    fn make_pos(om_x: i32, om_y: i32, z: i8, cx: u8, cy: u8) -> ChunkPosition {
        ChunkPosition {
            om_x,
            om_y,
            z: ZLevel::new(z),
            chunk_x: cx,
            chunk_y: cy,
        }
    }

    #[test]
    fn roundtrip_single_chunk() {
        let pos = make_pos(3, -2, 0, 1, 4);
        let mut chunk = OvermapChunk::new_filled(TerrainHandle::new(42, 0));
        chunk.set(0, 0, TerrainHandle::new(7, 1));
        chunk.set(29, 29, TerrainHandle::new(99, 3));

        let mut buf = Vec::new();
        serialize_chunk(&chunk, &pos, &mut buf).unwrap();

        // Expected size: 11 header + 900 * 4 terrain = 3611 bytes
        assert_eq!(buf.len(), 11 + CHUNK_SIZE * 4);

        let (pos2, chunk2) = deserialize_chunk(&mut buf.as_slice()).unwrap();
        assert_eq!(pos2.om_x, 3);
        assert_eq!(pos2.om_y, -2);
        assert_eq!(pos2.z.0, 0);
        assert_eq!(pos2.chunk_x, 1);
        assert_eq!(pos2.chunk_y, 4);
        assert_eq!(chunk2.get(0, 0), TerrainHandle::new(7, 1));
        assert_eq!(chunk2.get(29, 29), TerrainHandle::new(99, 3));
        // All other tiles should be fill value 42/0
        assert_eq!(chunk2.get(1, 0), TerrainHandle::new(42, 0));
    }

    #[test]
    fn roundtrip_multiple_chunks() {
        let pos1 = make_pos(0, 0, 0, 0, 0);
        let pos2 = make_pos(0, 0, 1, 0, 0);
        let chunk1 = OvermapChunk::new_filled(TerrainHandle::new(1, 0));
        let chunk2 = OvermapChunk::new_filled(TerrainHandle::new(2, 0));

        let mut buf = Vec::new();
        serialize_chunks(&[(pos1, &chunk1), (pos2, &chunk2)], &mut buf).unwrap();

        let chunks = deserialize_chunks(&mut buf.as_slice()).unwrap();
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].0.z.0, 0);
        assert_eq!(chunks[1].0.z.0, 1);
        assert_eq!(chunks[0].1.get(0, 0), TerrainHandle::new(1, 0));
        assert_eq!(chunks[1].1.get(0, 0), TerrainHandle::new(2, 0));
    }

    #[test]
    fn z_index_roundtrip() {
        for z in -10i8..=10 {
            let level = ZLevel::new(z);
            let idx = z_to_index(level);
            assert_eq!(z_from_index(idx).0, z);
        }
    }

    #[test]
    fn chunk_dim_is_30() {
        assert_eq!(CHUNK_DIM, 30);
        assert_eq!(CHUNK_SIZE, 900);
    }

    #[test]
    fn submap_origin_correct() {
        // Chunk (0,0) of overmap (0,0) at z=0 should have submap origin (0,0).
        let pos = make_pos(0, 0, 0, 0, 0);
        assert_eq!(pos.submap_origin(), (0, 0));

        // Chunk (1,0) of overmap (0,0): OMT origin = (30, 0), submap = (60, 0).
        let pos2 = make_pos(0, 0, 0, 1, 0);
        assert_eq!(pos2.omt_origin(), (30, 0));
        assert_eq!(pos2.submap_origin(), (60, 0));

        // Overmap (1,0), chunk (0,0): OMT origin = (180, 0), submap = (360, 0).
        let pos3 = make_pos(1, 0, 0, 0, 0);
        assert_eq!(pos3.omt_origin(), (180, 0));
        assert_eq!(pos3.submap_origin(), (360, 0));
    }
}
