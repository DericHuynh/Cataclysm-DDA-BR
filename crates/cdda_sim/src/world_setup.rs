//! World initialisation — register components and resources.

use bevy_ecs::world::World;

use crate::components::*;

/// Register all ECS component types and initial resources.
///
/// Event types are registered by `App::add_event` in cdda_app.
pub fn setup_world(world: &mut World) {
    // --- Register all component types ---
    world.register_component::<WorldPosition>();
    world.register_component::<Item>();
    world.register_component::<StackCount>();
    world.register_component::<Weapon>();
    world.register_component::<Armor>();
    world.register_component::<Container>();
    world.register_component::<Food>();
    world.register_component::<Tool>();
    world.register_component::<Creature>();
    world.register_component::<CombatStats>();
    world.register_component::<Vision>();
    world.register_component::<Health>();
    world.register_component::<Faction>();

    // Tag components (zero-sized markers)
    world.register_component::<Sealed>();
    world.register_component::<Rigid>();
    world.register_component::<Watertight>();
    world.register_component::<PreservesTemp>();
    world.register_component::<UsesCharges>();

    // Relationship components
    world.register_component::<InsideContainer>();
    world.register_component::<WieldedBy>();
    world.register_component::<WornBy>();
}
