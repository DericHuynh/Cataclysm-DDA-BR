use serde::{Deserialize, Serialize};

/// Core creature statistics.
///
/// These are the base attributes that every creature has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stats {
    pub strength: u32,
    pub dexterity: u32,
    pub intelligence: u32,
    pub perception: u32,
}

impl Stats {
    pub const fn new(strength: u32, dexterity: u32, intelligence: u32, perception: u32) -> Self {
        Stats {
            strength,
            dexterity,
            intelligence,
            perception,
        }
    }
}

impl Default for Stats {
    fn default() -> Self {
        Stats {
            strength: 8,
            dexterity: 8,
            intelligence: 8,
            perception: 8,
        }
    }
}
