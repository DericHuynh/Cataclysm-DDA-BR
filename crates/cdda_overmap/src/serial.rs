//! Binary chunk serialization — compact, zero-allocation-per-row.
//!
//! Each chunk serializes as:
//! ```text
//! [chunk_x: u8][chunk_y: u8][z_index: u8][om_x: i32 LE][om_y: i32 LE]
//! [terrain: [u32 LE; 1024]]     ← 4096 bytes
//! [total: 4111 bytes]
//! ```

use crate::chunk::{ChunkPosition, OvermapChunk, CHUNK_SIZE};
use crate::registry::TerrainHandle;
use cdda_core_types::core::coords::ZLevel;
use std::io;

/// Serialize a chunk to a binary writer.
pub fn serialize_chunk(
    chunk: &OvermapChunk,
    pos: &ChunkPosition,
    writer: &mut impl io::Write,
) -> io::Result<()> {
    // Header: 11 bytes
    writer.write_all(&[pos.chunk_x, pos.chunk_y, z_to_index(pos.z)])?;
    writer.write_all(&pos.om_x.to_le_bytes())?;
    writer.write_all(&pos.om_y.to_le_bytes())?;

    // Terrain data: 1024 × u32 = 4096 bytes (little-endian)
    for &handle in chunk.terrain.as_ref() {
        writer.write_all(&handle.0.to_le_bytes())?;
    }
    Ok(())
}

/// Deserialize a chunk from a binary reader.
pub fn deserialize_chunk(reader: &mut impl io::Read) -> io::Result<(ChunkPosition, OvermapChunk)> {
    let mut header = [0u8; 11];
    reader.read_exact(&mut header)?;

    let chunk_x = header[0];
    let chunk_y = header[1];
    let z = z_from_index(header[2]);
    let om_x = i32::from_le_bytes(header[3..7].try_into().unwrap());
    let om_y = i32::from_le_bytes(header[7..11].try_into().unwrap());

    let mut terrain = Box::new([TerrainHandle::NULL; CHUNK_SIZE]);
    for slot in terrain.iter_mut() {
        let mut bytes = [0u8; 4];
        reader.read_exact(&mut bytes)?;
        slot.0 = u32::from_le_bytes(bytes);
    }

    Ok((
        ChunkPosition {
            om_x,
            om_y,
            z,
            chunk_x,
            chunk_y,
        },
        OvermapChunk { terrain },
    ))
}

/// Serialize multiple chunks into a single buffer (overmap save file).
pub fn serialize_chunks(
    chunks: &[(ChunkPosition, &OvermapChunk)],
    writer: &mut impl io::Write,
) -> io::Result<()> {
    let count = chunks.len() as u32;
    writer.write_all(&count.to_le_bytes())?;
    for (pos, chunk) in chunks {
        serialize_chunk(chunk, pos, writer)?;
    }
    Ok(())
}

/// Deserialize multiple chunks from a buffer.
pub fn deserialize_chunks(reader: &mut impl io::Read) -> io::Result<Vec<(ChunkPosition, OvermapChunk)>> {
    let mut count_bytes = [0u8; 4];
    reader.read_exact(&mut count_bytes)?;
    let count = u32::from_le_bytes(count_bytes) as usize;

    let mut chunks = Vec::with_capacity(count);
    for _ in 0..count {
        chunks.push(deserialize_chunk(reader)?);
    }
    Ok(chunks)
}

/// Convert storage index (0 = z=-10) to ZLevel.
pub fn z_from_index(idx: u8) -> ZLevel {
    ZLevel::new(idx as i8 - 10)
}

/// Convert ZLevel to storage index.
pub fn z_to_index(z: ZLevel) -> u8 {
    (z.0 + 10) as u8
}
