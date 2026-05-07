//! World initialisation — register components and resources.

use bevy_ecs::prelude::Resource;
use bevy_ecs::world::World;

use crate::components::*;
use crate::dev_worldgen::DevWorldgenConfig;
use crate::spatial::EntitySpatialIndex;
use crate::state::*;
use crate::systems::turn::TurnQueue;


/// Wrapper to store `cdda_map::WorldMap` as a Bevy resource.
/// `WorldMap` lives in the zero-bevy `cdda_map` crate, so it cannot
/// derive `Resource` directly.
#[derive(Resource, Debug, Clone)]
pub struct WorldMapResource(pub cdda_map::WorldMap);

impl Default for WorldMapResource {
    fn default() -> Self {
        Self(cdda_map::WorldMap::new())
    }
}

/// Register all ECS component types and initial resources.
///
/// Note: Events in Bevy 0.18 are trigger-based (via `World::trigger` /
/// `Commands::trigger`) and observed via `Observer` systems.
/// There is no `Events<T>` resource to pre-register — events are
/// available as soon as the type is registered as a component
/// (which happens automatically when they are first triggered).
pub fn setup_world(world: &mut World) {
    // --- Register resources ---
    world.insert_resource(EntitySpatialIndex::new());
    world.insert_resource(GameTime::default());
    world.insert_resource(LoadingStatus::default());
    world.insert_resource(StartupConfig::default());
    world.insert_resource(TurnQueue::default());
    world.insert_resource(WorldMapResource::default());
    world.insert_resource(DevWorldgenConfig::default());
    world.insert_resource(crate::systems::dev_move::DevCamera::default());

    // --- Spatial ---
    world.register_component::<WorldPosition>();
    world.register_component::<Solid>();
    world.register_component::<Velocity>();

    // --- Item — mutable runtime state ---
    world.register_component::<StackCount>();
    world.register_component::<CurrentCharges>();
    world.register_component::<LoadedAmmo>();
    world.register_component::<Spoilable>();
    world.register_component::<ItemDamage>();

    // --- Container tags ---
    world.register_component::<Container>();
    world.register_component::<Sealed>();
    world.register_component::<Rigid>();
    world.register_component::<Watertight>();
    world.register_component::<PreservesTemp>();
    world.register_component::<Fireproof>();
    world.register_component::<GasTight>();

    // --- Creature core ---
    world.register_component::<Creature>();
    world.register_component::<CombatStats>();
    world.register_component::<Vision>();
    world.register_component::<Health>();
    world.register_component::<Faction>();
    world.register_component::<Stats>();
    world.register_component::<BodyTemperature>();
    world.register_component::<Wetness>();

    // --- Creature progression ---
    world.register_component::<SkillSet>();
    world.register_component::<Mutations>();
    world.register_component::<ProficiencySet>();

    // --- Bionics ---
    world.register_component::<BionicOf>();
    world.register_component::<InstalledBionics>();
    world.register_component::<Bionic>();

    // --- Morale ---
    world.register_component::<MoraleBonusOf>();
    world.register_component::<MoraleBonuses>();
    world.register_component::<MoraleBonus>();
    world.register_component::<Morale>();

    // --- Status effects ---
    world.register_component::<EffectOn>();
    world.register_component::<ActiveEffects>();
    world.register_component::<StatusEffect>();

    // --- Player / NPC ---
    world.register_component::<PlayerData>();
    world.register_component::<NpcData>();

    // --- Relationships ---
    world.register_component::<InsideContainer>();
    world.register_component::<ContainerContents>();
    world.register_component::<WieldedBy>();
    world.register_component::<WieldedItems>();
    world.register_component::<WornOn>();
    world.register_component::<WornBy>();
    world.register_component::<MountedOn>();
    world.register_component::<MountedPockets>();

    // --- Pocket system ---
    world.register_component::<Pocket>();
    world.register_component::<PocketRestriction>();
    world.register_component::<AttachmentSlot>();

    // --- Turn scheduling ---
    world.register_component::<MovePoints>();
    world.register_component::<Speed>();

    // --- Status markers ---
    world.register_component::<IsAlive>();
    world.register_component::<Stunned>();
    world.register_component::<Bleeding>();
    world.register_component::<OnFire>();

    // --- Body part def components ---
    world.register_component::<crate::def_components::BodyPartDefId>();
    world.register_component::<crate::def_components::BodyPartName>();
    world.register_component::<crate::def_components::BodyPartHitSize>();
    world.register_component::<crate::def_components::BodyPartHitDifficulty>();
    world.register_component::<crate::def_components::BodyPartBaseHp>();
    world.register_component::<crate::def_components::BodyPartDrenchCapacity>();
    world.register_component::<crate::def_components::BodyPartSide>();
    world.register_component::<crate::def_components::BodyPartLegacyId>();
    world.register_component::<crate::def_components::IsVital>();
    world.register_component::<crate::def_components::CanGrasp>();
    world.register_component::<crate::def_components::CanWalk>();
    world.register_component::<crate::def_components::CanSee>();
    world.register_component::<crate::def_components::CanBite>();
    world.register_component::<crate::def_components::CanFly>();
    world.register_component::<crate::def_components::SubParts>();
    world.register_component::<crate::def_components::ParentPart>();

    // --- Body part instance components ---
    world.register_component::<BodyPartOf>();
    world.register_component::<CreatureBodyParts>();
    world.register_component::<BodyPartDef>();
    world.register_component::<BodyPartSlot>();
    world.register_component::<BodyPartHp>();
    world.register_component::<BodyPartBroken>();
    world.register_component::<BodyPartSevered>();
    world.register_component::<InFlight>();
}
