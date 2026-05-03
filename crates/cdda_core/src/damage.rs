use serde::{Deserialize, Serialize};

/// Damage types used in combat calculations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Damage {
    pub bash: u32,
    pub cut: u32,
    pub stab: u32,
    pub bullet: u32,
    pub heat: u32,
    pub cold: u32,
    pub electric: u32,
    pub acid: u32,
    pub biological: u32,
    pub pure: u32,
}

impl Damage {
    pub const ZERO: Damage = Damage {
        bash: 0,
        cut: 0,
        stab: 0,
        bullet: 0,
        heat: 0,
        cold: 0,
        electric: 0,
        acid: 0,
        biological: 0,
        pure: 0,
    };

    pub fn total(&self) -> u32 {
        self.bash
            + self.cut
            + self.stab
            + self.bullet
            + self.heat
            + self.cold
            + self.electric
            + self.acid
            + self.biological
            + self.pure
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_damage_total() {
        let d = Damage {
            bash: 5,
            cut: 3,
            ..Damage::ZERO
        };
        assert_eq!(d.total(), 8);
    }
}
