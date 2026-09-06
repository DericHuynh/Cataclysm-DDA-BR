//! Planning models — the component-typed state slots the HTN kernels read
//! and (predictively) write.
//!
//! These are **simulated** planning values, not live world components (they
//! materialize as `PlanState` slots via `cdda_htn::PlanComponent`).
//! Overlapping selectors must answer from the SAME model: e.g. the
//! `InventoryModel` is one shared simulated inventory, so consuming water
//! also changes the answer to "how many food-category items do I carry?" —
//! independently cached counts would drift apart. Models are bounded and
//! divided by concern (needs / inventory / nearby / navigation), never one
//! world snapshot.

use bevy_ecs::prelude::*;
use cdda_core_types::core::coords::WorldPos;

/// Live actor needs, when the simulation tracks them. Optional: the planner
/// reads defaults (0) when an actor carries none. Owned by `cdda_sim::ai::htn`
/// until a second consumer justifies moving it to `cdda_components`.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct Needs {
    pub hunger: i32,
    pub thirst: i32,
    pub fatigue: i32,
}

/// The simulated needs model (planning slot).
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct NeedsModel {
    pub hunger: i32,
    pub thirst: i32,
    pub fatigue: i32,
}

/// One observed item instance: entity identity (for binding a concrete
/// request), definition facts, and count.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct ObservedItem {
    /// The live item entity, when this observation came from real entities.
    pub entity: Option<Entity>,
    /// The item definition id (e.g. `"water_clean"`).
    pub def: String,
    /// The item's category, resolved through the definition catalog.
    pub category: Option<String>,
    /// Stack count (1 for unstacked instances).
    pub count: i32,
    /// Where the item is (ground items only; carried items have none).
    pub pos: Option<WorldPos>,
}

/// The simulated carried inventory. Overlapping selectors (by id, by
/// category) all answer from this one model.
#[derive(Component, Clone, Debug, Default)]
pub struct InventoryModel {
    pub items: Vec<ObservedItem>,
}

impl InventoryModel {
    /// Count carried items matching a definition id exactly.
    pub fn count_of_def(&self, def: &str) -> i32 {
        self.items
            .iter()
            .filter(|i| i.def == def)
            .map(|i| i.count)
            .sum()
    }

    /// Count carried items in a category.
    pub fn count_of_category(&self, category: &str) -> i32 {
        self.items
            .iter()
            .filter(|i| i.category.as_deref() == Some(category))
            .map(|i| i.count)
            .sum()
    }

    /// The first carried item matching id or category, nearest-first is the
    /// observation's ordering responsibility; here it is inventory order.
    pub fn find(&self, def: Option<&str>, category: Option<&str>) -> Option<&ObservedItem> {
        self.items.iter().find(|i| match (def, category) {
            (Some(d), _) => i.def == d,
            (None, Some(c)) => i.category.as_deref() == Some(c),
            (None, None) => false,
        })
    }

    /// Predicted consumption: remove `count` matching items (stack-aware).
    pub fn remove(&mut self, def: Option<&str>, category: Option<&str>, count: i32) {
        let mut left = count;
        for item in self.items.iter_mut().rev() {
            if left == 0 {
                break;
            }
            let matches = match (def, category) {
                (Some(d), _) => item.def == d,
                (None, Some(c)) => item.category.as_deref() == Some(c),
                (None, None) => false,
            };
            if matches {
                let taken = item.count.min(left);
                item.count -= taken;
                left -= taken;
            }
        }
        self.items.retain(|i| i.count > 0);
    }
}

/// Simulated nearby ground items within the observation radius.
#[derive(Component, Clone, Debug, Default)]
pub struct NearbyModel {
    pub items: Vec<ObservedItem>,
}

impl NearbyModel {
    /// Count nearby items matching id or category.
    pub fn count(&self, def: Option<&str>, category: Option<&str>) -> i32 {
        self.items
            .iter()
            .filter(|i| match (def, category) {
                (Some(d), _) => i.def == d,
                (None, Some(c)) => i.category.as_deref() == Some(c),
                (None, None) => false,
            })
            .map(|i| i.count)
            .sum()
    }

    /// The nearest matching item (observation order is nearest-first).
    pub fn nearest(&self, def: Option<&str>, category: Option<&str>) -> Option<&ObservedItem> {
        self.items.iter().find(|i| match (def, category) {
            (Some(d), _) => i.def == d,
            (None, Some(c)) => i.category.as_deref() == Some(c),
            (None, None) => false,
        })
    }

    /// Predicted pickup: move one matching item from nearby to inventory.
    pub fn take_into(&mut self, inventory: &mut InventoryModel, def: Option<&str>, category: Option<&str>) {
        if let Some(pos) = self
            .items
            .iter()
            .position(|i| match (def, category) {
                (Some(d), _) => i.def == d,
                (None, Some(c)) => i.category.as_deref() == Some(c),
                (None, None) => false,
            })
        {
            let item = self.items.remove(pos);
            inventory.items.push(item);
        }
    }
}

/// Simulated navigation: where the actor stands right now (per the
/// observation). Approach-style kernels predict one-tile steps against it.
#[derive(Component, Clone, Debug, Default)]
pub struct NavigationModel {
    pub pos: Option<WorldPos>,
}
