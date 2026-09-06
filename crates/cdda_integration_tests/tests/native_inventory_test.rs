//! Headless import / publication / simulation acceptance slice.
use bevy_app::App;
use bevy_ecs::prelude::*;
use cdda_catalog::{
    definition::{DefCategory, DefinitionWorld},
    inventory::*,
};
use cdda_components::{
    actor::{ActionPoints, IsAlive},
    def::*,
    item::*,
    sim::WorldPosition,
};
use cdda_data::{inventory_import::*, inventory_publish::publish_inventory};
use cdda_sim::{
    crafting::systems::{complete_craft, do_craft, start_craft},
    item::spawn::PreparedItem,
    runtime::{step_simulation, SimulationPlugin},
};
use serde_json::json;

fn documents() -> Vec<SourceDocument> {
    vec![SourceDocument {
        path: "fixture/core.json".into(),
        mod_id: "core".into(),
        values: serde_json::from_str(include_str!("fixtures/inventory_native.json")).unwrap(),
    }]
}
fn world() -> (App, Entity) {
    let catalog = import_inventory(documents())
        .strict_catalog()
        .unwrap()
        .clone();
    let mut app = App::new();
    app.add_plugins(SimulationPlugin);
    publish_inventory(app.world_mut(), catalog).unwrap();
    let player = app
        .world_mut()
        .spawn((
            IsAlive,
            ActionPoints {
                current: 1000,
                speed: 100,
            },
        ))
        .id();
    (app, player)
}
fn definition(world: &World, category: DefCategory, key: &str) -> Entity {
    world
        .resource::<DefinitionWorld>()
        .entity_in(category, key)
        .unwrap()
}
fn spawn(world: &mut World, owner: Entity, key: &str, count: u32) -> Entity {
    let definition = definition(world, DefCategory::Item, key);
    PreparedItem::from_definition(world, definition)
        .unwrap()
        .spawn(world, owner, count)
        .unwrap()
}
fn recipe(world: &World) -> Entity {
    definition(world, DefCategory::Recipe, "cord_from_fiber")
}

