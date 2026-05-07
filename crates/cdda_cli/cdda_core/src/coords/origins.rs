//! Origin marker types — what position (0, 0) means.
//!
//! Zero-sized marker types used as type parameters on `Pos<Scale, Origin>`.
//! They exist only at compile time.

/// Absolute (global) origin — never shifts as the player moves.
/// Use `Abs` for all stored coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Abs {}

/// Relative to the reality-bubble top-left corner.
/// Used inside FOV and rendering code. Never stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Bubble {}

/// Generic relative origin — used for vehicle-relative coordinates,
/// submap-local offsets, and delta calculations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Rel {}
