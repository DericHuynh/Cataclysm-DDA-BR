use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A rotatable symbol definition from JSON type `"rotatable_symbol"`.
///
/// Defines a set of symbols that represent rotations of the same glyph,
/// used for terrain/furniture rotation.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RotatableSymbolDef {
    /// The tuple of rotated symbols. Length 2 or 4 depending on symmetry.
    pub tuple: Vec<String>,
}