#[test]
fn normalization_preserves_recipe_variants_units_and_original_values() {
    let docs = documents();
    let imported = import_inventory(docs.clone());
    let catalog = imported.strict_catalog().unwrap();
    assert_eq!(catalog.recipes.len(), 2);
    assert_eq!(
        catalog.items[&ItemKey("bag".into())].pockets[0].volume_ml,
        2000
    );
    assert_eq!(
        catalog.recipes[&RecipeKey("cord_from_fiber".into())].work_ap,
        200
    );
    assert_eq!(imported.documents[0].values, docs[0].values);
}
#[test]
fn unsupported_and_unresolved_content_has_provenance_and_cannot_publish() {
    let mut docs = documents();
    docs[0].values[0]["use_action"] = json!({"type": "unimplemented"});
    docs[0].values[4]["components"] = json!([[["missing", 2]]]);
    let imported = import_inventory(docs);
    assert!(imported.strict_catalog().is_err());
    assert!(imported
        .diagnostics
        .iter()
        .any(|d| d.support == Support::PreservedUnimplemented
            && d.path == "/use_action"
            && d.mod_id == "core"));
    assert!(imported
        .diagnostics
        .iter()
        .any(|d| d.support == Support::Rejected
            && d.path == "/components/0/0"
            && d.source.to_str() == Some("fixture/core.json")));
    assert_eq!(
        imported.documents[0].values[0]["use_action"]["type"],
        "unimplemented"
    );
}
#[test]
fn failed_publication_leaves_resources_entities_and_generation_untouched() {
    let (mut app, player) = world();
    let w = app.world_mut();
    let item = spawn(w, player, "fiber", 2);
    let old_definition = definition(w, DefCategory::Item, "fiber");
    let generation = w.resource::<DefinitionWorld>().generation();
    let count = w.entities().len();
    let mut candidate = w.resource::<InventoryCatalog>().clone();
    candidate.items.remove(&ItemKey("cord".into()));
    assert!(publish_inventory(w, candidate).is_err());
    assert_eq!(w.entities().len(), count);
    assert_eq!(w.resource::<DefinitionWorld>().generation(), generation);
    assert_eq!(definition(w, DefCategory::Item, "fiber"), old_definition);
    assert_eq!(w.get::<StackCount>(item).unwrap().get(), 2);
}
#[test]
fn production_schedule_finishes_a_craft_across_definition_reload() {
    let (mut app, player) = world();
    let w = app.world_mut();
    let fiber = spawn(w, player, "fiber", 2);
    spawn(w, player, "knife", 1);
    let recipe = recipe(w);
    let craft = start_craft(w, player, recipe).unwrap();
    assert!(w.get_entity(fiber).is_err());
    let mut next = w.resource::<InventoryCatalog>().clone();
    let cord = std::sync::Arc::make_mut(next.items.get_mut(&ItemKey("cord".into())).unwrap());
    cord.name = "new cord".into();
    publish_inventory(w, next).unwrap();
    assert!(w.get_entity(recipe).is_err());
    for _ in 0..4 {
        step_simulation(w);
    }
    assert!(w.get_entity(craft).is_err());
    let mut items =
        w.query_filtered::<(&DefStrId, &ItemName, &StackCount, &ItemVolume), Without<IsDef>>();
    let (_, name, count, volume) = items.iter(w).find(|(id, _, _, _)| id.0 == "cord").unwrap();
    assert_eq!(
        name.0, "cord",
        "in-progress output retains its prepared definition"
    );
    assert_eq!(count.get(), 1);
    assert_eq!(volume.0, 25);
}
#[test]
fn missing_result_and_overlapping_slots_never_consume_partial_inputs() {
    let (mut app, player) = world();
    let w = app.world_mut();
    let fiber = spawn(w, player, "fiber", 2);
    spawn(w, player, "knife", 1);
    let recipe = recipe(w);
    w.entity_mut(recipe).insert(RecipeResult("missing".into()));
    assert!(start_craft(w, player, recipe).is_err());
    assert_eq!(w.get::<StackCount>(fiber).unwrap().get(), 2);
    w.entity_mut(recipe).insert(RecipeResult("cord".into()));
    let slot = w.get::<RecipeComponents>(recipe).unwrap().0[0].clone();
    w.entity_mut(recipe)
        .insert(RecipeComponents(vec![slot.clone(), slot]));
    assert!(do_craft(w, player, recipe).is_err());
    assert_eq!(w.get::<StackCount>(fiber).unwrap().get(), 2);
}
#[test]
fn alternatives_backtrack_without_double_consumption() {
    let (mut app, player) = world();
    let w = app.world_mut();
    let fiber = spawn(w, player, "fiber", 2);
    let knife = spawn(w, player, "knife", 1);
    let recipe = recipe(w);
    let fiber_id = w.get::<ItemType>(fiber).unwrap().0;
    let knife_id = w.get::<ItemType>(knife).unwrap().0;
    let entry = |item_id, count| RecipeComponentEntry {
        item_id,
        count,
        recovered: false,
    };
    w.entity_mut(recipe).insert(RecipeComponents(vec![
        vec![entry(fiber_id, 2), entry(knife_id, 1)],
        vec![entry(fiber_id, 2)],
    ]));
    do_craft(w, player, recipe).unwrap();
    assert!(w.get_entity(fiber).is_err());
    assert!(w.get_entity(knife).is_err());
    assert!(do_craft(w, player, recipe).is_err());
}
#[test]
fn pockets_are_independent_entities_and_native_keys_survive_serialization_order() {
    let (mut app, player) = world();
    let w = app.world_mut();
    let bag = spawn(w, player, "bag", 1);
    let pocket = w.get::<MountedPockets>(bag).unwrap().iter().next().unwrap();
    assert_eq!(w.get::<Pocket>(pocket).unwrap().max_weight.0, 5000);
    assert!(w.get::<WorldPosition>(bag).is_none());
    let encoded = serde_json::to_value(w.resource::<InventoryCatalog>()).unwrap();
    let decoded: InventoryCatalog = serde_json::from_value(encoded).unwrap();
    decoded.validate().unwrap();
    // A fresh session with a different interner order must retain item meaning.
    let mut restored = World::new();
    restored.init_resource::<cdda_catalog::interner::ItemTypeRegistry>();
    restored
        .resource_mut::<cdda_catalog::interner::ItemTypeRegistry>()
        .intern("different_first_token");
    publish_inventory(&mut restored, decoded.clone()).unwrap();
    let restored_owner = restored.spawn_empty().id();
    let restored_bag = spawn(&mut restored, restored_owner, "bag", 1);
    assert_ne!(
        restored.get::<ItemType>(restored_bag).unwrap().0,
        w.get::<ItemType>(bag).unwrap().0
    );
    assert_eq!(
        restored
            .get::<ItemDefinitionRef>(restored_bag)
            .unwrap()
            .0
            .key,
        ItemKey("bag".into())
    );
    publish_inventory(w, decoded).unwrap();
    assert_eq!(
        w.get::<ItemDefinitionRef>(bag).unwrap().0.key,
        ItemKey("bag".into())
    );
    w.despawn(bag);
    assert!(w.get_entity(pocket).is_err());
}
#[test]
fn failed_completion_keeps_legacy_in_progress_item() {
    let (mut app, player) = world();
    let w = app.world_mut();
    let recipe = recipe(w);
    let craft = w
        .spawn(InProgressCraft {
            recipe_entity: recipe,
            result_id: "missing".into(),
            result_name: "missing".into(),
            result_count: 1,
            ap_total: 100,
            ap_spent: 100,
        })
        .id();
    assert!(complete_craft(w, player, craft).is_err());
    assert!(w.get::<InProgressCraft>(craft).is_some());
}

