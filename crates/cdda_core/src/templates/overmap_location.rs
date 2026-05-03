use crate::id::*;

#[derive(Debug, Clone, PartialEq)]
pub struct OvermapLocationTemplate {
    pub name: String,
    pub terrain: Vec<(OvermapTerrainId, u32)>,
}
