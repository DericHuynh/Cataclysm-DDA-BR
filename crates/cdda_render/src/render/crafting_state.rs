//! Crafting presenter: selection, filtering, labels, and screen data extraction.
use bevy::prelude::*;
use cdda_components::def::{
    ItemName, RecipeCategory, RecipeComponents, RecipeQualities, RecipeResult, RecipeResultCount,
    RecipeSubcategory, RecipeTime,
};
use cdda_components::item::InProgressCraft;
use cdda_components::recipe::RecipeIndex;
use cdda_components::ItemTypeId;
use cdda_context::state::ContextActions;
use cdda_data::def_world::DefinitionWorld;
use cdda_input::BindableAction;
use cdda_sim::crafting::systems::{
    check_can_craft, collect_available_items, find_dev_player, CraftRevision,
};
use cdda_sim::inventory::examine_resource::ExaminedItem;
// ---------------------------------------------------------------------------
// CategoryIndex — tabbed category navigation for the crafting menu
// ---------------------------------------------------------------------------

/// Maps recipe category → subcategory → recipe entity list.
/// Built in `build_craft_state` by iterating recipe entities and reading
/// `RecipeCategory` / `RecipeSubcategory` / `IsRecipeDef` components.
#[derive(Resource, Default, Debug, Clone, PartialEq, Eq)]
pub struct CategoryIndex {
    /// Ordered list of top-level category display names (e.g. "FOOD", "WEAPON").
    pub top_categories: Vec<String>,
    /// (top_category_display_name, subcategory_display_name) → list of recipe entities.
    pub sub_recipes: std::collections::BTreeMap<(String, String), Vec<Entity>>,
    /// Which top-level category is currently selected.
    pub selected_top: usize,
    /// Which subcategory within the selected top category is selected.
    pub selected_sub: usize,
    /// Which zone has keyboard focus: 0=recipe list, 1=category tabs, 2=subcategory tabs.
    pub focus_zone: usize,
}

/// Strip the "CC_" prefix from a category string for display.
pub fn display_category(raw: &str) -> String {
    raw.strip_prefix("CC_")
        .map(|s| s.to_string())
        .unwrap_or_else(|| raw.to_string())
}

/// Given the raw category (e.g. "CC_FOOD") and raw subcategory (e.g. "CSC_FOOD_BREAD"),
/// return a display name for the subcategory ("BREAD").
pub fn display_subcategory(raw_category: &str, raw_subcategory: &str) -> String {
    // Strip "CSC_" prefix, then strip the category short name + "_".
    let cat_short = raw_category.strip_prefix("CC_").unwrap_or(raw_category);
    let without_csc = raw_subcategory
        .strip_prefix("CSC_")
        .unwrap_or(raw_subcategory);
    // Remove the category short name prefix if present ("FOOD_BREAD" → "BREAD")
    without_csc
        .strip_prefix(&format!("{}_", cat_short))
        .map(|s| s.to_string())
        .unwrap_or_else(|| without_csc.to_string())
}

// ---------------------------------------------------------------------------
// CraftEntry / CraftState — UI-facing crafting data
// ---------------------------------------------------------------------------

/// One row in the crafting menu recipe list.
#[derive(Clone, PartialEq, Eq)]
pub struct CraftEntry {
    pub recipe_entity: Entity,
    pub recipe_key: String,
    pub result_id: String,
    pub result_name: String,
    pub result_count: u32,
    pub craftable: bool,
    /// First blocking reason when not craftable.
    pub reason: String,
    pub time_turns: u32,
    pub components_text: Vec<String>,
    pub qualities_text: Vec<String>,
}

/// UI state for the crafting menu, rebuilt each time the menu is opened.
#[derive(Resource, PartialEq, Eq)]
pub struct CraftState {
    pub focus: usize,
    /// When `true`, shows all recipes; when `false`, shows only craftable ones.
    pub show_all: bool,
    /// Message shown after a craft attempt (success or failure).
    pub last_message: Option<String>,
    /// Current substring filter (case-insensitive match on result name/ID).
    pub filter: String,
    /// True while the TextInput context is active for filter editing.
    pub filtering: bool,
}

impl Default for CraftState {
    fn default() -> Self {
        Self {
            focus: 0,
            show_all: true,
            last_message: None,
            filter: String::new(),
            filtering: false,
        }
    }
}

/// Immutable screen read model; selection and filter edits never change this resource.
#[derive(Resource, Default, PartialEq, Eq)]
pub struct CraftModel {
    pub entries: Vec<CraftEntry>,
}

