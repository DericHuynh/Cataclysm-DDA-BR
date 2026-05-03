//! # Recipe templates
//!
//! Blueprint types for crafting recipes — what items, skills, and time are
//! required to produce a result.

use crate::flags::FlagSet;
use crate::id::*;
use crate::units::*;

/// The blueprint for a crafting recipe.
///
/// A recipe describes how to turn one set of items into another item, given
/// sufficient skill, tools, and time.
#[derive(Debug, Clone, PartialEq)]
pub struct RecipeTemplate {
    /// The item produced by this recipe.
    pub result: ItemId,
    /// Difficulty rating (affects success chance / quality).
    pub difficulty: u32,
    /// Skills required and their minimum levels.
    pub skills_required: Vec<(SkillId, u32)>,
    /// Base time to craft (before tool / speed modifiers).
    pub time: Time,
    /// Number of charges produced (None = single item).
    pub charges: Option<u32>,
    /// Boolean tags (e.g. BLIND_EASY, SECRET, REVERSIBLE).
    pub flags: FlagSet,
}
