//! Crafting system — recipe validation, component consumption, and craft execution.

use bevy_ecs::prelude::*;

use cdda_components::activity::{ActivityPhase, ActivityProgress, Crafting};
use cdda_components::def::{
    ItemName, RecipeComponents, RecipeQualities, RecipeResult, RecipeResultCount, RecipeTime,
};
use cdda_components::dev::DevPlayer;
use cdda_components::item::{
    InProgressCraft, InsideContainer, ItemQualities, ItemType, MountedPockets, QualityId,
    StackCount,
};
pub use cdda_components::recipe::RecipeIndex;
use cdda_components::sim::WorldPosition;
use cdda_components::ItemTypeId;

use crate::item::spawn::PreparedItem;
use cdda_catalog::definition::DefinitionWorld;
use cdda_catalog::interner::ItemTypeRegistry;
use cdda_core_types::core::coords::TILES_PER_OMT;
// ---------------------------------------------------------------------------
// Item collection
// ---------------------------------------------------------------------------

/// Collect accessible nested inventory items through the shared ownership traversal,
/// followed by unowned ground items in the same OMT (legacy crafting reach).
pub fn collect_available_items(world: &mut World, player: Entity) -> Vec<Entity> {
    let player_pos = world.get::<WorldPosition>(player).map(|wp| wp.0);
    let mut items = crate::inventory::systems::all_items_for_creature(player, world);
    items.retain(|&e| {
        crate::inventory::capacity::check_access(
            world,
            crate::inventory::capacity::parent(world, e),
        )
        .is_ok()
    });

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
                    && world.get::<cdda_components::def::IsDef>(*e).is_none()
                    && world.get::<cdda_components::actor::IsAlive>(*e).is_none()
                    && crate::inventory::transfer::location_root(world, *e) == Ok(*e)
                    && wp.0.x.div_euclid(TILES_PER_OMT) == px
                    && wp.0.y.div_euclid(TILES_PER_OMT) == py
                    && wp.0.z == pz
            })
            .map(|(e, _)| e)
            .collect();
        items.extend(ground);
    }

    let mut seen = std::collections::HashSet::new();
    items.retain(|e| seen.insert(*e));
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

    ingredient_plan(world, recipe_entity, available)?;

    Ok(())
}

/// Reserve counts across ALL slots before committing. Backtrack alternatives so
/// an early flexible slot cannot steal the only item satisfying a later slot.
fn ingredient_plan(
    world: &World,
    recipe: Entity,
    available: &[Entity],
) -> Result<Vec<(ItemTypeId, u32)>, String> {
    use std::collections::{HashMap, HashSet};
    let mut counts = HashMap::<ItemTypeId, u64>::new();
    let mut seen = HashSet::new();
    for &entity in available {
        if !seen.insert(entity) {
            continue;
        }
        if let Some(item) = world.get::<ItemType>(entity) {
            *counts.entry(item.0).or_default() += world
                .get::<StackCount>(entity)
                .map(|n| n.get())
                .unwrap_or(1) as u64;
        }
    }
    let Some(components) = world.get::<RecipeComponents>(recipe) else {
        return Ok(Vec::new());
    };
    fn allocate(
        slots: &[Vec<cdda_components::def::RecipeComponentEntry>],
        counts: &mut HashMap<ItemTypeId, u64>,
        plan: &mut Vec<(ItemTypeId, u32)>,
    ) -> bool {
        let Some((slot, rest)) = slots.split_first() else {
            return true;
        };
        for entry in slot {
            let remaining = counts.get(&entry.item_id).copied().unwrap_or(0);
            if entry.count == 0 || remaining < entry.count as u64 {
                continue;
            }
            counts.insert(entry.item_id, remaining - entry.count as u64);
            plan.push((entry.item_id, entry.count));
            if allocate(rest, counts, plan) {
                return true;
            }
            plan.pop();
            counts.insert(entry.item_id, remaining);
        }
        false
    }
    let mut plan = Vec::new();
    if allocate(&components.0, &mut counts, &mut plan) {
        Ok(plan)
    } else {
        Err("Insufficient ingredients across recipe slots".into())
    }
}

fn validate_crafter(world: &World, player: Entity) -> Result<(), String> {
    use cdda_components::actor::{ActionPoints, IsAlive};
    if world.get::<IsAlive>(player).is_none() {
        return Err("Crafter is not alive".into());
    }
    if !world
        .get::<ActionPoints>(player)
        .is_some_and(|ap| ap.current >= 0)
    {
        return Err("Crafter has no available action budget".into());
    }
    Ok(())
}

