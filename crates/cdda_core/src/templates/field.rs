//! # Field templates
//!
//! Blueprint types for field / smoke / gas / effect definitions.  Fields are
//! tile-wide effects that exist for a duration — smoke, fire, electric
//! fields, gas clouds, etc.

use crate::units::*;

/// The blueprint for a field type at a given intensity level.
///
/// Fields have multiple intensity levels (e.g. light smoke → thick smoke),
/// each with their own decay rate and behaviour flags.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldTemplate {
    /// Display name.
    pub name: String,
    /// Map-display character.
    pub symbol: char,
    /// Intensity level (1 = weakest, higher = stronger).
    pub intensity: u32,
    /// How long this field intensity lasts before decaying / disappearing.
    pub decay: Time,
    /// Whether line-of-sight passes through this field.
    pub transparent: bool,
    /// Whether entering / being in this field harms the player.
    pub dangerous: bool,
}
