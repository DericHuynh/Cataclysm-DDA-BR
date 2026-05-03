//! # Mutation templates
//!
//! Blueprint types for mutation definitions — permanent or temporary
//! alterations to a character's body, stats, and abilities.

use crate::flags::FlagSet;

/// The blueprint for a mutation definition.
///
/// Mutations are the core of CDDA's character transformation system.  They
/// can be gained through radiation exposure, mutagen serums, or threshold
/// breakthroughs.  Each mutation has a point cost (positive = beneficial,
/// negative = detrimental) and a set of flags that drive game-mechanic
/// effects.
#[derive(Debug, Clone, PartialEq)]
pub struct MutationTemplate {
    /// Display name.
    pub name: String,
    /// Flavour / examine description.
    pub description: String,
    /// Point cost (positive = advantage, negative = disadvantage).
    pub points: i32,
    /// Boolean tags driving behaviour (e.g. TAIL, CLAWS, NIGHT_VISION).
    pub flags: FlagSet,
}
