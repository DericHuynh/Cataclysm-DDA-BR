use crate::flags::FlagSet;
use std::collections::BTreeSet;

/// Behavioural tags for traps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TrapTag {
    /// Creatures will try to avoid stepping on this trap.
    Avoid,
}

/// A trap placed on a tile that triggers when stepped on or examined.
#[derive(Debug, Clone, PartialEq)]
pub struct TrapTemplate {
    pub name: String,
    pub symbol: char,
    pub color: String,
    pub flags: FlagSet,
    /// Behavioural tags.
    pub tags: BTreeSet<TrapTag>,
    pub difficulty: u32,
}
