use crate::id::*;

#[derive(Debug, Clone, PartialEq)]
pub struct OvermapConnectionTemplate {
    pub name: String,
    pub from: OvermapTerrainId,
    pub to: OvermapTerrainId,
}
