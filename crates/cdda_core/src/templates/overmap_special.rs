use crate::flags::FlagSet;
use crate::id::*;

#[derive(Debug, Clone, PartialEq)]
pub struct OvermapSpecialTemplate {
    pub name: String,
    pub description: String,
    pub occurrences: Vec<OvermapOccurrence>,
    pub flags: FlagSet,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OvermapOccurrence {
    pub location: OvermapTerrainId,
    pub x: i32,
    pub y: i32,
    pub z: i32,
}
