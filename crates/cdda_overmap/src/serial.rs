//! Versioned terrain persistence using stable definition IDs, not runtime slots.
//!
//! All integers are little-endian. A chunk record is `OMTC`, version:u16,
//! chunk_x:u8, chunk_y:u8, z_index:u8, om_x:i32, om_y:i32, palette_count:u16,
//! then that many (byte_length:u16, UTF-8 ID) entries, followed by 900
//! (palette_index:u16, rotation:u8) cells. Palette index 0 is implicit NULL;
//! IDs occupy indices 1..=palette_count. IDs must be nonempty and at most
//! 4096 bytes; the palette has at most 900 entries. NULL has rotation 0.
//!
//! Multi-chunk streams are `OMTS`, version:u16, count:u32, then chunk records.
//! Unknown versions, legacy raw-handle files, unknown IDs and malformed records
//! are errors, not fallback terrain. Each reader consumes one record/stream;
//! callers may frame or concatenate them. Submap tile content is not included.

use crate::chunk::{ChunkPosition, OvermapChunk, CHUNK_SIZE};
use crate::registry::{TerrainHandle, TerrainRegistry};
use cdda_core_types::core::coords::ZLevel;
use std::collections::{HashMap, HashSet};
use std::io;

pub const TERRAIN_FORMAT_VERSION: u16 = 1;
const CHUNK_MAGIC: &[u8; 4] = b"OMTC";
const CHUNKS_MAGIC: &[u8; 4] = b"OMTS";
const MAX_ID_BYTES: usize = 4096;

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

fn read_u16(reader: &mut impl io::Read) -> io::Result<u16> {
    let mut bytes = [0; 2];
    reader.read_exact(&mut bytes)?;
    Ok(u16::from_le_bytes(bytes))
}

fn write_prefix(writer: &mut impl io::Write, magic: &[u8; 4]) -> io::Result<()> {
    writer.write_all(magic)?;
    writer.write_all(&TERRAIN_FORMAT_VERSION.to_le_bytes())
}

fn read_prefix(reader: &mut impl io::Read, magic: &[u8; 4]) -> io::Result<()> {
    let mut actual = [0; 4];
    reader.read_exact(&mut actual)?;
    if &actual != magic {
        return Err(invalid(
            "unrecognized terrain format (legacy raw handles are unsupported)",
        ));
    }
    let version = read_u16(reader)?;
    if version != TERRAIN_FORMAT_VERSION {
        return Err(invalid(format!(
            "unsupported terrain format version {version}"
        )));
    }
    Ok(())
}

/// Serialize one chunk, resolving runtime handles through its owning registry.
/// Validation finishes before any record bytes are written (I/O may still fail).
pub fn serialize_chunk(
    chunk: &OvermapChunk,
    pos: &ChunkPosition,
    registry: &TerrainRegistry,
    writer: &mut impl io::Write,
) -> io::Result<()> {
    if pos.chunk_x >= 6 || pos.chunk_y >= 6 || !(-10..=10).contains(&pos.z.0) {
        return Err(invalid("invalid chunk coordinates"));
    }
    let mut palette = Vec::new();
    let mut indices = HashMap::new();
    let mut cells = Vec::with_capacity(CHUNK_SIZE);
    for &handle in chunk.terrain.as_ref() {
        if handle.type_index() == 0 {
            if !handle.is_null() {
                return Err(invalid("NULL terrain must have rotation 0"));
            }
            cells.push((0u16, 0u8));
            continue;
        }
        let index = if let Some(&index) = indices.get(&handle.type_index()) {
            index
        } else {
            let id = registry.string_id_for(handle).ok_or_else(|| {
                invalid(format!(
                    "unregistered terrain handle {}",
                    handle.type_index()
                ))
            })?;
            if id.is_empty() || id.len() > MAX_ID_BYTES {
                return Err(invalid("terrain ID length is outside format limits"));
            }
            palette.push(id);
            let index = palette.len() as u16;
            indices.insert(handle.type_index(), index);
            index
        };
        cells.push((index, handle.rotation()));
    }

    write_prefix(writer, CHUNK_MAGIC)?;
    writer.write_all(&[pos.chunk_x, pos.chunk_y, z_to_index(pos.z)])?;
    writer.write_all(&pos.om_x.to_le_bytes())?;
    writer.write_all(&pos.om_y.to_le_bytes())?;
    writer.write_all(&(palette.len() as u16).to_le_bytes())?;
    for id in palette {
        writer.write_all(&(id.len() as u16).to_le_bytes())?;
        writer.write_all(id.as_bytes())?;
    }
    for (index, rotation) in cells {
        writer.write_all(&index.to_le_bytes())?;
        writer.write_all(&[rotation])?;
    }
    Ok(())
}

