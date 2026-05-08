use crate::id::GenId;

/// Dense generational storage with ABA protection.
///
/// O(1) lookup by [`GenId`], O(1) iteration. Array-backed with free-list
/// for efficient reuse of freed indices.
pub struct IdSlab<T> {
    entries: Vec<Option<(u32, T)>>,
    free: Vec<u32>,
    next_generation: u32,
}

impl<T> IdSlab<T> {
    pub fn new() -> Self {
        IdSlab {
            entries: Vec::new(),
            free: Vec::new(),
            next_generation: 1,
        }
    }

    pub fn insert(&mut self, value: T) -> GenId {
        if let Some(idx) = self.free.pop() {
            let gen = self.next_generation;
            self.next_generation += 1;
            self.entries[idx as usize] = Some((gen, value));
            GenId {
                index: idx,
                generation: gen,
            }
        } else {
            let idx = u32::try_from(self.entries.len()).expect("slab overflow");
            let gen = 0;
            self.entries.push(Some((gen, value)));
            GenId {
                index: idx,
                generation: gen,
            }
        }
    }

    pub fn get(&self, id: GenId) -> Option<&T> {
        self.entries
            .get(id.index as usize)?
            .as_ref()
            .filter(|(gen, _)| *gen == id.generation)
            .map(|(_, v)| v)
    }

    pub fn get_mut(&mut self, id: GenId) -> Option<&mut T> {
        self.entries
            .get_mut(id.index as usize)?
            .as_mut()
            .filter(|(gen, _)| *gen == id.generation)
            .map(|(_, v)| v)
    }

    pub fn remove(&mut self, id: GenId) -> Option<T> {
        let entry = self.entries.get_mut(id.index as usize)?;
        if let Some((gen, _)) = entry {
            if *gen == id.generation {
                let (_, value) = entry.take().unwrap();
                self.free.push(id.index);
                return Some(value);
            }
        }
        None
    }

    pub fn iter(&self) -> impl Iterator<Item = (GenId, &T)> + '_ {
        self.entries.iter().enumerate().filter_map(|(idx, opt)| {
            opt.as_ref().map(|(gen, v)| {
                (
                    GenId {
                        index: idx as u32,
                        generation: *gen,
                    },
                    v,
                )
            })
        })
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (GenId, &mut T)> + '_ {
        self.entries
            .iter_mut()
            .enumerate()
            .filter_map(|(idx, opt)| {
                opt.as_mut().map(|(gen, v)| {
                    (
                        GenId {
                            index: idx as u32,
                            generation: *gen,
                        },
                        v,
                    )
                })
            })
    }

    pub fn len(&self) -> usize {
        self.entries.iter().filter(|e| e.is_some()).count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<T> Default for IdSlab<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_get() {
        let mut slab = IdSlab::new();
        let id = slab.insert("hello");
        assert_eq!(slab.get(id), Some(&"hello"));
    }

    #[test]
    fn remove_and_reuse_frees_list() {
        let mut slab = IdSlab::new();
        let id_a = slab.insert("a");
        let id_b = slab.insert("b");
        slab.remove(id_a);
        let id_c = slab.insert("c");
        assert_eq!(id_c.index, id_a.index);
        assert_ne!(id_c.generation, id_a.generation);
    }

    #[test]
    fn older_generation_fails_after_reuse() {
        let mut slab = IdSlab::new();
        let id_a = slab.insert("a");
        slab.remove(id_a);
        slab.insert("b");
        assert_eq!(slab.get(id_a), None);
    }

    #[test]
    fn iter_yields_occupied() {
        let mut slab = IdSlab::new();
        let a = slab.insert("a");
        let b = slab.insert("b");
        slab.remove(a);
        let vals: Vec<_> = slab.iter().map(|(_, v)| *v).collect();
        assert_eq!(vals, vec!["b"]);
    }

    #[test]
    fn len_and_is_empty() {
        let mut slab = IdSlab::new();
        assert!(slab.is_empty());
        let id = slab.insert(1);
        assert_eq!(slab.len(), 1);
        slab.remove(id);
        assert!(slab.is_empty());
    }

    #[test]
    fn remove_nonexistent_returns_none() {
        let mut slab: IdSlab<i32> = IdSlab::new();
        let bad = GenId {
            index: 999,
            generation: 0,
        };
        assert_eq!(slab.remove(bad), None);
    }
}
