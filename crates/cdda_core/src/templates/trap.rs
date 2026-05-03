use crate::flags::FlagSet;

/// A trap placed on a tile that triggers when stepped on or examined.
#[derive(Debug, Clone, PartialEq)]
pub struct TrapTemplate {
    pub name: String,
    pub symbol: char,
    pub color: String,
    pub flags: FlagSet,
    pub avoid: bool,
    pub difficulty: u32,
}
