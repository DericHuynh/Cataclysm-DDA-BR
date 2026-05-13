//! Placeholder systems for complex generation steps not yet fully ported.
//!
//! Each function logs a warning and returns immediately. As the port matures,
//! these stubs will be replaced with real implementations.

use tracing::warn;

/// Placeholder for `overmap::place_nemesis()`.
pub fn place_nemesis() {
    warn!("Nemesis placement not yet ported from CDDA");
}
