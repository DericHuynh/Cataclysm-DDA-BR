//! # Bionic templates
//!
//! Blueprint types for bionic / CBM definitions — surgically-installed
//! cybernetic augmentations that grant special abilities.

use crate::flags::FlagSet;

/// The blueprint for a bionic (CBM) definition.
///
/// Bionics are permanent cybernetic implants that consume power and provide
/// active or passive abilities — night vision, integrated toolkits, stat
/// boosts, etc.
#[derive(Debug, Clone, PartialEq)]
pub struct BionicTemplate {
    /// Display name.
    pub name: String,
    /// Flavour / examine description.
    pub description: String,
    /// Boolean tags driving behaviour (e.g. ACTIVE, PASSIVE, POWER_SOURCE).
    pub flags: FlagSet,
}