/// Filter membership is independent of selection. Both input and rendering use
/// this cache contract, including multiple actions delivered in the same frame.
#[derive(Default)]
pub struct RecipeFilter {
    key: Option<(u32, u32, usize, usize, String, bool)>,
    pub indices: Vec<usize>,
    pub rebuilds: u64,
}
impl RecipeFilter {
    pub fn update(
        &mut self,
        model: &CraftModel,
        state: &CraftState,
        categories: &CategoryIndex,
        versions: (u32, u32),
    ) -> bool {
        let key = (
            versions.0,
            versions.1,
            categories.selected_top,
            categories.selected_sub,
            state.filter.clone(),
            state.show_all,
        );
        if self.key.as_ref() == Some(&key) {
            return false;
        }
        self.indices = category_entry_indices(model, state, categories);
        self.key = Some(key);
        self.rebuilds += 1;
        true
    }
}

// ---------------------------------------------------------------------------
// build_craft_state — exclusive state builder
// ---------------------------------------------------------------------------

/// Rebuild `CraftState` from the current world state.
/// Runs on `OnEnter(Ctx::CraftingMenu)` and after each craft completes.
pub fn build_craft_state(world: &mut World) {
    let Some(player) = find_dev_player(world) else {
        return;
    };

    let previous_categories = world
        .get_resource::<CategoryIndex>()
        .cloned()
        .unwrap_or_default();
    let previous_top = previous_categories
        .top_categories
        .get(previous_categories.selected_top)
        .cloned();
    let previous_sub = previous_categories
        .sub_recipes
        .keys()
        .filter(|(top, _)| Some(top) == previous_top.as_ref())
        .nth(previous_categories.selected_sub)
        .map(|(_, sub)| sub.clone());
    let selected_key = world
        .get_resource::<CraftState>()
        .zip(world.get_resource::<CraftModel>())
        .and_then(|(state, model)| {
            category_entry_indices(model, state, &previous_categories)
                .get(state.focus)
                .map(|&i| model.entries[i].recipe_key.clone())
        });
    let available = collect_available_items(world, player);

    let recipe_entities: Vec<Entity> = world
        .get_resource::<RecipeIndex>()
        .map(|ri| ri.0.clone())
        .unwrap_or_default();

    // ── Build category index ──────────────────────────────────────────────
    let mut cat_index = CategoryIndex::default();
    let mut seen_top: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut seen_sub: std::collections::BTreeSet<(String, String)> =
        std::collections::BTreeSet::new();

    for &re in &recipe_entities {
        if world.get::<RecipeResult>(re).is_none() {
            continue;
        }
        let raw_cat = world
            .get::<RecipeCategory>(re)
            .map(|c| c.0.clone())
            .unwrap_or_else(|| "CC_MISC".to_string());
        let raw_sub = world
            .get::<RecipeSubcategory>(re)
            .map(|s| s.0.clone())
            .unwrap_or_else(|| "CSC_MISC_NONE".to_string());

        let cat_display = display_category(&raw_cat);
        let sub_display = display_subcategory(&raw_cat, &raw_sub);

        seen_top.insert(cat_display.clone());
        seen_sub.insert((cat_display.clone(), sub_display.clone()));

        cat_index
            .sub_recipes
            .entry((cat_display, sub_display))
            .or_default()
            .push(re);
    }

    cat_index.top_categories = seen_top.into_iter().collect();
    cat_index.selected_top = cat_index
        .top_categories
        .iter()
        .position(|top| Some(top) == previous_top.as_ref())
        .unwrap_or(0);
    let top = cat_index.top_categories.get(cat_index.selected_top);
    cat_index.selected_sub = cat_index
        .sub_recipes
        .keys()
        .filter(|(t, _)| Some(t) == top)
        .position(|(_, sub)| Some(sub) == previous_sub.as_ref())
        .unwrap_or(0);
    cat_index.focus_zone = previous_categories.focus_zone;

    if let Some(mut current) = world.get_resource_mut::<CategoryIndex>() {
        current.set_if_neq(cat_index.clone());
    } else {
        world.insert_resource(cat_index.clone());
    }

    // ── Build craft entries ───────────────────────────────────────────────
    let def_world: Option<&DefinitionWorld> = world.get_resource::<DefinitionWorld>();

    let mut entries: Vec<CraftEntry> = recipe_entities
        .iter()
        .filter(|&&re| world.get::<RecipeResult>(re).is_some())
        .filter_map(|&re| {
            let result_id = world
                .get::<RecipeResult>(re)
                .map(|r| r.0.clone())
                .unwrap_or_default();

            // Look up display name from the item def entity
            let result_name = def_world
                .and_then(|dw| dw.entity_by_str(&result_id))
                .and_then(|def_e| world.get::<ItemName>(def_e).map(|n| n.0.clone()))
                .unwrap_or_else(|| result_id.clone());

            let result_count = world.get::<RecipeResultCount>(re).map(|c| c.0).unwrap_or(1);
            let time_turns = world.get::<RecipeTime>(re).map(|t| t.0).unwrap_or(0);

            let (craftable, reason) = match check_can_craft(world, re, &available) {
                Ok(()) => (true, String::new()),
                Err(e) => (false, e),
            };

            let components_text = world
                .get::<RecipeComponents>(re)
                .map(|comps| {
                    comps
                        .0
                        .iter()
                        .filter_map(|slot| slot.first())
                        .map(|entry| {
                            if slot_has_alternatives(world, re, entry.item_id) {
                                format!("  {:?} x{}  (or alternatives)", entry.item_id, entry.count)
                            } else {
                                format!("  {:?} x{}", entry.item_id, entry.count)
                            }
                        })
                        .collect()
                })
                .unwrap_or_default();

            let qualities_text = world
                .get::<RecipeQualities>(re)
                .map(|quals| {
                    quals
                        .0
                        .iter()
                        .map(|(id, lvl)| format!("  {:?} (level {})", id, lvl))
                        .collect()
                })
                .unwrap_or_default();

            Some(CraftEntry {
                recipe_entity: re,
                recipe_key: world
                    .get::<cdda_components::def::DefStrId>(re)
                    .map(|id| id.0.clone())
                    .unwrap_or_else(|| format!("{re:?}")),
                result_id,
                result_name,
                result_count,
                craftable,
                reason,
                time_turns,
                components_text,
                qualities_text,
            })
        })
        .collect();

    // Craftable first, then alphabetical by display name
    entries.sort_by(|a, b| {
        b.craftable
            .cmp(&a.craftable)
            .then(a.result_name.cmp(&b.result_name))
    });

    let (show_all, last_message, filter, filtering) = world
        .get_resource::<CraftState>()
        .map(|s| {
            (
                s.show_all,
                s.last_message.clone(),
                s.filter.clone(),
                s.filtering,
            )
        })
        .unwrap_or((true, None, String::new(), false));

    let model = CraftModel { entries };
    let mut state = CraftState {
        focus: 0,
        show_all,
        last_message,
        filter,
        filtering,
    };
    state.focus = category_entry_indices(&model, &state, &cat_index)
        .iter()
        .position(|&i| Some(&model.entries[i].recipe_key) == selected_key.as_ref())
        .unwrap_or(0);
    if let Some(mut current) = world.get_resource_mut::<CraftModel>() {
        current.set_if_neq(model);
    } else {
        world.insert_resource(model);
    }
    if let Some(mut current) = world.get_resource_mut::<CraftState>() {
        current.set_if_neq(state);
    } else {
        world.insert_resource(state);
    }
}

