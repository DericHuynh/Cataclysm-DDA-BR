use crate::core::DefId;

/// Phantom type for damage type definitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DamageTypeDef;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DamageEntry {
    pub damage_type: DefId<DamageTypeDef>,
    pub amount: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Damage {
    entries: Vec<DamageEntry>,
}

impl Damage {
    pub const ZERO: Damage = Damage {
        entries: Vec::new(),
    };
    pub fn new() -> Self {
        Self::ZERO
    }

    pub fn add(&mut self, damage_type: DefId<DamageTypeDef>, amount: u32) {
        if amount == 0 {
            return;
        }
        for entry in &mut self.entries {
            if entry.damage_type == damage_type {
                entry.amount = entry.amount.saturating_add(amount);
                return;
            }
        }
        self.entries.push(DamageEntry {
            damage_type,
            amount,
        });
    }

    pub fn merge(&mut self, other: &Damage) {
        for entry in &other.entries {
            self.add(entry.damage_type.clone(), entry.amount);
        }
    }

    pub fn total(&self) -> u32 {
        self.entries.iter().map(|e| e.amount).sum()
    }
    pub fn by_type(&self, damage_type: DefId<DamageTypeDef>) -> u32 {
        self.entries
            .iter()
            .find(|e| e.damage_type == damage_type)
            .map_or(0, |e| e.amount)
    }
    pub fn iter(&self) -> impl Iterator<Item = &DamageEntry> + '_ {
        self.entries.iter()
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

impl Default for Damage {
    fn default() -> Self {
        Self::ZERO
    }
}

impl IntoIterator for Damage {
    type Item = DamageEntry;
    type IntoIter = std::vec::IntoIter<DamageEntry>;
    fn into_iter(self) -> Self::IntoIter {
        self.entries.into_iter()
    }
}

impl std::ops::Add for Damage {
    type Output = Self;
    fn add(mut self, rhs: Self) -> Self {
        self.merge(&rhs);
        self
    }
}
impl std::ops::AddAssign for Damage {
    fn add_assign(&mut self, rhs: Self) {
        self.merge(&rhs);
    }
}

impl Damage {
    pub fn of(damage_type: DefId<DamageTypeDef>, amount: u32) -> Self {
        let mut d = Self::ZERO;
        d.add(damage_type, amount);
        d
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn bash() -> DefId<DamageTypeDef> {
        DefId::new("bash")
    }
    fn cut() -> DefId<DamageTypeDef> {
        DefId::new("cut")
    }

    #[test]
    fn zero_is_empty() {
        let d = Damage::ZERO;
        assert!(d.is_empty());
        assert_eq!(d.total(), 0);
    }

    #[test]
    fn add_and_total() {
        let mut d = Damage::ZERO;
        d.add(bash(), 5);
        d.add(cut(), 3);
        assert_eq!(d.total(), 8);
        assert_eq!(d.len(), 2);
    }

    #[test]
    fn same_type_merges() {
        let mut d = Damage::ZERO;
        d.add(bash(), 5);
        d.add(bash(), 3);
        assert_eq!(d.len(), 1);
        assert_eq!(d.by_type(bash()), 8);
    }

    #[test]
    fn zero_not_stored() {
        let mut d = Damage::ZERO;
        d.add(bash(), 0);
        assert!(d.is_empty());
    }

    #[test]
    fn by_type_missing() {
        assert_eq!(Damage::ZERO.by_type(bash()), 0);
    }

    #[test]
    fn merge_combines() {
        let mut a = Damage::new();
        a.add(bash(), 5);
        let mut b = Damage::new();
        b.add(cut(), 3);
        b.add(bash(), 2);
        a.merge(&b);
        assert_eq!(a.total(), 10);
        assert_eq!(a.by_type(bash()), 7);
        assert_eq!(a.by_type(cut()), 3);
    }

    #[test]
    fn add_op() {
        let c = Damage::of(bash(), 5) + Damage::of(cut(), 3);
        assert_eq!(c.total(), 8);
    }

    #[test]
    fn clear_empties() {
        let mut d = Damage::of(bash(), 5);
        d.clear();
        assert!(d.is_empty());
    }
}