/// Resolve every palette ID against the destination registry before returning
/// a chunk. Rotation bytes are preserved independently of registry rotation links.
pub fn deserialize_chunk(
    reader: &mut impl io::Read,
    registry: &TerrainRegistry,
) -> io::Result<(ChunkPosition, OvermapChunk)> {
    read_prefix(reader, CHUNK_MAGIC)?;
    let mut header = [0; 11];
    reader.read_exact(&mut header)?;
    if header[0] >= 6 || header[1] >= 6 || header[2] > 20 {
        return Err(invalid("invalid chunk coordinates"));
    }
    let pos = ChunkPosition {
        chunk_x: header[0],
        chunk_y: header[1],
        z: z_from_index(header[2]),
        om_x: i32::from_le_bytes(header[3..7].try_into().unwrap()),
        om_y: i32::from_le_bytes(header[7..11].try_into().unwrap()),
    };
    let count = read_u16(reader)? as usize;
    if count > CHUNK_SIZE {
        return Err(invalid("terrain palette exceeds chunk size"));
    }
    let mut palette = Vec::with_capacity(count + 1);
    palette.push(TerrainHandle::NULL);
    let mut seen = HashSet::new();
    for _ in 0..count {
        let len = read_u16(reader)? as usize;
        if len == 0 || len > MAX_ID_BYTES {
            return Err(invalid("terrain ID length is outside format limits"));
        }
        let mut bytes = vec![0; len];
        reader.read_exact(&mut bytes)?;
        let id = String::from_utf8(bytes).map_err(|_| invalid("terrain ID is not UTF-8"))?;
        let handle = registry
            .handle_by_id(&id)
            .ok_or_else(|| invalid(format!("unknown terrain ID {id:?}")))?;
        if !seen.insert(handle) {
            return Err(invalid(format!("duplicate terrain palette ID {id:?}")));
        }
        palette.push(handle);
    }
    let mut chunk = OvermapChunk::new_filled(TerrainHandle::NULL);
    for slot in chunk.terrain.iter_mut() {
        let index = read_u16(reader)? as usize;
        let mut rotation = [0];
        reader.read_exact(&mut rotation)?;
        let base = palette
            .get(index)
            .ok_or_else(|| invalid("invalid terrain palette index"))?;
        if index == 0 && rotation[0] != 0 {
            return Err(invalid("NULL terrain must have rotation 0"));
        }
        *slot = TerrainHandle::new(base.type_index(), rotation[0]);
    }
    Ok((pos, chunk))
}

/// Serialize a framed sequence of chunks using their owning registry.
pub fn serialize_chunks(
    chunks: &[(ChunkPosition, &OvermapChunk)],
    registry: &TerrainRegistry,
    writer: &mut impl io::Write,
) -> io::Result<()> {
    let count = u32::try_from(chunks.len()).map_err(|_| invalid("too many chunks"))?;
    write_prefix(writer, CHUNKS_MAGIC)?;
    writer.write_all(&count.to_le_bytes())?;
    for (pos, chunk) in chunks {
        serialize_chunk(chunk, pos, registry, writer)?;
    }
    Ok(())
}

/// Load a framed sequence atomically: no partially decoded chunks escape on error.
pub fn deserialize_chunks(
    reader: &mut impl io::Read,
    registry: &TerrainRegistry,
) -> io::Result<Vec<(ChunkPosition, OvermapChunk)>> {
    read_prefix(reader, CHUNKS_MAGIC)?;
    let mut bytes = [0; 4];
    reader.read_exact(&mut bytes)?;
    let count = u32::from_le_bytes(bytes);
    // Never reserve memory using an untrusted file's chunk count.
    let mut chunks = Vec::new();
    for _ in 0..count {
        chunks.push(deserialize_chunk(reader, registry)?);
    }
    Ok(chunks)
}

/// Convert a valid storage index (0 = z=-10, 20 = z=+10) to a ZLevel.
#[inline]
pub fn z_from_index(idx: u8) -> ZLevel {
    ZLevel::new(idx as i8 - 10)
}

/// Convert a ZLevel to its storage index.
#[inline]
pub fn z_to_index(z: ZLevel) -> u8 {
    (z.0 + 10) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::CHUNK_DIM;

    #[test]
    fn chunk_dim_is_30() {
        assert_eq!(CHUNK_DIM, 30);
        assert_eq!(CHUNK_SIZE, 900);
    }

    #[test]
    fn submap_origin_correct() {
        let mut pos = ChunkPosition {
            om_x: 0,
            om_y: 0,
            z: ZLevel::new(0),
            chunk_x: 0,
            chunk_y: 0,
        };
        assert_eq!(pos.submap_origin(), (0, 0));
        pos.chunk_x = 1;
        assert_eq!(pos.omt_origin(), (30, 0));
        assert_eq!(pos.submap_origin(), (60, 0));
        pos.om_x = 1;
        pos.chunk_x = 0;
        assert_eq!(pos.omt_origin(), (180, 0));
        assert_eq!(pos.submap_origin(), (360, 0));
    }

    #[test]
    fn z_index_roundtrip() {
        for z in -10i8..=10 {
            assert_eq!(z_from_index(z_to_index(ZLevel::new(z))).0, z);
        }
    }
}