pub(crate) fn validate_craft_access(
    world: &World,
    player: Entity,
    craft: Entity,
) -> Result<(), String> {
    if !crate::inventory::transfer::belongs_to(world, craft, player) {
        return Err("Craft is not owned by this actor".into());
    }
    crate::inventory::capacity::check_access(
        world,
        crate::inventory::capacity::parent(world, craft),
    )
    .map_err(|_| "Craft is inaccessible".to_string())
}

struct CraftPlan {
    available: Vec<Entity>,
    consume: Vec<(ItemTypeId, u32)>,
    output: PreparedItem,
    count: u32,
    work_ap: i32,
    owner: Entity,
    name: String,
}
fn prepare_craft(world: &mut World, player: Entity, recipe: Entity) -> Result<CraftPlan, String> {
    if world.get_entity(player).is_err() {
        return Err("Crafter no longer exists".into());
    }
    if world
        .get::<ActivityProgress>(player)
        .is_some_and(|a| a.phase != ActivityPhase::Done)
    {
        return Err("Another activity is already in progress".into());
    }
    let result = world
        .get::<RecipeResult>(recipe)
        .ok_or("Recipe has no result")?
        .0
        .clone();
    let def = world
        .get_resource::<DefinitionWorld>()
        .and_then(|d| d.entity_by_str(&result))
        .ok_or_else(|| format!("Unknown item def: {result}"))?;
    let output = PreparedItem::from_definition(world, def)?;
    let count = world
        .get::<RecipeResultCount>(recipe)
        .map(|n| n.0)
        .unwrap_or(1);
    if count == 0 {
        return Err("Recipe result count must be positive".into());
    }
    let turns = world.get::<RecipeTime>(recipe).map(|t| t.0).unwrap_or(1);
    let work_ap = turns
        .checked_mul(100)
        .and_then(|v| i32::try_from(v.max(100)).ok())
        .ok_or("Recipe work exceeds activity budget range")?;
    let available = collect_available_items(world, player);
    check_can_craft(world, recipe, &available)?;
    let consume = ingredient_plan(world, recipe, &available)?;
    let owner = world
        .get::<MountedPockets>(player)
        .and_then(|p| p.iter().next())
        .unwrap_or(player);
    output.validate_spawn(world, owner, count)?;
    let name = world.get::<ItemName>(def).unwrap().0.clone();
    Ok(CraftPlan {
        available,
        consume,
        output,
        count,
        work_ap,
        owner,
        name,
    })
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
/// the activity budget once enough AP has been invested.
///
/// Returns the `InProgressCraft` entity on success.
pub fn start_craft(
    world: &mut World,
    player: Entity,
    recipe_entity: Entity,
) -> Result<Entity, String> {
    validate_crafter(world, player)?;
    let plan = prepare_craft(world, player, recipe_entity)?;
    crate::activity::lifecycle::interrupt_activity(world, player);
    let result_id = plan.output.key.clone();
    let result_count = plan.count;
    let result_name = plan.name;
    let ap_total = plan.work_ap;
    for (id, count) in &plan.consume {
        consume_items(world, &plan.available, *id, *count);
    }
    let body_pocket = plan.owner;
    world.init_resource::<ItemTypeRegistry>();
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
            plan.output,
            ItemType(craft_type_token),
            InsideContainer(body_pocket),
        ))
        .id();

    // Attach ActivityProgress + Crafting so the activity system drives the craft.
    // Phase is set to Active immediately — start logic has already computed ap_total.
    let progress = {
        let mut p = ActivityProgress::new(ap_total);
        p.phase = ActivityPhase::Active;
        p
    };
    world
        .entity_mut(player)
        .insert((progress, Crafting { craft_entity }));

    Ok(craft_entity)
}

// ---------------------------------------------------------------------------
// complete_craft — finish an in-progress craft
// ---------------------------------------------------------------------------

