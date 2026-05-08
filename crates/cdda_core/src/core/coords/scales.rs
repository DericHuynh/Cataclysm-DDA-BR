//! Scale marker types — what one unit of a coordinate represents.
//!
//! These are zero-sized marker types used as type parameters on `Pos<Scale, Origin>`.
//! They exist only at compile time and have no runtime representation.

/// Map square (the smallest unit — 1 tile).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ms {}

/// Submap — 12×12 tiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Sm {}

/// Overmap terrain — 24×24 tiles (2×2 submaps).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Omt {}

/// Overmap — 180×180 omts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Om {}
