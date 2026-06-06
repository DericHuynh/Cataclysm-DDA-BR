//! Crafting system — recipe validation, component consumption, and craft execution.

use bevy_ecs::prelude::*;

use cdda_activity::actor::{ActivityActor, CraftActor};
use cdda_activity::components::{ActivityPhase, PlayerActivity};
use cdda_components::def::{
    ItemName, RecipeCategory, RecipeComponents, RecipeQualities, RecipeResult, RecipeResultCount,
    RecipeSubcategory, RecipeTime,
};
use cdda_components::dev::DevPlayer;
use cdda_components::input::BindableAction;
use cdda_components::item::{
    ContainerContents, InProgressCraft, InsideContainer, ItemQualities, ItemType, MountedPockets,
    QualityId, StackCount, WieldedItems,
};
pub use cdda_components::recipe::RecipeIndex;
use cdda_components::sim::WorldPosition;
use cdda_components::ItemTypeId;

use cdda_context::ContextActions;
use cdda_core_types::core::coords::{WorldPos, ZLevel, TILES_PER_OMT};
use cdda_data::def_world::DefinitionWorld;
use cdda_data::interner::ItemTypeRegistry;
use cdda_inventory::examine_resource::ExaminedItem;
// Stub: spawn_item_from_def (module cdda_overmap_gen::spawning doesn't exist yet).
// TODO: move to cdda_overmap_gen::spawning when implemented.
fn spawn_item_from_def(
    world: &mut World,
    _def_entity: Entity,
    _pos: WorldPos,
    _count: u32,
) -> Entity {
    world.spawn_empty().id()
}

// ---------------------------------------------------------------------------
// CategoryIndex — tabbed category navigation for the crafting menu
// ---------------------------------------------------------------------------

/// Maps recipe category → subcategory → recipe entity list.
/// Built in `build_craft_state` by iterating recipe entities and reading
/// `RecipeCategory` / `RecipeSubcategory` / `IsRecipeDef` components.
#[derive(Resource, Default, Debug, Clone)]
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
// Item collection
// ---------------------------------------------------------------------------

/// Collect all item entities available for crafting:
/// items in the player's body pockets (via MountedPockets -> ContainerContents),
/// wielded items (WieldedItems), fallback direct ContainerContents on player,
/// plus items on the ground in the same OMT tile.
pub fn collect_available_items(world: &mut World, player: Entity) -> Vec<Entity> {
    let player_pos = world.get::<WorldPosition>(player).map(|wp| wp.0);

    let mut items: Vec<Entity> = Vec::new();

    // Primary: items in pockets mounted on the player.
    if let Some(pockets) = world.get::<MountedPockets>(player) {
        let pocket_list: Vec<Entity> = pockets.iter().collect();
        for pocket in pocket_list {
            if let Some(cc) = world.get::<ContainerContents>(pocket) {
                for e in cc.iter() {
                    if !items.contains(&e) {
                        items.push(e);
                    }
                }
            }
        }
    }

    // Wielded items.
    if let Some(wi) = world.get::<WieldedItems>(player) {
        for e in wi.iter() {
            if !items.contains(&e) {
                items.push(e);
            }
        }
    }

    // Backwards-compat: direct ContainerContents on player (tests + old save data).
    if let Some(cc) = world.get::<ContainerContents>(player) {
        for e in cc.iter() {
            if !items.contains(&e) {
                items.push(e);
            }
        }
    }

    // Items on the ground within the same 24x24 OMT tile as the player
    if let Some(pos) = player_pos {
        let px = pos.x.div_euclid(TILES_PER_OMT);
        let py = pos.y.div_euclid(TILES_PER_OMT);
        let pz = pos.z;

        let mut q = world.query::<(Entity, &WorldPosition)>();
        let ground: Vec<Entity> = q
            .iter(world)
            .filter(|(e, wp)| {
                *e != player
                    && wp.0.x.div_euclid(TILES_PER_OMT) == px
                    && wp.0.y.div_euclid(TILES_PER_OMT) == py
                    && wp.0.z == pz
            })
            .map(|(e, _)| e)
            .collect();
        items.extend(ground);
    }

    items
}

