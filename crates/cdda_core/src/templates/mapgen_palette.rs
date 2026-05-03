//! # Mapgen-palette templates
//!
//! Blueprint types for mapgen-palette definitions — character-to-definition
//! mappings used by procedural map generation.

use crate::id::*;

/// The blueprint for a mapgen-palette definition.
///
/// Palettes map the glyph characters used in mapgen blueprints to concrete
/// terrain and furniture definitions.  This allows mapgen JSON to be succinct
/// while still having precise control over what each tile represents.
#[derive(Debug, Clone, PartialEq)]
pub struct MapgenPaletteTemplate {
    /// Glyph → terrain mappings.
    pub terrain_mappings: Vec<(char, TerrainId)>,
    /// Glyph → furniture mappings.
    pub furniture_mappings: Vec<(char, FurnitureId)>,
}
