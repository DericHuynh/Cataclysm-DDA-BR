//! # Effect templates
//!
//! Blueprint types for status-effect definitions — temporary conditions
//! applied to characters or monsters (bleeding, stunned, on-fire, etc.).

use crate::flags::FlagSet;
use crate::units::Time;

/// The blueprint for a status-effect definition.
///
/// Effects are temporary conditions that modify a character's stats,
/// behaviour, or appearance for a duration.  Examples include bleeding,
/// stunned, poisoned, on-fire, and drug-induced states.
#[derive(Debug, Clone, PartialEq)]
pub struct EffectTemplate {
    /// Display name.
    pub name: String,
    /// Default duration.
    pub duration: Time,
    /// Boolean tags (e.g. HARMFUL, BENEFICIAL, NO_RECOVER).
    pub flags: FlagSet,
}
