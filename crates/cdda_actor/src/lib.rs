//! # cdda_actor — Creature, player, NPC, and stats domain crate
//!
//! Owns all ECS components related to creatures:
//! identity (`Creature`, `PlayerData`, `NpcData`), stats (`Health`, `Stats`),
//! combat (`CombatStats`, `Vision`), bionics, morale, status effects,
//! body parts, status markers, and turn scheduling (`Speed`, `MovePoints`).
//!
//! Depends only on `cdda_core` and `bevy_ecs`.  No item, map, or sim deps.

pub mod components;
pub mod plugin;