#[test]
fn competing_actors_cannot_consume_the_same_ground_stack() {
    use cdda_core_types::core::coords::{WorldPos, ZLevel};
    let (mut app, player) = world();
    let w = app.world_mut();
    let position = WorldPosition(WorldPos::new(0, 0, ZLevel::new(0)));
    w.entity_mut(player).insert(position);
    let other = w
        .spawn((
            position,
            IsAlive,
            ActionPoints {
                current: 500,
                speed: 100,
            },
        ))
        .id();
    let fiber = spawn(w, player, "fiber", 2);
    w.entity_mut(fiber)
        .remove::<InsideContainer>()
        .insert(position);
    spawn(w, player, "knife", 1);
    spawn(w, other, "knife", 1);
    let recipe = recipe(w);
    start_craft(w, player, recipe).unwrap();
    assert!(start_craft(w, other, recipe).is_err());
    assert_eq!(w.get::<ActionPoints>(other).unwrap().current, 500);
}
#[test]
fn mod_order_survives_alias_changes_and_inherited_templates_are_not_spawnable() {
    let docs = vec![
        SourceDocument {
            path: "core.json".into(),
            mod_id: "core".into(),
            values: vec![
                json!({"type":"GENERIC", "abstract":"base", "volume":100, "weight":10}),
                json!({"type":"GENERIC", "id":"fiber", "copy-from":"base", "name":"old"}),
            ],
        },
        SourceDocument {
            path: "mod.json".into(),
            mod_id: "mod".into(),
            values: vec![
                json!({"type":"ITEM", "id":"fiber", "copy-from":"base", "name":"new", "relative":{"weight":5}}),
            ],
        },
    ];
    let report = import_inventory(docs);
    let catalog = report.strict_catalog().unwrap();
    assert_eq!(catalog.items.len(), 1);
    assert_eq!(catalog.items[&ItemKey("fiber".into())].name, "new");
    assert_eq!(catalog.items[&ItemKey("fiber".into())].weight_g, 15);
    assert!(report
        .diagnostics
        .iter()
        .any(|d| d.definition == "ITEM:fiber" && d.mod_id == "mod"));
}

#[test]
fn disk_ingest_skips_manifest_without_skipping_later_content() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("a.json"),
        r#"[{"type":"GENERIC","id":"a"}]"#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("modinfo.json"),
        r#"[{"type":"MOD_INFO","id":"mod"}]"#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("z.json"),
        r#"[{"type":"GENERIC","id":"z"}]"#,
    )
    .unwrap();
    let mut loader = cdda_data::Loader::new(vec![dir.path().into()]);
    loader.ingest_all();
    let (items, errors) = loader.resolve_type_raw("ITEM");
    assert!(errors.is_empty());
    assert_eq!(
        items
            .iter()
            .map(|(key, _)| key.as_str())
            .collect::<Vec<_>>(),
        ["a", "z"]
    );
}

#[test]
fn unkeyed_upstream_families_are_preserved_not_mislabeled_as_malformed() {
    let report = import_inventory(vec![SourceDocument {
        path: "dreams.json".into(),
        mod_id: "core".into(),
        values: vec![json!({"type":"dream", "messages":["A dream"]})],
    }]);
    assert!(report.strict_catalog().is_err());
    assert!(report
        .diagnostics
        .iter()
        .all(|d| d.support == Support::PreservedUnimplemented));
    assert_eq!(report.documents[0].values[0]["messages"][0], "A dream");
}

