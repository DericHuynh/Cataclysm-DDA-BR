use crate::core::components::actor::*;
use bevy_app::{App, Plugin};

pub struct ActorPlugin;

impl Plugin for ActorPlugin {
    fn build(&self, app: &mut App) {
        // Creature identity
        app.register_type::<Creature>();
        app.register_type::<Gender>();
        app.register_type::<PlayerData>();
        app.register_type::<NpcPersonality>();
        app.register_type::<NpcData>();

        // Stats
        app.register_type::<Health>();
        app.register_type::<Stats>();
        app.register_type::<Faction>();
        app.register_type::<BodyTemperature>();
        app.register_type::<Wetness>();

        // Combat
        app.register_type::<DamageReduction>();
        app.register_type::<CombatStats>();
        app.register_type::<Vision>();

        // Skills
        app.register_type::<SkillOf>();
        app.register_type::<CreatureSkills>();
        app.register_type::<SkillEntry>();

        // Mutations
        app.register_type::<MutationOf>();
        app.register_type::<CreatureMutations>();
        app.register_type::<MutationEntry>();

        // Proficiencies
        app.register_type::<ProficiencyOf>();
        app.register_type::<CreatureProficiencies>();
        app.register_type::<ProficiencyEntry>();

        // Bionics
        app.register_type::<BionicOf>();
        app.register_type::<InstalledBionics>();
        app.register_type::<Bionic>();

        // Morale
        app.register_type::<MoraleBonusOf>();
        app.register_type::<MoraleBonuses>();
        app.register_type::<MoraleBonus>();
        app.register_type::<Morale>();

        // Status effects
        app.register_type::<EffectOn>();
        app.register_type::<ActiveEffects>();
        app.register_type::<StatusEffect>();

        // Turn scheduling + status markers
        app.register_type::<ActionPoints>();
        app.register_type::<IsAlive>();
        app.register_type::<Stunned>();
        app.register_type::<Bleeding>();
        app.register_type::<OnFire>();

        // Body parts
        app.register_type::<BodyPartOf>();
        app.register_type::<CreatureBodyParts>();
        app.register_type::<BodyPartDef>();
        app.register_type::<BodyPartSlot>();
        app.register_type::<BodyPartHp>();
        app.register_type::<BodyPartBroken>();
        app.register_type::<BodyPartSevered>();
    }
}
