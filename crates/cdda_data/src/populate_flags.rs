//! Populate flag components on definition entities from the DefRegistry.
//!
//! Called after `build_def_world`. Reads flag strings from the
//! `DefRegistry`, registers them in the per-category `FlagMap` resources,
//! and inserts the resulting `FixedBitSet` into each entity's flag component.

use bevy_ecs::prelude::*;
use fixedbitset::FixedBitSet;

use crate::def_world::{flags_to_vec, DefCategory, DefinitionWorld};
use crate::flags::{
    FurnitureFlagRegistry, FurnitureFlags, ItemFlagRegistry, ItemFlags, MonsterFlagRegistry,
    MonsterFlags, TerrainFlagRegistry, TerrainFlags,
};

/// Populate all flag components on definition entities.
///
/// Iterates every item/monster/terrain/furniture in the `DefRegistry`,
/// registers their flag strings into the corresponding `*FlagRegistry`
/// resource, and inserts the resulting bitset on the entity.
///
/// Must be called after `CddaDataPlugin` has inserted the registry resources
/// and `build_def_world` has spawned the definition entities.
pub fn populate_def_flags(
    world: &mut World,
    registry: &crate::DefRegistry,
    def_world: &DefinitionWorld,
) {
    // ── Item flags ───────────────────────────────────────────────────
    {
        let pairs: Vec<(Entity, FixedBitSet)> = {
            let mut reg = world.resource_mut::<ItemFlagRegistry>();
            registry
                .items
                .iter()
                .filter_map(|(def_id, item)| {
                    let entity = def_world.entity_in(DefCategory::Item, def_id.as_str())?;
                    let bitset = reg.0.register_all(&item.flags);
                    Some((entity, bitset))
                })
                .collect()
        };
        for (entity, bitset) in pairs {
            world.entity_mut(entity).insert(ItemFlags(bitset));
        }
    }
    // ── Monster flags ────────────────────────────────────────────────
    {
        let pairs: Vec<(Entity, FixedBitSet)> = {
            let mut reg = world.resource_mut::<MonsterFlagRegistry>();
            registry
                .monsters
                .iter()
                .filter_map(|(def_id, m)| {
                    let entity = def_world.entity_in(DefCategory::Monster, def_id.as_str())?;
                    let bitset = reg.0.register_all(&m.flags);
                    Some((entity, bitset))
                })
                .collect()
        };
        for (entity, bitset) in pairs {
            world.entity_mut(entity).insert(MonsterFlags(bitset));
        }
    }
    // ── Terrain flags ────────────────────────────────────────────────
    {
        let pairs: Vec<(Entity, FixedBitSet)> = {
            let mut reg = world.resource_mut::<TerrainFlagRegistry>();
            registry
                .terrain
                .iter()
                .filter_map(|(def_id, t)| {
                    let entity = def_world.entity_in(DefCategory::Terrain, def_id.as_str())?;
                    let strings = flags_to_vec(&t.flags);
                    let bitset = reg.0.register_all(&strings);
                    Some((entity, bitset))
                })
                .collect()
        };
        for (entity, bitset) in pairs {
            world.entity_mut(entity).insert(TerrainFlags(bitset));
        }
    }
    // ── Furniture flags ──────────────────────────────────────────────
    {
        let pairs: Vec<(Entity, FixedBitSet)> = {
            let mut reg = world.resource_mut::<FurnitureFlagRegistry>();
            registry
                .furniture
                .iter()
                .filter_map(|(def_id, f)| {
                    let entity = def_world.entity_in(DefCategory::Furniture, def_id.as_str())?;
                    let strings = flags_to_vec(&f.flags);
                    let bitset = reg.0.register_all(&strings);
                    Some((entity, bitset))
                })
                .collect()
        };
        for (entity, bitset) in pairs {
            world.entity_mut(entity).insert(FurnitureFlags(bitset));
        }
    }
}
