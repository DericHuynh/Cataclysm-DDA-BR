use bevy_ecs::prelude::*;
use std::collections::HashMap;
/// Definition category — the namespace of a definition key. Same-text IDs in
/// different categories are distinct definitions (an item "zombie" and a
/// monster "zombie" must not overwrite each other).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DefCategory {
    Item,
    Monster,
    Terrain,
    Furniture,
    Recipe,
    BodyPart,
}

/// Maps typed definition keys (category + string ID) to the definition Entity
/// in the main game World. Also keeps a flat item index: every current string
/// lookup is an item lookup, and items were the only unambiguous users.
///
/// A `generation` counter increments on every rebuild so runtime caches can
/// detect staleness. Definition entities are marked with `IsDef`.
#[derive(Resource, Debug, Default, Clone)]
pub struct DefinitionWorld {
    by_key: HashMap<(DefCategory, String), Entity>,
    items: HashMap<String, Entity>,
    generation: u64,
}

impl DefinitionWorld {
    pub fn at_generation(generation: u64) -> Self {
        Self {
            generation,
            ..Self::default()
        }
    }

    pub fn empty() -> Self {
        Self {
            by_key: HashMap::new(),
            items: HashMap::new(),
            generation: 0,
        }
    }

    /// Look up an ITEM definition entity by its string ID. (Legacy flat
    /// lookup; unambiguous because only item builders feed the flat index.)
    pub fn entity_by_str(&self, id: &str) -> Option<Entity> {
        self.items.get(id).copied()
    }

    /// Look up a definition entity by its typed key.
    pub fn entity_in(&self, category: DefCategory, id: &str) -> Option<Entity> {
        self.by_key.get(&(category, id.to_string())).copied()
    }

    /// Number of times the definition set was rebuilt (0 = never built).
    /// Runtime components caching definition entities should re-resolve when
    /// this changes.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Register a definition entity under its typed key; items also enter the
    /// flat legacy index. Same-key replacement is the normal reload path;
    /// cross-category same-text IDs no longer collide.
    pub fn register(&mut self, category: DefCategory, id: String, entity: Entity) {
        self.by_key.insert((category, id.clone()), entity);
        if category == DefCategory::Item {
            self.items.insert(id, entity);
        }
    }

    /// Number of registered definitions across all categories.
    pub fn len(&self) -> usize {
        self.by_key.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_key.is_empty()
    }

    /// Iterate over all (category, id, entity) triples.
    pub fn iter(&self) -> impl Iterator<Item = (DefCategory, &str, Entity)> + '_ {
        self.by_key.iter().map(|((c, id), &e)| (*c, id.as_str(), e))
    }
}