fn slot_has_alternatives(world: &World, re: Entity, first_id: ItemTypeId) -> bool {
    world
        .get::<RecipeComponents>(re)
        .map(|comps| {
            comps
                .0
                .iter()
                .any(|slot| slot.iter().any(|e| e.item_id == first_id) && slot.len() > 1)
        })
        .unwrap_or(false)
}
// ---------------------------------------------------------------------------
// on_examine_item_changed — dynamic resume-craft action
// ---------------------------------------------------------------------------

/// Runs when `ExaminedItem` changes (user selects a different item in the
/// examine overlay).  Checks whether the new item has an `InProgressCraft`
/// and appends the "resume craft" action if so.
pub fn on_examine_item_changed(
    examined: Res<ExaminedItem>,
    in_progress_q: Query<&InProgressCraft>,
    mut ctx_actions: ResMut<ContextActions>,
) {
    if !examined.is_changed() {
        return;
    }
    let has_in_progress = examined
        .0
        .map(|e| in_progress_q.get(e).is_ok())
        .unwrap_or(false);

    if has_in_progress {
        ctx_actions.push("resume craft", BindableAction::HotkeyR);
    }
}

/// Bounded ECS invalidation: only craft/catalog/inventory inputs rebuild the model.
pub fn craft_model_changed(
    revision: Option<Res<CraftRevision>>,
    definitions: Option<Res<DefinitionWorld>>,
    recipes: Option<Res<RecipeIndex>>,
    changed: Query<
        (),
        (
            Without<cdda_components::def::IsDef>,
            Or<(
                Changed<cdda_components::item::ItemType>,
                Changed<cdda_components::item::StackCount>,
                Changed<cdda_components::item::ItemQualities>,
                Changed<cdda_components::item::InsideContainer>,
                Changed<cdda_components::sim::WorldPosition>,
                Changed<cdda_components::item::WieldedBy>,
                Changed<cdda_components::item::WornOn>,
                Changed<cdda_components::item::MountedOn>,
                Changed<cdda_components::item::Sealed>,
            )>,
        ),
    >,
    mut removed_items: RemovedComponents<cdda_components::item::ItemType>,
    mut removed_location: RemovedComponents<cdda_components::item::InsideContainer>,
    mut removed_qualities: RemovedComponents<cdda_components::item::ItemQualities>,
    mut removed_wielded: RemovedComponents<cdda_components::item::WieldedBy>,
    mut removed_worn: RemovedComponents<cdda_components::item::WornOn>,
    mut removed_mounted: RemovedComponents<cdda_components::item::MountedOn>,
    mut removed_sealed: RemovedComponents<cdda_components::item::Sealed>,
    mut removed_position: RemovedComponents<cdda_components::sim::WorldPosition>,
    mut removed_count: RemovedComponents<cdda_components::item::StackCount>,
) -> bool {
    // Drain every reader even when another dependency is already dirty.
    let removed = removed_items.read().count()
        + removed_location.read().count()
        + removed_qualities.read().count()
        + removed_wielded.read().count()
        + removed_worn.read().count()
        + removed_mounted.read().count()
        + removed_sealed.read().count()
        + removed_position.read().count()
        + removed_count.read().count();
    removed > 0
        || !changed.is_empty()
        || revision.is_some_and(|r| r.is_changed())
        || definitions.is_some_and(|r| r.is_changed())
        || recipes.is_some_and(|r| r.is_changed())
}