#[test]
fn imported_pocket_capacity_drives_native_transfers_and_crafting_frees_space() {
    use cdda_components::intent::{ActionIntent, ActionOutcome, ActionOutcomeState};
    use cdda_core_types::core::coords::{WorldPos, ZLevel};
    use cdda_sim::inventory::capacity::contents_load;
    let (mut app, player) = world();
    let w = app.world_mut();
    let pos = WorldPosition::new(WorldPos::new(0, 0, ZLevel::new(0)));
    w.entity_mut(player).insert(pos);
    let bag = spawn(w, player, "bag", 1);
    let pocket = w.get::<MountedPockets>(bag).unwrap().iter().next().unwrap();
    let fiber = spawn(w, player, "fiber", 200);
    w.entity_mut(fiber).remove::<InsideContainer>().insert(pos);
    spawn(w, player, "knife", 1);
    w.entity_mut(player).insert(ActionIntent::Transfer {
        item: fiber,
        container: pocket,
    });
    assert!(step_simulation(w));
    assert_eq!(
        w.get::<ActionOutcome>(player).unwrap().state,
        ActionOutcomeState::Completed
    );
    assert_eq!(contents_load(w, pocket).unwrap().volume_ml, 2000);
    let extra = spawn(w, player, "fiber", 1);
    w.entity_mut(extra).remove::<InsideContainer>().insert(pos);
    let before = w.get::<ActionPoints>(player).unwrap().current;
    w.entity_mut(player).insert(ActionIntent::Transfer {
        item: extra,
        container: pocket,
    });
    assert!(step_simulation(w));
    assert_eq!(
        w.get::<ActionOutcome>(player).unwrap().state,
        ActionOutcomeState::Rejected
    );
    assert_eq!(
        w.get::<ActionPoints>(player).unwrap().current,
        before + 100,
        "rejection charges none of the turn's AP grant"
    );
    let recipe = recipe(w);
    w.entity_mut(bag).insert(Sealed);
    assert!(
        do_craft(w, player, recipe).is_err(),
        "sealed nested ingredients are inaccessible"
    );
    assert_eq!(w.get::<StackCount>(fiber).unwrap().get(), 200);
    w.entity_mut(bag).remove::<Sealed>();
    do_craft(w, player, recipe).unwrap();
    assert_eq!(w.get::<StackCount>(fiber).unwrap().get(), 198);
    assert_eq!(contents_load(w, pocket).unwrap().volume_ml, 1980);
    w.entity_mut(player).insert(ActionIntent::Transfer {
        item: extra,
        container: pocket,
    });
    assert!(step_simulation(w));
    assert_eq!(w.get::<InsideContainer>(extra).unwrap().0, pocket);
    assert_eq!(contents_load(w, pocket).unwrap().volume_ml, 1990);
}

#[test]
fn native_stacks_retain_definition_generation_during_explicit_merge() {
    use cdda_sim::inventory::systems::merge_or_stack;
    let (mut app, player) = world();
    let w = app.world_mut();
    let old = spawn(w, player, "fiber", 1);
    let same = spawn(w, player, "fiber", 1);
    assert!(merge_or_stack(w, old, same));
    let mut next = w.resource::<InventoryCatalog>().clone();
    std::sync::Arc::make_mut(next.items.get_mut(&ItemKey("fiber".into())).unwrap()).name =
        "new generation fiber".into();
    publish_inventory(w, next).unwrap();
    let new = spawn(w, player, "fiber", 1);
    assert!(!merge_or_stack(w, old, new));
    assert_eq!(w.get::<StackCount>(old).unwrap().get(), 2);
    assert_eq!(w.get::<ItemName>(new).unwrap().0, "new generation fiber");
}

