use std::collections::HashMap;

/// A bidirectional string ↔ u32 mapping.
///
/// Strings are interned on first insertion and assigned consecutive integers
/// starting from 0.  The mapping is append-only: indices are never recycled.
#[derive(Debug, Default, Clone)]
pub struct StringInterner {
    strings: Vec<String>,
    lookup: HashMap<String, u32>,
}

impl StringInterner {
    /// Intern a string, returning its stable index.  Idempotent.
    pub fn intern(&mut self, s: impl Into<String>) -> u32 {
        let s = s.into();
        if let Some(&idx) = self.lookup.get(&s) {
            return idx;
        }
        let idx = self.strings.len() as u32;
        self.lookup.insert(s.clone(), idx);
        self.strings.push(s);
        idx
    }

    /// Look up the string for an index.  Returns `None` for out-of-range indices.
    pub fn resolve(&self, idx: u32) -> Option<&str> {
        self.strings.get(idx as usize).map(String::as_str)
    }

    /// Look up the index for a string without interning it.
    pub fn get(&self, s: &str) -> Option<u32> {
        self.lookup.get(s).copied()
    }

    /// Number of interned strings.
    pub fn len(&self) -> usize {
        self.strings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intern_is_stable() {
        let mut i = StringInterner::default();
        let a = i.intern("hello");
        let b = i.intern("world");
        let c = i.intern("hello");
        assert_eq!(a, c);
        assert_ne!(a, b);
    }

    #[test]
    fn resolve_roundtrip() {
        let mut i = StringInterner::default();
        let idx = i.intern("rock");
        assert_eq!(i.resolve(idx), Some("rock"));
    }

    #[test]
    fn get_without_intern() {
        let mut i = StringInterner::default();
        assert_eq!(i.get("missing"), None);
        i.intern("present");
        assert!(i.get("present").is_some());
    }

    #[test]
    fn consecutive_indices() {
        let mut i = StringInterner::default();
        let a = i.intern("a");
        let b = i.intern("b");
        let c = i.intern("c");
        assert_eq!(a, 0);
        assert_eq!(b, 1);
        assert_eq!(c, 2);
    }
}