/// Despawn `craft_entity`, spawn the result item in `player`'s inventory.
pub fn complete_craft(
    world: &mut World,
    player: Entity,
    craft_entity: Entity,
) -> Result<Entity, String> {
    let craft = world
        .get::<InProgressCraft>(craft_entity)
        .ok_or("Craft no longer exists")?
        .clone();
    validate_craft_access(world, player, craft_entity)?;
    if !craft.is_complete() {
        return Err("Craft still requires work".into());
    }
    let output = if let Some(prepared) = world.get::<PreparedItem>(craft_entity) {
        prepared.clone()
    } else {
        // Legacy in-progress items can resolve by stable key. Failure retains
        // the craft entity, so completion can be retried after content is restored.
        let definition = world
            .get_resource::<DefinitionWorld>()
            .and_then(|d| d.entity_by_str(&craft.result_id))
            .ok_or("Craft output definition unavailable")?;
        PreparedItem::from_definition(world, definition)?
    };
    if world
        .get::<cdda_components::actor::IsAlive>(player)
        .is_none()
    {
        return Err("Crafter is not alive".into());
    }
    let owner = world
        .get::<MountedPockets>(player)
        .and_then(|p| p.iter().next())
        .unwrap_or(player);
    let item = output.spawn(world, owner, craft.result_count)?;
    world.despawn(craft_entity);
    Ok(item)
}

// ---------------------------------------------------------------------------
// resume_craft — re-attach activity to an interrupted in-progress craft
// ---------------------------------------------------------------------------

/// Resume an interrupted `InProgressCraft` by re-attaching activity components
/// on the player.  The craft picks up where it left off
///
/// Returns `Ok(())` if the activity was created, or `Err` if the craft entity
/// no longer exists or the player already has an active craft.
pub fn resume_craft(world: &mut World, player: Entity, craft_entity: Entity) -> Result<(), String> {
    resume_craft_with_outcome(world, player, craft_entity).map(|_| ())
}

/// Resume command implementation with a semantic result, including output retry.
pub fn resume_craft_with_outcome(
    world: &mut World,
    player: Entity,
    craft_entity: Entity,
) -> Result<CraftOutcome, String> {
    validate_crafter(world, player)?;
    validate_craft_access(world, player, craft_entity)?;
    if let Some(progress) = world
        .get::<ActivityProgress>(player)
        .filter(|p| p.phase != ActivityPhase::Done)
    {
        if world
            .get::<Crafting>(player)
            .is_some_and(|c| c.craft_entity == craft_entity)
        {
            if progress.phase == ActivityPhase::Suspended {
                world.get_mut::<ActivityProgress>(player).unwrap().phase = ActivityPhase::Active;
            }
            return Ok(CraftOutcome::Started {
                craft: craft_entity,
            });
        }
        return Err("Another activity is already in progress".into());
    }
    let ap_remaining = world
        .get::<InProgressCraft>(craft_entity)
        .map(|c| c.ap_total.saturating_sub(c.ap_spent))
        .ok_or("Craft entity no longer exists")?;
    if ap_remaining <= 0 {
        let item = complete_craft(world, player, craft_entity)?;
        crate::activity::lifecycle::interrupt_activity(world, player);
        return Ok(CraftOutcome::Completed { item });
    }
    crate::activity::lifecycle::interrupt_activity(world, player);

    let progress = {
        let mut p = ActivityProgress::new(ap_remaining);
        p.phase = ActivityPhase::Active;
        p
    };
    world
        .entity_mut(player)
        .insert((progress, Crafting { craft_entity }));

    Ok(CraftOutcome::Started {
        craft: craft_entity,
    })
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
    let plan = prepare_craft(world, player, recipe_entity)?;
    // Spawn validation precedes input consumption; commit is synchronous.
    let item = plan.output.spawn(world, plan.owner, plan.count)?;
    for (id, count) in plan.consume {
        consume_items(world, &plan.available, id, count);
    }
    Ok(item)
}

/// Legacy single-player menu mailbox. The turn ingress translates it into
/// StartCraft without replacing an existing intent. Native callers submit intents.
#[derive(Resource, Default)]
pub struct PendingCraft(pub Option<Entity>);

/// Semantic execution revision; presentation adapters choose their own labels and selection.
#[derive(Resource, Default)]
pub struct CraftRevision {
    pub revision: u64,
    pub last_result: Option<CraftOutcome>,
}

/// Committed operation status; screen adapters own its display text.
#[derive(Debug, Clone)]
pub enum CraftOutcome {
    Started { craft: Entity },
    Completed { item: Entity },
    Interrupted { craft: Entity },
    Failed { reason: String },
}

/// Publish one committed craft operation or validation failure.
pub fn publish_craft_outcome(world: &mut World, outcome: CraftOutcome) {
    world.init_resource::<CraftRevision>();
    let mut revision = world.resource_mut::<CraftRevision>();
    revision.last_result = Some(outcome);
    revision.revision += 1;
}
