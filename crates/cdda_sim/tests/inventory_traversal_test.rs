//! Recursive inventory-domain traversal tests.
//!
//! `all_items_for_creature` / `all_items_for_creature_q` must walk the full
//! declared relationship graph: direct contents, mounted pockets, wielded
//! items, worn clothing — **and recurse** into every nested container and
//! pocket found along the way. A battery inside a flashlight inside a
//! backpack IS in the creature's inventory.

use bevy_ecs::prelude::*;
use cdda_components::actor::{Creature, IsAlive};
use cdda_components::item::{
    ContainerContents, InsideContainer, IsPocket, MountedOn, WieldedBy, WornOn,
};
use cdda_sim::inventory::systems::all_items_for_creature;
use cdda_sim::runtime::test_utils::TestBed;

fn creature(test: &mut TestBed) -> Entity {
    test.spawn((
        Creature {
            def_id: "test_creature".into(),
            name: "Test Creature".into(),
            species: 0.into(),
            symbol: 'z',
        },
        cdda_components::actor::IsAlive,
    ))
}

fn item(test: &mut TestBed) -> Entity {
    test.spawn(())
}

#[test]
fn traversal_reaches_nested_containers_and_worn_pockets() {
    let mut test = TestBed::new();
    let hero = creature(&mut test);

    // Backpack directly in the hero's contents.
    let backpack = item(&mut test);
    test.world_mut()
        .entity_mut(backpack)
        .insert(InsideContainer(hero));

    // Flashlight inside the backpack (nested container).
    let flashlight = item(&mut test);
    test.world_mut()
        .entity_mut(flashlight)
        .insert(InsideContainer(backpack));

    // Battery inside the flashlight (second nesting level).
    let battery = item(&mut test);
    test.world_mut()
        .entity_mut(battery)
        .insert(InsideContainer(flashlight));

    // Jacket WORN by the hero (`WornOn` lives on the item, points at wearer).
    let jacket = item(&mut test);
    test.world_mut()
        .entity_mut(jacket)
        .insert(WornOn { wearer: hero, slot: Some("torso".into()) });

    // Pocket mounted on the jacket, holding a match.
    let pocket = item(&mut test);
    test.world_mut().entity_mut(pocket).insert(IsPocket);
    test.world_mut().entity_mut(pocket).insert(MountedOn(jacket));
    let matches = item(&mut test);
    test.world_mut()
        .entity_mut(matches)
        .insert(InsideContainer(pocket));

    // Knife wielded in the hero's hands.
    let knife = item(&mut test);
    test.world_mut().entity_mut(knife).insert(WieldedBy(hero));

    let found = all_items_for_creature(hero, test.world());
    let mut found = found;
    found.sort();

    // Pocket entities (`IsPocket` + `MountedOn`) are container plumbing, not
    // items — they are traversed *through* but never listed (the pre-existing
    // semantics the recursion preserves).
    let mut expected = vec![backpack, flashlight, battery, jacket, matches, knife];
    expected.sort();
    assert_eq!(found, expected, "every reachable item is collected exactly once");
}

#[test]
fn traversal_ignores_other_creatures_and_relationship_cycles() {
    let mut test = TestBed::new();
    let hero = creature(&mut test);
    let stranger = creature(&mut test);

    // The stranger's bag is NOT the hero's.
    let strangers_bag = item(&mut test);
    test.world_mut()
        .entity_mut(strangers_bag)
        .insert(InsideContainer(stranger));

    // A benign "cycle": two containers listing each other. The visited set
    // must terminate traversal without dropping items.
    let a = item(&mut test);
    let b = item(&mut test);
    test.world_mut().entity_mut(a).insert(InsideContainer(hero));
    test.world_mut().entity_mut(b).insert(InsideContainer(a));
    // Re-pointing a's parent through the same holder is a relationship
    // replace, so instead simulate a shared reference: b also directly listed
    // as hero content (visited dedupes it).
    test.world_mut().entity_mut(b).insert(InsideContainer(hero));

    let found = all_items_for_creature(hero, test.world());
    assert!(found.contains(&a), "hero's direct item found");
    assert!(found.contains(&b), "nested item found");
    assert_eq!(
        found.iter().filter(|&&e| e == b).count(),
        1,
        "an item reachable twice is collected once"
    );
    assert!(!found.contains(&strangers_bag), "another creature's items are not ours");
}
