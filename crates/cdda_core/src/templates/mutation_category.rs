use crate::id::*;

/// A category grouping related mutations (e.g., "LIZARD", "SPIDER").
#[derive(Debug, Clone, PartialEq)]
pub struct MutationCategoryTemplate {
    pub name: String,
    pub description: String,
    pub threshold_mutation: Option<MutationId>,
}
