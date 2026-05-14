//! World initialisation — register ECS components and resources.
//!
//! This replaces the old `cdda_worldgen::setup::setup_world` function.

use bevy_ecs::world::World;

/// Register all game component types needed before spawning any entity.
///
/// Must be called once during app initialisation, before `AppState::DataLoading`.
/// In Bevy 0.18, components auto-register on first use, but for custom
/// component types without automatic registration, we call this explicitly.
pub fn register_game_components(world: &mut World) {
    // ── Spatial ───────────────────────────────────────────────────────
    world.register_component::<cdda_components::sim::WorldPosition>();
    world.register_component::<cdda_components::sim::Solid>();
    world.register_component::<cdda_components::sim::Velocity>();

    // ── Item — mutable runtime state ──────────────────────────────────
    world.register_component::<cdda_components::item::StackCount>();
    world.register_component::<cdda_components::item::CurrentCharges>();
    world.register_component::<cdda_components::item::LoadedAmmo>();
    world.register_component::<cdda_components::item::Spoilable>();
    world.register_component::<cdda_components::item::ItemDamage>();
    world.register_component::<cdda_components::item::ItemQualities>();

    // ── Container tags ────────────────────────────────────────────────
    world.register_component::<cdda_components::item::Container>();
    world.register_component::<cdda_components::item::Sealed>();
    world.register_component::<cdda_components::item::Rigid>();
    world.register_component::<cdda_components::item::Watertight>();
    world.register_component::<cdda_components::item::PreservesTemp>();
    world.register_component::<cdda_components::item::Fireproof>();
    world.register_component::<cdda_components::item::GasTight>();

    // ── Creature core ─────────────────────────────────────────────────
    world.register_component::<cdda_components::actor::Creature>();
    world.register_component::<cdda_components::actor::CombatStats>();
    world.register_component::<cdda_components::actor::Vision>();
    world.register_component::<cdda_components::actor::Health>();
    world.register_component::<cdda_components::actor::Faction>();
    world.register_component::<cdda_components::actor::Stats>();
    world.register_component::<cdda_components::actor::BodyTemperature>();
    world.register_component::<cdda_components::actor::Wetness>();

    // ── Skills (relationship-based) ──────────────────────────────────
    world.register_component::<cdda_components::actor::SkillOf>();
    world.register_component::<cdda_components::actor::CreatureSkills>();
    world.register_component::<cdda_components::actor::SkillEntry>();

    // ── Mutations (relationship-based) ────────────────────────────────
    world.register_component::<cdda_components::actor::MutationOf>();
    world.register_component::<cdda_components::actor::CreatureMutations>();
    world.register_component::<cdda_components::actor::MutationEntry>();

    // ── Proficiencies (relationship-based) ────────────────────────────
    world.register_component::<cdda_components::actor::ProficiencyOf>();
    world.register_component::<cdda_components::actor::CreatureProficiencies>();
    world.register_component::<cdda_components::actor::ProficiencyEntry>();

    // ── Bionics ───────────────────────────────────────────────────────
    world.register_component::<cdda_components::actor::BionicOf>();
    world.register_component::<cdda_components::actor::InstalledBionics>();
    world.register_component::<cdda_components::actor::Bionic>();

    // ── Morale ────────────────────────────────────────────────────────
    world.register_component::<cdda_components::actor::MoraleBonusOf>();
    world.register_component::<cdda_components::actor::MoraleBonuses>();
    world.register_component::<cdda_components::actor::MoraleBonus>();
    world.register_component::<cdda_components::actor::Morale>();

    // ── Status effects ────────────────────────────────────────────────
    world.register_component::<cdda_components::actor::EffectOn>();
    world.register_component::<cdda_components::actor::ActiveEffects>();
    world.register_component::<cdda_components::actor::StatusEffect>();

    // ── Player / NPC ──────────────────────────────────────────────────
    world.register_component::<cdda_components::actor::PlayerData>();
    world.register_component::<cdda_components::actor::NpcData>();

    // ── Relationships ─────────────────────────────────────────────────
    world.register_component::<cdda_components::item::InsideContainer>();
    world.register_component::<cdda_components::item::ContainerContents>();
    world.register_component::<cdda_components::item::WieldedBy>();
    world.register_component::<cdda_components::item::WieldedItems>();
    world.register_component::<cdda_components::item::WornOn>();
    world.register_component::<cdda_components::item::WornBy>();
    world.register_component::<cdda_components::item::MountedOn>();
    world.register_component::<cdda_components::item::MountedPockets>();

    // ── Pocket system ─────────────────────────────────────────────────
    world.register_component::<cdda_components::item::Pocket>();
    world.register_component::<cdda_components::item::PocketRestriction>();
    world.register_component::<cdda_components::item::AttachmentSlot>();

    // ── Inventory ─────────────────────────────────────────────────────
    world.register_component::<cdda_components::item::Invlet>();
    world.register_component::<cdda_components::dev::DevPlayer>();
    world.register_component::<cdda_components::dev::DevGroundItemName>();
    world.register_component::<cdda_components::item::InProgressCraft>();

    // ── Turn scheduling ───────────────────────────────────────────────
    world.register_component::<cdda_components::actor::ActionPoints>();
    world.register_component::<cdda_components::actor::HandCount>();

    // ── Status markers ────────────────────────────────────────────────
    world.register_component::<cdda_components::actor::IsAlive>();
    world.register_component::<cdda_components::actor::Stunned>();
    world.register_component::<cdda_components::actor::Bleeding>();
    world.register_component::<cdda_components::actor::OnFire>();

    // ── Overmap ───────────────────────────────────────────────────────
    world.register_component::<cdda_overmap::chunk::ChunkPosition>();
    world.register_component::<cdda_overmap::chunk::OvermapChunk>();

    // ── Generation ────────────────────────────────────────────────────
    world.register_component::<crate::pipeline::OvermapEntity>();
    world.register_component::<crate::steps::finalize::Finalized>();
    world.register_component::<crate::steps::cities::City>();
}
