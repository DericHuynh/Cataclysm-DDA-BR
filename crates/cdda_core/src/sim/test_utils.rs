//! # Test utilities for ECS feature tests
//!
//! Provides `TestBed` — a lightweight, `bevy_ecs`-compatible wrapper around `World`
//! for testing systems and entities in isolation.  No full `bevy` dependency needed.

use crate::core::components::actor::*;
use crate::core::components::def::*;
use crate::core::components::item::*;
use crate::core::components::sim::{InFlight, Solid, Velocity, WorldPosition};
use bevy_ecs::prelude::*;
use bevy_ecs::system::IntoSystem;
use bevy_ecs::world::World;

/// A lightweight test environment wrapping a `World`.
pub struct TestBed {
    world: World,
}

impl TestBed {
    pub fn new() -> Self {
        Self {
            world: World::new(),
        }
    }
    pub fn world(&self) -> &World {
        &self.world
    }
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }
    pub fn spawn(&mut self, bundle: impl Bundle) -> Entity {
        self.world.spawn(bundle).id()
    }
    pub fn get<T: Component>(&self, e: Entity) -> Option<&T> {
        self.world.get::<T>(e)
    }
    pub fn resource<T: Resource>(&self) -> &T {
        self.world.resource::<T>()
    }
    pub fn resource_mut<T: Resource>(&mut self) -> Mut<'_, T> {
        self.world.resource_mut::<T>()
    }
    pub fn insert_resource<T: Resource>(&mut self, r: T) {
        self.world.insert_resource(r);
    }
    pub fn register<T: Component>(&mut self) -> &mut Self {
        self.world.register_component::<T>();
        self
    }

    pub fn run_system<M: 'static>(&mut self, system: impl IntoSystem<(), (), M>) {
        let mut sys = IntoSystem::into_system(system);
        sys.initialize(&mut self.world);
        let _ = sys.run((), &mut self.world);
        sys.apply_deferred(&mut self.world);
    }

    /// Load def data from a DefRegistry. Registers all def components, builds def world.
    pub fn load_data(
        &mut self,
        registry: &crate::data::DefRegistry,
    ) -> crate::data::def_world::DefinitionWorld {
        Self::register_all_def_components(&mut self.world);
        crate::data::def_world::build_def_world(&mut self.world, registry, true)
    }

    // ── Batch registration ────────────────────────────────────────

    pub fn register_all_def_components(world: &mut World) {
        world.register_component::<IsDef>();
        world.register_component::<DefStrId>();
        world.register_component::<ItemName>();
        world.register_component::<ItemDescription>();
        world.register_component::<ItemWeight>();
        world.register_component::<ItemVolume>();
        world.register_component::<ItemSymbol>();
        world.register_component::<ItemColor>();
        world.register_component::<ItemMaterials>();
        world.register_component::<ItemPhase>();
        world.register_component::<ItemCountMode>();
        world.register_component::<ItemPrice>();
        world.register_component::<ItemCategory>();
        world.register_component::<ItemStackSize>();
        world.register_component::<WeaponData>();
        world.register_component::<GunData>();
        world.register_component::<AmmoData>();
        world.register_component::<MagazineData>();
        world.register_component::<ArmourData>();
        world.register_component::<FoodData>();
        world.register_component::<ToolData>();
        world.register_component::<BookData>();
        world.register_component::<GunModData>();
        world.register_component::<ContainerData>();
        world.register_component::<DrugData>();
        world.register_component::<MonsterName>();
        world.register_component::<MonsterDescription>();
        world.register_component::<MonsterStats>();
        world.register_component::<MonsterMelee>();
        world.register_component::<MonsterVision>();
        world.register_component::<MonsterArmour>();
        world.register_component::<MonsterSpecies>();
        world.register_component::<MonsterDefaultFaction>();
        world.register_component::<MonsterBodyType>();
        world.register_component::<TerrainName>();
        world.register_component::<TerrainSymbol>();
        world.register_component::<TerrainColor>();
        world.register_component::<TerrainMoveCost>();
        world.register_component::<TerrainLightEmitted>();
        world.register_component::<TerrainHasCeiling>();
        world.register_component::<TerrainConnectsTo>();
        world.register_component::<FurnitureName>();
        world.register_component::<FurnitureSymbol>();
        world.register_component::<FurnitureColor>();
        world.register_component::<FurnitureMoveCostMod>();
        world.register_component::<FurnitureCoverage>();
        world.register_component::<FurnitureLightEmitted>();
        world.register_component::<FurnitureMaxVolume>();
        world.register_component::<BodyPartDefId>();
        world.register_component::<BodyPartName>();
        world.register_component::<BodyPartHitSize>();
        world.register_component::<BodyPartHitDifficulty>();
        world.register_component::<BodyPartBaseHp>();
        world.register_component::<BodyPartDrenchCapacity>();
        world.register_component::<BodyPartSide>();
        world.register_component::<BodyPartLegacyId>();
        world.register_component::<IsVital>();
        world.register_component::<CanGrasp>();
        world.register_component::<CanWalk>();
        world.register_component::<CanSee>();
        world.register_component::<CanBite>();
        world.register_component::<CanFly>();
        world.register_component::<SubParts>();
        world.register_component::<ParentPart>();
    }

    pub fn register_gameplay_components(world: &mut World) {
        world.register_component::<WorldPosition>();
        world.register_component::<Solid>();
        world.register_component::<Velocity>();
        world.register_component::<StackCount>();
        world.register_component::<CurrentCharges>();
        world.register_component::<LoadedAmmo>();
        world.register_component::<Spoilable>();
        world.register_component::<ItemDamage>();
        world.register_component::<Container>();
        world.register_component::<Sealed>();
        world.register_component::<Rigid>();
        world.register_component::<Watertight>();
        world.register_component::<PreservesTemp>();
        world.register_component::<Fireproof>();
        world.register_component::<GasTight>();
        world.register_component::<Creature>();
        world.register_component::<CombatStats>();
        world.register_component::<Vision>();
        world.register_component::<Health>();
        world.register_component::<Faction>();
        world.register_component::<BodyTemperature>();
        world.register_component::<Wetness>();
        world.register_component::<SkillOf>();
        world.register_component::<CreatureSkills>();
        world.register_component::<SkillEntry>();
        world.register_component::<MutationOf>();
        world.register_component::<CreatureMutations>();
        world.register_component::<MutationEntry>();
        world.register_component::<ProficiencyOf>();
        world.register_component::<CreatureProficiencies>();
        world.register_component::<ProficiencyEntry>();
        world.register_component::<BionicOf>();
        world.register_component::<InstalledBionics>();
        world.register_component::<Bionic>();
        world.register_component::<MoraleBonusOf>();
        world.register_component::<MoraleBonuses>();
        world.register_component::<MoraleBonus>();
        world.register_component::<Morale>();
        world.register_component::<EffectOn>();
        world.register_component::<ActiveEffects>();
        world.register_component::<StatusEffect>();
        world.register_component::<PlayerData>();
        world.register_component::<NpcData>();
        world.register_component::<InsideContainer>();
        world.register_component::<ContainerContents>();
        world.register_component::<WieldedBy>();
        world.register_component::<WieldedItems>();
        world.register_component::<WornOn>();
        world.register_component::<WornBy>();
        world.register_component::<MountedOn>();
        world.register_component::<MountedPockets>();
        world.register_component::<Pocket>();
        world.register_component::<PocketRestriction>();
        world.register_component::<AttachmentSlot>();
        world.register_component::<BodyPartOf>();
        world.register_component::<CreatureBodyParts>();
        world.register_component::<BodyPartDef>();
        world.register_component::<BodyPartSlot>();
        world.register_component::<BodyPartHp>();
        world.register_component::<BodyPartBroken>();
        world.register_component::<BodyPartSevered>();
        world.register_component::<IsAlive>();
        world.register_component::<Stunned>();
        world.register_component::<Bleeding>();
        world.register_component::<OnFire>();
        world.register_component::<InFlight>();
        world.register_component::<ActionPoints>();
        world.register_component::<Speed>();
    }
}

impl Default for TestBed {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bed_spawns_entity() {
        let mut tb = TestBed::new();
        let e = tb.spawn((ItemName("test".to_string()),));
        assert!(tb.get::<ItemName>(e).is_some());
    }

    #[test]
    fn test_bed_runs_system() {
        fn add_ten(mut q: Query<&mut ItemWeight>) {
            for mut w in &mut q {
                w.0 += 10;
            }
        }
        let mut tb = TestBed::new();
        tb.register::<ItemWeight>();
        let e = tb.spawn((ItemWeight(5),));
        tb.run_system(add_ten);
        assert_eq!(tb.get::<ItemWeight>(e).unwrap().0, 15);
    }

    #[test]
    fn test_world_can_query() {
        let mut tb = TestBed::new();
        tb.register::<ItemName>();
        tb.spawn((ItemName("a".into()),));
        tb.spawn((ItemName("b".into()),));
        // Query via the World — get query state first (mut borrow), then iterate (immutable borrow)
        let mut q = {
            let w = tb.world_mut();
            w.query::<&ItemName>()
        };
        let names: Vec<&str> = q.iter(&*tb.world()).map(|n| n.0.as_str()).collect();
        assert_eq!(names, vec!["a", "b"]);
    }
}
