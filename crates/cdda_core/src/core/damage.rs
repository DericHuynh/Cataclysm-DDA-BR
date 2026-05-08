use crate::core::id::DamageTypeId;

/// A single damage type + amount pair.
///
/// Used within [`Damage`] to represent one contribution to a damage profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DamageEntry {
    /// Which damage type this entry applies to (bash, cut, fire, arcane, …).
    pub damage_type: DamageTypeId,
    /// Raw amount before armor mitigation.
    pub amount: u32,
}

/// A sparse, dynamic damage profile.
///
/// Stores only non-zero entries. Unlike the original CDDA which hardcodes
/// 10 damage-type fields (bash, cut, stab, bullet, …), this representation
/// supports **any** damage type defined in JSON data — including types added
/// by mods like Magiclysm (arcane, spirit, necrotic).
///
/// Most attacks deal 1–3 damage types, so iteration is always cheap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Damage {
    entries: Vec<DamageEntry>,
}

impl Damage {
    /// Empty damage profile (no entries, zero total).
    pub const ZERO: Damage = Damage {
        entries: Vec::new(),
    };

    /// Create an empty damage profile.
    #[inline]
    pub fn new() -> Self {
        Self::ZERO
    }

    /// Add (or merge into) a damage entry.
    ///
    /// If an entry for the same damage type already exists, the amounts are
    /// summed.  Zero-amount entries are silently ignored.
    pub fn add(&mut self, damage_type: DamageTypeId, amount: u32) {
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

    /// Union of two damage profiles (type-merged).
    ///
    /// Entries with matching damage types are merged using the same rules
    /// as [`add`](Damage::add).
    pub fn merge(&mut self, other: &Damage) {
        for entry in &other.entries {
            self.add(entry.damage_type, entry.amount);
        }
    }

    /// Sum of all damage amounts (raw total, ignoring armour).
    pub fn total(&self) -> u32 {
        self.entries.iter().map(|e| e.amount).sum()
    }

    /// Amount for a specific damage type, or 0 if absent.
    pub fn by_type(&self, damage_type: DamageTypeId) -> u32 {
        self.entries
            .iter()
            .find(|e| e.damage_type == damage_type)
            .map_or(0, |e| e.amount)
    }

    /// Iterate over all damage entries.
    pub fn iter(&self) -> impl Iterator<Item = &DamageEntry> + '_ {
        self.entries.iter()
    }

    /// Number of distinct damage types stored.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether this profile is empty (no damage types).
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Remove all entries.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

impl Default for Damage {
    #[inline]
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

// ---- Arithmetic -----------------------------------------------------------

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

// ---- Helper constructors for ergonomic testing ----------------------------

impl Damage {
    /// Build a simple single-type damage profile.
    pub fn of(damage_type: DamageTypeId, amount: u32) -> Self {
        let mut d = Self::ZERO;
        d.add(damage_type, amount);
        d
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::id::{DamageTypeId, DefIdx};
    fn some_type() -> DamageTypeId {
        DamageTypeId(DefIdx(0))
    }

    fn other_type() -> DamageTypeId {
        DamageTypeId(DefIdx(1))
    }

    #[test]
    fn zero_is_empty() {
        let d = Damage::ZERO;
        assert!(d.is_empty());
        assert_eq!(d.total(), 0);
        assert_eq!(d.len(), 0);
    }

    #[test]
    fn add_and_total() {
        let mut d = Damage::ZERO;
        d.add(some_type(), 5);
        d.add(other_type(), 3);
        assert_eq!(d.total(), 8);
        assert_eq!(d.len(), 2);
    }

    #[test]
    fn same_type_merges() {
        let mut d = Damage::ZERO;
        d.add(some_type(), 5);
        d.add(some_type(), 3);
        assert_eq!(d.len(), 1);
        assert_eq!(d.by_type(some_type()), 8);
    }

    #[test]
    fn zero_amount_not_stored() {
        let mut d = Damage::ZERO;
        d.add(some_type(), 0);
        assert!(d.is_empty());
    }

    #[test]
    fn by_type_returns_zero_for_missing() {
        let d = Damage::ZERO;
        assert_eq!(d.by_type(some_type()), 0);
    }

    #[test]
    fn merge_combines() {
        let mut a = Damage::new();
        a.add(some_type(), 5);

        let mut b = Damage::new();
        b.add(other_type(), 3);
        b.add(some_type(), 2);

        a.merge(&b);
        assert_eq!(a.total(), 10);
        assert_eq!(a.by_type(some_type()), 7);
        assert_eq!(a.by_type(other_type()), 3);
    }

    #[test]
    fn add_operator() {
        let a = Damage::of(some_type(), 5);
        let b = Damage::of(other_type(), 3);
        let c = a + b;
        assert_eq!(c.total(), 8);
    }

    #[test]
    fn iter_yields_entries() {
        let d = Damage::of(some_type(), 5) + Damage::of(other_type(), 3);
        let types: Vec<_> = d.iter().map(|e| (e.damage_type, e.amount)).collect();
        assert_eq!(types.len(), 2);
    }

    #[test]
    fn clear_empties() {
        let mut d = Damage::of(some_type(), 5);
        assert!(!d.is_empty());
        d.clear();
        assert!(d.is_empty());
    }
}
