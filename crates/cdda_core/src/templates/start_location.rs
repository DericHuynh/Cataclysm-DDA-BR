use crate::id::*;

/// A starting location for character creation or scenario spawning.
#[derive(Debug, Clone, PartialEq)]
pub struct StartLocationTemplate {
    pub name: String,
    pub terrain: Vec<OvermapTerrainId>,
}