// ---------------------------------------------------------------------------
// Availability helpers
// ---------------------------------------------------------------------------

/// Sum `StackCount` across all items in `available` whose `ItemType` matches.
pub fn count_available(world: &World, available: &[Entity], type_id: ItemTypeId) -> u32 {
    available
        .iter()
        .filter_map(|&e| {
            let matches = world
                .get::<ItemType>(e)
                .map(|t| t.0 == type_id)
                .unwrap_or(false);
            matches.then(|| world.get::<StackCount>(e).map(|s| s.get()).unwrap_or(1))
        })
        .sum()
}

/// Return `true` if any item in `available` has `quality_id` at `>= min_level`.
pub fn has_quality(
    world: &World,
    available: &[Entity],
    quality_id: QualityId,
    min_level: u32,
) -> bool {
    available.iter().any(|&e| {
        world
            .get::<ItemQualities>(e)
            .map(|iq| {
                iq.0.iter()
                    .any(|(qid, lvl)| *qid == quality_id && *lvl >= min_level as i32)
            })
            .unwrap_or(false)
    })
}

/// Check whether `available` satisfies all requirements of `recipe_entity`.
///
/// Returns `Ok(())` if craftable, `Err(reason)` otherwise.
pub fn check_can_craft(
    world: &World,
    recipe_entity: Entity,
    available: &[Entity],
) -> Result<(), String> {
    // Quality requirements
    if let Some(quals) = world.get::<RecipeQualities>(recipe_entity) {
        for (quality_id, min_level) in &quals.0 {
            if !has_quality(world, available, *quality_id, *min_level) {
                return Err(format!("Need quality {:?} level {}", quality_id, min_level));
            }
        }
    }

    // Component requirements — each slot must be met by at least one alternative
    if let Some(comps) = world.get::<RecipeComponents>(recipe_entity) {
        for slot in &comps.0 {
            let satisfied = slot
                .iter()
                .any(|entry| count_available(world, available, entry.item_id) >= entry.count);
            if !satisfied {
                let needed: Vec<String> = slot
                    .iter()
                    .map(|e| format!("{:?} x{}", e.item_id, e.count))
                    .collect();
                return Err(format!("Need: {}", needed.join(" OR ")));
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Component consumption
// ---------------------------------------------------------------------------

/// Deduct `needed` items of `type_id` from `available`.
/// Decrements `StackCount`; despawns the entity when the stack reaches zero.
///
/// Since inventory tracking is now relationship-based,
/// despawning the item entity automatically removes it from `ContainerContents`
/// and cleans up `Invlet` components.
pub fn consume_items(
    world: &mut World,
    available: &[Entity],
    type_id: ItemTypeId,
    mut needed: u32,
) {
    for &e in available {
        if needed == 0 {
            break;
        }
        let matches = world
            .get::<ItemType>(e)
            .map(|t| t.0 == type_id)
            .unwrap_or(false);
        if !matches {
            continue;
        }
        let stack = world.get::<StackCount>(e).map(|s| s.get()).unwrap_or(1);
        if stack <= needed {
            needed -= stack;
            // Despawning the entity automatically removes it from all
            // relationships (InsideContainer, ContainerContents, etc.)
            // and cleans up Invlet components.
            world.despawn(e);
        } else {
            world.entity_mut(e).insert(
                StackCount::new(stack - needed).expect("stack > needed in this branch, count >= 1"),
            );
            needed = 0;
        }
    }
}

// ---------------------------------------------------------------------------
// Player lookup
// ---------------------------------------------------------------------------

/// Return the entity of the dev-world player, if any.
pub fn find_dev_player(world: &mut World) -> Option<Entity> {
    let mut q = world.query_filtered::<Entity, With<DevPlayer>>();
    q.iter(world).next()
}

// ---------------------------------------------------------------------------
// Craft execution
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// start_craft — begin an in-progress craft
// ---------------------------------------------------------------------------

/// Validate requirements, consume components, and spawn an `InProgressCraft`
/// entity in `player`'s inventory.  The result item is produced later by
/// `continue_crafts` once enough AP has been invested.
///
/// Returns the `InProgressCraft` entity on success.
pub fn start_craft(
    world: &mut World,
    player: Entity,
    recipe_entity: Entity,
) -> Result<Entity, String> {
    let available = collect_available_items(world, player);
    check_can_craft(world, recipe_entity, &available)?;

    // Build consume plan from available components.
    let consume_plan: Vec<(ItemTypeId, u32)> = world
        .get::<RecipeComponents>(recipe_entity)
        .map(|comps| {
            comps
                .0
                .iter()
                .filter_map(|slot| {
                    slot.iter()
                        .find(|entry| {
                            count_available(world, &available, entry.item_id) >= entry.count
                        })
                        .map(|entry| (entry.item_id, entry.count))
                })
                .collect()
        })
        .unwrap_or_default();

    for (type_id, count) in &consume_plan {
        consume_items(world, &available, *type_id, *count);
    }

    // Gather result metadata.
    let result_id = world
        .get::<RecipeResult>(recipe_entity)
        .map(|r| r.0.clone())
        .ok_or_else(|| "Recipe has no result".to_string())?;

    let result_count = world
        .get::<RecipeResultCount>(recipe_entity)
        .map(|c| c.0)
        .unwrap_or(1);

    // Look up the display name from the definition world.
    let result_name = world
        .get_resource::<DefinitionWorld>()
        .and_then(|dw| dw.entity_by_str(&result_id))
        .and_then(|de| world.get::<ItemName>(de).map(|n| n.0.clone()))
        .unwrap_or_else(|| result_id.clone());

    // RecipeTime is in turns; multiply by 100 for AP (speed=100 baseline).
    let ap_total = world
        .get::<RecipeTime>(recipe_entity)
        .map(|t| (t.0 as i32 * 100).max(100))
        .unwrap_or(100);

    // Spawn the in-progress entity into the player's body pocket.
    let body_pocket = world
        .get::<MountedPockets>(player)
        .and_then(|mp| mp.iter().next())
        .unwrap_or(player);

    let craft_type_token = world
        .resource_mut::<ItemTypeRegistry>()
        .intern(&format!("craft:in_progress:{result_id}"));
    let craft_entity = world
        .spawn((
            InProgressCraft {
                recipe_entity,
                result_id: result_id.clone(),
                result_name,
                result_count,
                ap_total,
                ap_spent: 0,
            },
            ItemType(craft_type_token),
            InsideContainer(body_pocket),
        ))
        .id();

    // Attach a PlayerActivity so the activity system drives the craft.
    // Phase starts as Active — start() has already been accounted for via ap_total.
    let actor = ActivityActor::Craft(CraftActor { craft_entity });
    world.entity_mut(player).insert({
        let mut act = PlayerActivity::new("ACT_CRAFT", actor);
        act.moves_total = ap_total;
        act.moves_left = ap_total;
        act.phase = ActivityPhase::Active;
        act
    });

    Ok(craft_entity)
}

// ---------------------------------------------------------------------------
// complete_craft — finish an in-progress craft
// ---------------------------------------------------------------------------

/// Despawn `craft_entity`, spawn the result item in `player`'s inventory.
pub fn complete_craft(world: &mut World, player: Entity, craft_entity: Entity) {
    let (result_id, result_count) = {
        let Some(craft) = world.get::<InProgressCraft>(craft_entity) else {
            return;
        };
        (craft.result_id.clone(), craft.result_count)
    };

    // Despawn the craft entity — relationships and components are cleaned up
    // automatically by Bevy.
    world.despawn(craft_entity);

    // Spawn the result item.
    let player_pos = world
        .get::<WorldPosition>(player)
        .map(|wp| wp.0)
        .unwrap_or_else(|| WorldPos::new(0, 0, ZLevel::new(0)));

    let def_entity = world
        .get_resource::<DefinitionWorld>()
        .and_then(|dw| dw.entity_by_str(&result_id));

    if let Some(def_entity) = def_entity {
        let crafted = spawn_item_from_def(world, def_entity, player_pos, result_count);

        // Intern the result_id for ItemTypeId
        let type_token = world.resource_mut::<ItemTypeRegistry>().intern(&result_id);
        if world.get::<ItemType>(crafted).is_none() {
            world.entity_mut(crafted).insert(ItemType(type_token));
        }

        let body_pocket = world
            .get::<MountedPockets>(player)
            .and_then(|mp| mp.iter().next())
            .unwrap_or(player);

        world
            .entity_mut(crafted)
            .remove::<WorldPosition>()
            .insert(InsideContainer(body_pocket));

        tracing::info!("Craft complete: {}", result_id);
    }
}

// ---------------------------------------------------------------------------
// resume_craft — re-attach activity to an interrupted in-progress craft
// ---------------------------------------------------------------------------

/// Resume an interrupted `InProgressCraft` by re-attaching a `PlayerActivity`
/// with `CraftActor` on the player.  The craft picks up where it left off
/// (`ap_spent` is preserved).
///
/// Returns `Ok(())` if the activity was created, or `Err` if the craft entity
/// no longer exists or the player already has an active craft.
pub fn resume_craft(world: &mut World, player: Entity, craft_entity: Entity) -> Result<(), String> {
    let ap_remaining = world
        .get::<InProgressCraft>(craft_entity)
        .map(|c| c.ap_total.saturating_sub(c.ap_spent))
        .ok_or_else(|| "Craft entity no longer exists".to_string())?;

    if ap_remaining <= 0 {
        // Craft is already complete — finish it immediately.
        complete_craft(world, player, craft_entity);
        return Ok(());
    }

    // Don't overwrite an existing activity.
    if world
        .get::<PlayerActivity>(player)
        .map(|a| a.phase != ActivityPhase::Done)
        .unwrap_or(false)
    {
        // If the existing activity is also a craft, just return — already crafting.
        if world
            .get::<PlayerActivity>(player)
            .map(|a| a.activity_type.as_str() == "ACT_CRAFT")
            .unwrap_or(false)
        {
            return Ok(());
        }
        return Err("Another activity is already in progress".to_string());
    }

    let actor = ActivityActor::Craft(CraftActor { craft_entity });
    world.entity_mut(player).insert({
        let mut act = PlayerActivity::new("ACT_CRAFT", actor);
        act.moves_total = ap_remaining;
        act.moves_left = ap_remaining;
        act.phase = ActivityPhase::Active;
        act
    });

    Ok(())
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

// ---------------------------------------------------------------------------
// continue_crafts — tick in-progress crafts each turn
// ---------------------------------------------------------------------------

/// Advance the player's in-progress craft by one tick.
///
/// Delegates to the `CraftActor` via the activity system. Each call spends
/// `AP_COST_CRAFT_TICK` from the player's `ActionPoints`, advances
/// `InProgressCraft::ap_spent`, and spawns the result item once complete.
///
/// Called each game turn from `cdda_app` (and directly by tests).
pub fn continue_crafts(world: &mut World) {
    let Some(player) = find_dev_player(world) else {
        return;
    };

    let has_active_craft = world
        .get::<PlayerActivity>(player)
        .map(|a| a.activity_type.as_str() == "ACT_CRAFT" && a.phase == ActivityPhase::Active)
        .unwrap_or(false);

    if has_active_craft {
        cdda_activity::systems::tick_one(world, player);
    }
}

// ---------------------------------------------------------------------------
// do_craft — immediate craft (legacy helper, used by tests)
// ---------------------------------------------------------------------------

/// Validate requirements, consume components, and spawn the result item
/// directly into `player`'s inventory (no AP cost / in-progress step).
///
/// Prefer `start_craft` for gameplay; this exists for tests and one-shot
/// dev commands where AP tracking is not needed.
pub fn do_craft(
    world: &mut World,
    player: Entity,
    recipe_entity: Entity,
) -> Result<Entity, String> {
    let available = collect_available_items(world, player);
    check_can_craft(world, recipe_entity, &available)?;

    // Plan: which alternative to consume for each component slot
    let consume_plan: Vec<(ItemTypeId, u32)> = world
        .get::<RecipeComponents>(recipe_entity)
        .map(|comps| {
            comps
                .0
                .iter()
                .filter_map(|slot| {
                    slot.iter()
                        .find(|entry| {
                            count_available(world, &available, entry.item_id) >= entry.count
                        })
                        .map(|entry| (entry.item_id, entry.count))
                })
                .collect()
        })
        .unwrap_or_default();

    // Consume components (live reads from world — handles partial stacks correctly)
    for (type_id, count) in &consume_plan {
        consume_items(world, &available, *type_id, *count);
    }

    // Resolve result item def
    let result_id = world
        .get::<RecipeResult>(recipe_entity)
        .map(|r| r.0.clone())
        .ok_or_else(|| "Recipe has no result".to_string())?;

    let result_count = world
        .get::<RecipeResultCount>(recipe_entity)
        .map(|c| c.0)
        .unwrap_or(1);

    let def_entity = world
        .get_resource::<DefinitionWorld>()
        .and_then(|dw| dw.entity_by_str(&result_id))
        .ok_or_else(|| format!("Unknown item def: {}", result_id))?;

    let player_pos = world
        .get::<WorldPosition>(player)
        .map(|wp| wp.0)
        .unwrap_or_else(|| WorldPos::new(0, 0, ZLevel::new(0)));

    // Clone def entity into a runtime item
    let crafted = spawn_item_from_def(world, def_entity, player_pos, result_count);

    // Ensure ItemType is present for crafting/quality checks on next open
    let type_token = world.resource_mut::<ItemTypeRegistry>().intern(&result_id);
    if world.get::<ItemType>(crafted).is_none() {
        world.entity_mut(crafted).insert(ItemType(type_token));
    }

    // Move into body pocket (remove ground position, add containment).
    let body_pocket = world
        .get::<MountedPockets>(player)
        .and_then(|mp| mp.iter().next())
        .unwrap_or(player);

    world
        .entity_mut(crafted)
        .remove::<WorldPosition>()
        .insert(InsideContainer(body_pocket));

    Ok(crafted)
}

// ---------------------------------------------------------------------------
// CraftEntry / CraftState — UI-facing crafting data
// ---------------------------------------------------------------------------

/// One row in the crafting menu recipe list.
#[derive(Clone)]
pub struct CraftEntry {
    pub recipe_entity: Entity,
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
#[derive(Resource)]
pub struct CraftState {
    pub focus: usize,
    /// When `true`, shows all recipes; when `false`, shows only craftable ones.
    pub show_all: bool,
    pub entries: Vec<CraftEntry>,
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
            entries: Vec::new(),
            last_message: None,
            filter: String::new(),
            filtering: false,
        }
    }
}

impl CraftState {
    /// Entries matching the current filter (and show_all/craftable toggle).
    pub fn visible(&self) -> impl Iterator<Item = &CraftEntry> {
        let filter = self.filter.to_lowercase();
        self.entries.iter().filter(move |e| {
            (self.show_all || e.craftable)
                && (filter.is_empty()
                    || e.result_name.to_lowercase().contains(&filter)
                    || e.result_id.to_lowercase().contains(&filter))
        })
    }

    pub fn visible_count(&self) -> usize {
        self.visible().count()
    }

    pub fn focused_entry(&self) -> Option<&CraftEntry> {
        self.visible().nth(self.focus)
    }
}

/// Set by `crafting_menu_input` when the player confirms a craft.
/// Drained each frame by `process_pending_craft`.
#[derive(Resource, Default)]
pub struct PendingCraft(pub Option<Entity>);

// ---------------------------------------------------------------------------
// build_craft_state — exclusive state builder
// ---------------------------------------------------------------------------

/// Rebuild `CraftState` from the current world state.
/// Runs on `OnEnter(Ctx::CraftingMenu)` and after each craft completes.
pub fn build_craft_state(world: &mut World) {
    let Some(player) = find_dev_player(world) else {
        return;
    };

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
    cat_index.selected_top = 0;
    cat_index.selected_sub = 0;

    world.insert_resource(cat_index.clone());

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

    world.insert_resource(CraftState {
        focus: 0,
        show_all,
        entries,
        last_message,
        filter,
        filtering,
    });
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
