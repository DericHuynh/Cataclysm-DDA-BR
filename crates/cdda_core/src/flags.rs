use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// A set of string flags.
///
/// Flags are used throughout CDDA definitions to tag items, terrain, monsters, etc.
/// with boolean properties (e.g. "FIRE", "CONTAINER", "NO_FLOOR").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlagSet {
    flags: BTreeSet<String>,
}

impl FlagSet {
    pub fn new() -> Self {
        FlagSet {
            flags: BTreeSet::new(),
        }
    }

    /// Create a `FlagSet` from a list of flag strings.
    pub fn from_strings<I, S>(strings: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        FlagSet {
            flags: strings.into_iter().map(|s| s.into()).collect(),
        }
    }

    /// Check if a flag is present.
    pub fn has(&self, flag: &str) -> bool {
        self.flags.contains(flag)
    }

    /// Insert a flag.
    pub fn insert(&mut self, flag: impl Into<String>) {
        self.flags.insert(flag.into());
    }

    /// Remove a flag.
    pub fn remove(&mut self, flag: &str) {
        self.flags.remove(flag);
    }

    /// Return an iterator over all flags.
    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.flags.iter().map(|s| s.as_str())
    }

    /// Returns true if the set is empty.
    pub fn is_empty(&self) -> bool {
        self.flags.is_empty()
    }

    /// Returns the number of flags.
    pub fn len(&self) -> usize {
        self.flags.len()
    }
}

impl Default for FlagSet {
    fn default() -> Self {
        FlagSet::new()
    }
}

impl IntoIterator for FlagSet {
    type Item = String;
    type IntoIter = std::collections::btree_set::IntoIter<String>;

    fn into_iter(self) -> Self::IntoIter {
        self.flags.into_iter()
    }
}
