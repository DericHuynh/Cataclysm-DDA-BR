use std::collections::BTreeSet;

/// Behavioural tags for overmap land-use codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum OvermapLandUseCodeTag {
    /// The land-use code has detailed subregions.
    Detailed,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OvermapLandUseCodeTemplate {
    pub name: String,
    /// Behavioural tags.
    pub tags: BTreeSet<OvermapLandUseCodeTag>,
}