#[test]
fn master_craft_ticks_consume_all_moves_including_the_finishing_tick() {
    use cdda_components::{activity::*, intent::*};
    // activity_actor.cpp craft_activity_actor::do_turn calls set_moves(0)
    // before clamping completion, even when the remaining recipe work is less.
    for speed in [50, 100, 250, 1000] {
        let (mut app, player) = world();
        let w = app.world_mut();
        *w.get_mut::<ActionPoints>(player).unwrap() = ActionPoints { current: 0, speed };
        spawn(w, player, "fiber", 2);
        spawn(w, player, "knife", 1);
        let recipe = recipe(w);
        w.entity_mut(player)
            .insert(ActionIntent::StartCraft { recipe });
        step_simulation(w);
        assert_eq!(
            w.get::<ActionOutcome>(player).unwrap().state,
            ActionOutcomeState::Completed
        );
        if speed < 200 {
            let craft = w.get::<Crafting>(player).unwrap().craft_entity;
            assert_eq!(w.get::<InProgressCraft>(craft).unwrap().ap_spent, speed);
            assert_eq!(w.get::<ActionPoints>(player).unwrap().current, 0);
            w.entity_mut(player).insert(ActionIntent::Wait);
            for _ in 1..(200 / speed) {
                step_simulation(w);
            }
            assert!(w.get::<ActivityProgress>(player).is_none());
            assert_eq!(w.get::<ActionPoints>(player).unwrap().current, 0);
            assert!(
                w.get::<ActionIntent>(player).is_some(),
                "no extra budget on completion"
            );
        } else {
            assert!(w.get::<ActivityProgress>(player).is_none());
            assert_eq!(w.get::<ActionPoints>(player).unwrap().current, 0);
        }
    }
}

#[test]
fn menu_craft_uses_player_remainder_and_refreshes_output_without_an_extra_turn() {
    use cdda_components::{activity::*, dev::DevPlayer, intent::*, sim::GameTime};
    use cdda_sim::crafting::systems::{CraftOutcome, CraftRevision, PendingCraft};
    let (mut app, player) = world();
    let w = app.world_mut();
    w.entity_mut(player).insert(DevPlayer);
    *w.get_mut::<ActionPoints>(player).unwrap() = ActionPoints {
        current: 0,
        speed: 250,
    };
    spawn(w, player, "fiber", 2);
    spawn(w, player, "knife", 1);
    w.entity_mut(player).insert(ActionIntent::Wait);
    app.update();
    assert_eq!(
        app.world().get::<ActionPoints>(player).unwrap().current,
        150
    );
    let recipe = recipe(app.world());
    app.world_mut().resource_mut::<PendingCraft>().0 = Some(recipe);
    app.update();
    let w = app.world();
    assert_eq!(w.resource::<GameTime>().turn, 1);
    let craft = w.get::<Crafting>(player).unwrap().craft_entity;
    assert_eq!(w.get::<InProgressCraft>(craft).unwrap().ap_spent, 150);
    assert_eq!(w.get::<ActionPoints>(player).unwrap().current, 0);
    app.update();
    let w = app.world();
    assert_eq!(w.resource::<GameTime>().turn, 2);
    assert_eq!(w.get::<ActionPoints>(player).unwrap().current, 0);
    assert!(w.get_entity(craft).is_err());
    let Some(CraftOutcome::Completed { item }) = w.resource::<CraftRevision>().last_result else {
        panic!("craft should complete on its second turn");
    };
    assert!(
        w.get::<Invlet>(item).is_some(),
        "result bookkeeping runs after completion"
    );
}

#[test]
fn pending_craft_waits_for_budget_without_consuming_or_overwriting_actions() {
    use cdda_components::{dev::DevPlayer, intent::*};
    use cdda_sim::crafting::systems::{CraftOutcome, CraftRevision, PendingCraft};
    let (mut app, player) = world();
    let w = app.world_mut();
    w.entity_mut(player).insert(DevPlayer);
    *w.get_mut::<ActionPoints>(player).unwrap() = ActionPoints {
        current: -50,
        speed: 25,
    };
    let fiber = spawn(w, player, "fiber", 2);
    spawn(w, player, "knife", 1);
    let recipe = recipe(w);
    w.resource_mut::<PendingCraft>().0 = Some(recipe);
    for _ in 0..2 {
        step_simulation(w);
        assert_eq!(w.get::<StackCount>(fiber).unwrap().get(), 2);
        assert!(w.get::<ActionOutcome>(player).is_none());
    }
    step_simulation(w);
    assert!(w.get_entity(fiber).is_err());
    assert_eq!(w.get::<ActionPoints>(player).unwrap().current, 0);
    cdda_sim::activity::lifecycle::interrupt_activity(w, player);
    w.entity_mut(player).insert(ActionIntent::Wait);
    w.resource_mut::<PendingCraft>().0 = Some(recipe);
    step_simulation(w);
    assert!(matches!(
        w.resource::<CraftRevision>().last_result,
        Some(CraftOutcome::Failed { .. })
    ));
    assert_eq!(
        w.get::<ActionOutcome>(player).unwrap().state,
        ActionOutcomeState::Completed
    );
    assert!(w.resource::<PendingCraft>().0.is_none());
}

