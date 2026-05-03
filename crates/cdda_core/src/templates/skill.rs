//! # Skill templates
//!
//! Blueprint types for skill definitions — player-advanceable proficiencies
//! that govern crafting, combat, and interaction.

/// The blueprint for a skill definition.
///
/// Skills are the core progression mechanic — characters gain experience
/// through use, and higher levels unlock recipes, improve combat, and reduce
/// failure rates.
#[derive(Debug, Clone, PartialEq)]
pub struct SkillTemplate {
    /// Display name.
    pub name: String,
    /// Flavour / examine description.
    pub description: String,
    /// Maximum level this skill can reach (typically 10).
    pub max_level: u32,
}
