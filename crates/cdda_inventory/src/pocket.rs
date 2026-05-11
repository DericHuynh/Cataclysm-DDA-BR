//! Body-pocket helpers for the player character.
//!
//! In CDDA every item must live inside a pocket.  The player has implicit
//! "body" pockets (clothing pockets, backpack, etc.).  We model this with a
//! single omnibus body-pocket entity that owns everything not held in hand or
//! worn as clothing.

use bevy_ecs::prelude::*;

use cdda_components::item::{IsPocket, MountedOn, MountedPockets, Pocket, PocketOf, PocketType};
use cdda_core_types::core::units::{Length, Volume, Weight};

/// Spawn a body-pocket entity owned by `player` and return its `Entity`.
///
/// The pocket is effectively unlimited (max values / 2 to avoid overflow in
/// arithmetic elsewhere).  Volume / weight enforcement is deferred to a later
/// milestone.
pub fn spawn_body_pocket(world: &mut World, player: Entity) -> Entity {
    world
        .spawn((
            IsPocket,
            PocketOf(player),
            MountedOn(player),
            Pocket {
                max_volume: Volume(u64::MAX / 2),
                max_weight: Weight(u64::MAX / 2),
                max_item_length: Length(u32::MAX / 2),
                min_item_volume: Volume(0),
                pocket_type: PocketType::Container,
            },
        ))
        .id()
}

/// Return the first body-pocket of `player`, or `None` if not found.
pub fn get_body_pocket(player: Entity, mounted_pockets: &Query<&MountedPockets>) -> Option<Entity> {
    mounted_pockets
        .get(player)
        .ok()
        .and_then(|mp| mp.iter().next())
}