#[test]
fn native_craft_requests_contend_in_stable_actor_order() {
    use cdda_components::intent::*;
    use cdda_core_types::{
        core::coords::{WorldPos, ZLevel},
        sim_id::SimId,
    };
    let (mut app, first) = world();
    let w = app.world_mut();
    let pos = WorldPosition::new(WorldPos::new(0, 0, ZLevel::new(0)));
    w.entity_mut(first).insert((
        pos,
        SimId(2),
        ActionPoints {
            current: 0,
            speed: 100,
        },
    ));
    let winner = w
        .spawn((
            pos,
            SimId(1),
            ActionPoints {
                current: 0,
                speed: 100,
            },
        ))
        .id();
    let fiber = spawn(w, first, "fiber", 2);
    w.entity_mut(fiber).remove::<InsideContainer>().insert(pos);
    for actor in [first, winner] {
        spawn(w, actor, "knife", 1);
        let recipe = recipe(w);
        w.entity_mut(actor)
            .insert(ActionIntent::StartCraft { recipe });
    }
    step_simulation(w);
    assert_eq!(
        w.get::<ActionOutcome>(winner).unwrap().state,
        ActionOutcomeState::Completed
    );
    assert_eq!(
        w.get::<ActionOutcome>(first).unwrap().state,
        ActionOutcomeState::Rejected
    );
    assert_eq!(w.get::<ActionPoints>(first).unwrap().current, 100);
    assert_eq!(w.get::<ActionPoints>(winner).unwrap().current, 0);
}

#[test]
fn interrupt_resume_and_completion_validate_saved_work_and_ownership() {
    use cdda_components::activity::*;
    use cdda_sim::{activity::lifecycle::interrupt_activity, crafting::systems::resume_craft};
    let (mut app, player) = world();
    let w = app.world_mut();
    *w.get_mut::<ActionPoints>(player).unwrap() = ActionPoints {
        current: 0,
        speed: 50,
    };
    spawn(w, player, "fiber", 2);
    spawn(w, player, "knife", 1);
    let recipe = recipe(w);
    let craft = start_craft(w, player, recipe).unwrap();
    assert!(
        complete_craft(w, player, craft).is_err(),
        "cannot finish unearned work"
    );
    step_simulation(w);
    assert!(interrupt_activity(w, player));
    assert!(w.get::<Crafting>(player).is_none());
    assert_eq!(w.get::<InProgressCraft>(craft).unwrap().ap_spent, 50);
    let other = w
        .spawn(ActionPoints {
            current: 100,
            speed: 100,
        })
        .id();
    assert!(resume_craft(w, other, craft).is_err());
    assert!(complete_craft(w, other, craft).is_err());
    resume_craft(w, player, craft).unwrap();
    assert_eq!(w.get::<ActivityProgress>(player).unwrap().moves_left, 150);
    for _ in 0..3 {
        step_simulation(w);
    }
    assert!(w.get_entity(craft).is_err());
    assert_eq!(w.get::<ActionPoints>(player).unwrap().current, 0);
}

#[test]
fn inaccessible_craft_interrupts_without_spending_and_completion_is_not_replayed() {
    use cdda_components::{activity::Crafting, messages::CraftCompleted};
    use cdda_sim::crafting::systems::{resume_craft, CraftRevision};
    let (mut app, player) = world();
    let w = app.world_mut();
    *w.get_mut::<ActionPoints>(player).unwrap() = ActionPoints {
        current: 0,
        speed: 50,
    };
    spawn(w, player, "fiber", 2);
    spawn(w, player, "knife", 1);
    let bag = spawn(w, player, "bag", 1);
    let pocket = w.get::<MountedPockets>(bag).unwrap().iter().next().unwrap();
    let recipe = recipe(w);
    let craft = start_craft(w, player, recipe).unwrap();
    w.entity_mut(craft).insert(InsideContainer(pocket));
    step_simulation(w);
    assert_eq!(w.get::<InProgressCraft>(craft).unwrap().ap_spent, 50);
    w.entity_mut(pocket).insert(Sealed);
    step_simulation(w);
    assert!(w.get::<Crafting>(player).is_none());
    assert_eq!(w.get::<ActionPoints>(player).unwrap().current, 50);
    assert_eq!(w.get::<InProgressCraft>(craft).unwrap().ap_spent, 50);
    assert!(resume_craft(w, player, craft).is_err());
    w.entity_mut(pocket).remove::<Sealed>();
    resume_craft(w, player, craft).unwrap();
    for _ in 0..2 {
        step_simulation(w);
    }
    assert!(w.get_entity(craft).is_err());
    let revision = w.resource::<CraftRevision>().revision;
    let mut cursor = bevy_ecs::message::MessageCursor::<CraftCompleted>::default();
    assert_eq!(
        cursor
            .read(w.resource::<Messages<CraftCompleted>>())
            .count(),
        1,
        "completion adapter leaves notifications for independent readers"
    );
    step_simulation(w);
    assert_eq!(w.resource::<CraftRevision>().revision, revision);
}

