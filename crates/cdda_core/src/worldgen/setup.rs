//! World initialisation — register components and resources.

use bevy_ecs::prelude::Resource;
use bevy_ecs::world::World;

use crate::actor::turn::TurnQueue;
use crate::core::components::actor::*;
use crate::core::components::item::*;
use crate::core::components::item::{
    Inventory, InventoryBin, InventoryFocus, Invlet, InvletFavorites,
};
use crate::core::components::sim::{InFlight, Solid, Velocity, WorldPosition};
use crate::map::spatial::EntitySpatialIndex;
use crate::sim::state::*;
use crate::worldgen::dev::DevWorldgenConfig;
use crate::worldgen::dev_spawn::{DevSpawnFocus, DevSpawnQueue};
use cdda_components::dev::{DevCamera, DevGroundItemName, DevPlayer};

/// Wrapper to store `crate::map::WorldMap` as a Bevy resource.
/// `WorldMap` lives in the zero-bevy `cdda_map` crate, so it cannot
/// derive `Resource` directly.
#[derive(Resource, Debug, Clone)]
pub struct WorldMapResource(pub crate::map::WorldMap);

impl Default for WorldMapResource {
    fn default() -> Self {
        Self(crate::map::WorldMap::new())
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
    world.insert_resource(InventoryFocus::default());
    world.insert_resource(DevSpawnFocus::default());
    world.insert_resource(DevSpawnQueue::default());
    world.insert_resource(GameTime::default());
    world.insert_resource(LoadingStatus::default());
    world.insert_resource(StartupConfig::default());
    world.insert_resource(TurnQueue::default());
    world.insert_resource(WorldMapResource::default());
    world.insert_resource(DevWorldgenConfig::default());
    world.insert_resource(DevCamera::default());
    world.insert_resource(InventoryBin::default());
    world.insert_resource(crate::inventory::examine_resource::ExaminedItem::default());

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
    world.register_component::<ItemQualities>();

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

    // --- Skills (relationship-based) ---
    world.register_component::<SkillOf>();
    world.register_component::<CreatureSkills>();
    world.register_component::<SkillEntry>();

    // --- Mutations (relationship-based) ---
    world.register_component::<MutationOf>();
    world.register_component::<CreatureMutations>();
    world.register_component::<MutationEntry>();

    // --- Proficiencies (relationship-based) ---
    world.register_component::<ProficiencyOf>();
    world.register_component::<CreatureProficiencies>();
    world.register_component::<ProficiencyEntry>();

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

    // --- Inventory ---
    world.register_component::<Inventory>();
    world.register_component::<InvletFavorites>();
    world.register_component::<Invlet>();
    world.register_component::<DevPlayer>();
    world.register_component::<DevGroundItemName>();
    world.register_component::<crate::core::components::item::InProgressCraft>();

    // --- Turn scheduling ---
    world.register_component::<ActionPoints>();
    world.register_component::<HandCount>();

    // --- Status markers ---
    world.register_component::<IsAlive>();
    world.register_component::<Stunned>();
    world.register_component::<Bleeding>();
    world.register_component::<OnFire>();

    // --- Body part def components ---
    world.register_component::<crate::core::components::def::BodyPartDefId>();
    world.register_component::<crate::core::components::def::ItemName>();
    world.register_component::<crate::core::components::def::BodyPartHitSize>();
    world.register_component::<crate::core::components::def::BodyPartHitDifficulty>();
    world.register_component::<crate::core::components::def::BodyPartBaseHp>();
    world.register_component::<crate::core::components::def::BodyPartDrenchCapacity>();
    world.register_component::<crate::core::components::def::BodyPartSide>();
    world.register_component::<crate::core::components::def::BodyPartLegacyId>();
    world.register_component::<crate::core::components::def::IsVital>();
    world.register_component::<crate::core::components::def::CanGrasp>();
    world.register_component::<crate::core::components::def::CanWalk>();
    world.register_component::<crate::core::components::def::CanSee>();
    world.register_component::<crate::core::components::def::CanBite>();
    world.register_component::<crate::core::components::def::CanFly>();
    world.register_component::<crate::core::components::def::SubParts>();
    world.register_component::<crate::core::components::def::ParentPart>();

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
