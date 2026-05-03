use crate::id::*;

/// A group of traits (mutations) selectable together during character creation.
#[derive(Debug, Clone, PartialEq)]
pub struct TraitGroupTemplate {
    pub name: String,
    pub traits: Vec<MutationId>,
}