#[test]
fn native_interrupt_and_resume_commands_precede_activity_work_and_respect_pause() {
    use cdda_components::{activity::*, intent::*};
    use cdda_sim::{
        crafting::systems::{CraftOutcome, CraftRevision},
        runtime::SimulationControl,
    };
    let (mut app, player) = world();
    let w = app.world_mut();
    *w.get_mut::<ActionPoints>(player).unwrap() = ActionPoints {
        current: 0,
        speed: 50,
    };
    spawn(w, player, "fiber", 2);
    spawn(w, player, "knife", 1);
    let recipe = recipe(w);
    let craft = start_craft(w, player, recipe).unwrap();
    step_simulation(w);
    w.entity_mut(player).insert(ActionIntent::InterruptActivity);
    w.resource_mut::<SimulationControl>().paused = true;
    assert!(!step_simulation(w));
    assert!(w.get::<ActionIntent>(player).is_some());
    w.resource_mut::<SimulationControl>().paused = false;
    step_simulation(w);
    assert!(w.get::<Crafting>(player).is_none());
    assert_eq!(w.get::<InProgressCraft>(craft).unwrap().ap_spent, 50);
    assert_eq!(w.get::<ActionPoints>(player).unwrap().current, 50);
    let outcome = *w.get::<ActionOutcome>(player).unwrap();
    assert!(outcome.matches(*w.get::<ActionRequestId>(player).unwrap()));
    assert_eq!(outcome.state, ActionOutcomeState::Completed);
    assert!(
        matches!(w.resource::<CraftRevision>().last_result, Some(CraftOutcome::Interrupted { craft: c }) if c == craft)
    );
    w.entity_mut(player)
        .insert(ActionIntent::ResumeCraft { craft });
    step_simulation(w);
    assert_eq!(w.get::<InProgressCraft>(craft).unwrap().ap_spent, 150);
    assert_eq!(w.get::<ActionPoints>(player).unwrap().current, 0);
    assert!(w.get::<ActionOutcome>(player).unwrap().request.0 > outcome.request.0);
    step_simulation(w);
    assert!(w.get_entity(craft).is_err());
}

#[test]
fn rejected_resume_preserves_the_existing_activity_and_charges_nothing() {
    use cdda_components::{activity::*, intent::*};
    let (mut app, player) = world();
    let w = app.world_mut();
    *w.get_mut::<ActionPoints>(player).unwrap() = ActionPoints {
        current: 0,
        speed: 50,
    };
    w.entity_mut(player).insert((
        Waiting { turns: 2 },
        ActivityProgress::new(200),
        ActionIntent::ResumeCraft {
            craft: Entity::PLACEHOLDER,
        },
    ));
    step_simulation(w);
    assert_eq!(
        w.get::<ActionOutcome>(player).unwrap().state,
        ActionOutcomeState::Rejected
    );
    assert_eq!(w.get::<ActionPoints>(player).unwrap().current, 50);
    assert_eq!(w.get::<ActivityProgress>(player).unwrap().moves_left, 200);
    assert!(w.get::<Waiting>(player).is_some());
    w.entity_mut(player).insert(ActionIntent::InterruptActivity);
    step_simulation(w);
    assert!(w.get::<Waiting>(player).is_none());
    w.entity_mut(player).insert(ActionIntent::InterruptActivity);
    step_simulation(w);
    assert_eq!(
        w.get::<ActionOutcome>(player).unwrap().state,
        ActionOutcomeState::Rejected
    );
    assert_eq!(w.get::<ActionPoints>(player).unwrap().current, 150);
}