/// Rebuild after a relevant committed change; preserve selected recipe/category.
pub fn refresh_craft_state(world: &mut World) {
    build_craft_state(world);
    use cdda_sim::crafting::systems::CraftOutcome;
    let message = world
        .get_resource::<CraftRevision>()
        .and_then(|r| r.last_result.as_ref())
        .map(|outcome| match outcome {
            CraftOutcome::Started { craft } => format!(
                "Crafting: {}",
                world
                    .get::<InProgressCraft>(*craft)
                    .map(|c| c.result_name.as_str())
                    .unwrap_or("item")
            ),
            CraftOutcome::Completed { item } => format!(
                "Crafted: {}",
                world
                    .get::<ItemName>(*item)
                    .map(|n| n.0.as_str())
                    .unwrap_or("item")
            ),
            CraftOutcome::Interrupted { .. } => "Craft interrupted; progress saved".into(),
            CraftOutcome::Failed { reason } => format!("Failed: {reason}"),
        });
    if let Some(mut state) = world.get_resource_mut::<CraftState>() {
        if state.last_message != message {
            state.last_message = message;
        }
    }
}

/// Linear category membership lookup, shared with the input adapter so crafting
/// confirms exactly the entry shown in the list.
pub(crate) fn category_entry_indices(
    model: &CraftModel,
    state: &CraftState,
    categories: &CategoryIndex,
) -> Vec<usize> {
    let Some(top) = categories.top_categories.get(categories.selected_top) else {
        return Vec::new();
    };
    let subcategories: Vec<_> = categories
        .sub_recipes
        .iter()
        .filter(|((category, _), _)| category == top)
        .collect();
    let Some((_, recipes)) = subcategories.get(
        categories
            .selected_sub
            .min(subcategories.len().saturating_sub(1)),
    ) else {
        return Vec::new();
    };
    let recipes: std::collections::HashSet<Entity> = recipes.iter().copied().collect();
    let filter = state.filter.to_lowercase();
    model
        .entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| {
            recipes.contains(&entry.recipe_entity)
                && (state.show_all || entry.craftable)
                && (filter.is_empty()
                    || entry.result_name.to_lowercase().contains(&filter)
                    || entry.result_id.to_lowercase().contains(&filter))
        })
        .map(|(i, _)| i)
        .collect()
}
