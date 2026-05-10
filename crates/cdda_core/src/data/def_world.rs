//! # DefinitionWorld — index of definition entities in the main World
//!
//! Definitions are spawned as entities into the **main game World**, marked
//! with the `IsDef` component.  Each definition is an **entity** with exactly
//! the components its subtype requires.  Items with `"subtypes": ["AMMO"]` get
//! `AmmoData`; items with `"subtypes": ["ARMOR"]` get `ArmourData`; items with
//! both get both.  No monolithic structs.
//!
//! The `DefinitionWorld` resource is just a `HashMap<String, Entity>` index
//! that maps string IDs to the definition entities in the main World.
//! Systems that need definition data query directly:
//! `Query<&GunData, With<IsDef>>` — the entities are in the main World.

use crate::core::components::actor::{
    ActionPoints, Creature, Faction, Gender, Health, IsAlive, PlayerData,
};
use crate::core::components::def::*;
use crate::core::components::item::ItemQualities;
use crate::core::components::sim::{Solid, WorldPosition};
use crate::core::coords::WorldPos;
use crate::crafting::systems::RecipeIndex;
use crate::sim::state::{AppState, GameTime, LoadingStatus, StartupConfig};
use bevy_ecs::prelude::*;
use bevy_state::state::NextState;
use std::collections::HashMap;

// ===========================================================================
// Resource: DefinitionWorld
// ===========================================================================

/// Maps string definition IDs (e.g. "glock_17", "zombie") to the Entity
/// in the main game World that holds the definition components.
///
/// Definition entities are marked with `IsDef` and can be queried directly
/// from any system: `Query<&GunData, With<IsDef>>`.
#[derive(Resource, Debug, Default)]
pub struct DefinitionWorld {
    index: HashMap<String, Entity>,
}

impl DefinitionWorld {
    pub fn empty() -> Self {
        Self {
            index: HashMap::new(),
        }
    }

    /// Look up a definition entity by its string ID.
    pub fn entity_by_str(&self, id: &str) -> Option<Entity> {
        self.index.get(id).copied()
    }

    /// Register a mapping from string ID to entity.
    fn register(&mut self, id: String, entity: Entity) {
        self.index.insert(id, entity);
    }

    /// Number of registered definitions.
    pub fn len(&self) -> usize {
        self.index.len()
    }

    /// Iterate over all (id, entity) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&str, Entity)> + '_ {
        self.index.iter().map(|(id, &e)| (id.as_str(), e))
    }
}

// ===========================================================================
// Helper functions
// ===========================================================================

/// Parse a CDDA volume string like "250 ml" or "1 L" into milliliters.
fn parse_volume_string_to_ml(s: &str) -> Option<u32> {
    let s = s.trim();
    let split_idx = s
        .find(|c: char| c == ' ' || (!c.is_ascii_digit() && c != '.' && c != '-' && c != '+'))
        .unwrap_or(s.len());
    if split_idx == 0 || split_idx == s.len() {
        return None;
    }
    let value_str = s[..split_idx].trim();
    let unit_str = s[split_idx..].trim().to_lowercase();
    let value: f64 = value_str.parse().ok()?;

    match unit_str.as_str() {
        "ml" | "milliliter" | "milliliters" => Some(value.round() as u32),
        "l" | "liter" | "liters" => Some((value * 1000.0).round() as u32),
        _ => None,
    }
}

/// Parse a CDDA weight string like "100 g" or "1 kg" into grams.
fn parse_weight_string_to_grams(s: &str) -> Option<u32> {
    let s = s.trim();
    let split_idx = s
        .find(|c: char| c == ' ' || (!c.is_ascii_digit() && c != '.' && c != '-' && c != '+'))
        .unwrap_or(s.len());
    if split_idx == 0 || split_idx == s.len() {
        return None;
    }
    let value_str = s[..split_idx].trim();
    let unit_str = s[split_idx..].trim().to_lowercase();
    let value: f64 = value_str.parse().ok()?;

    match unit_str.as_str() {
        "g" | "gram" | "grams" => Some(value.round() as u32),
        "kg" | "kilogram" | "kilograms" => Some((value * 1000.0).round() as u32),
        "mg" | "milligram" | "milligrams" => Some((value / 1000.0).round() as u32),
        _ => None,
    }
}

/// Extract numeric ammo damage from a RawValue.
/// Handles: bare number, {"amount": 25}, [{"amount": 25}]
fn extract_ammo_damage(raw: &crate::core::raw_defs::RawValue) -> Option<i32> {
    use crate::core::raw_defs::RawValue;
    match raw {
        RawValue::Number(n) => Some(*n as i32),
        RawValue::String(s) => s.parse::<i32>().ok(),
        RawValue::Object(map) => map.get("amount").and_then(|v| match v {
            RawValue::Number(n) => Some(*n as i32),
            RawValue::String(s) => s.parse::<i32>().ok(),
            _ => None,
        }),
        RawValue::Array(arr) => arr.first().and_then(|v| match v {
            RawValue::Object(map) => map.get("amount").and_then(|v| match v {
                RawValue::Number(n) => Some(*n as i32),
                RawValue::String(s) => s.parse::<i32>().ok(),
                _ => None,
            }),
            RawValue::Number(n) => Some(*n as i32),
            _ => None,
        }),
        _ => None,
    }
}

fn extract_price(p: &crate::core::raw_defs::CddaPrice) -> u64 {
    match p {
        crate::core::raw_defs::CddaPrice::Numeric(c) => *c as u64,
        crate::core::raw_defs::CddaPrice::Text(s) => s
            .split_whitespace()
            .next()
            .and_then(|w| w.parse::<f64>().ok())
            .map(|v| (v * 100.0).round() as u64)
            .unwrap_or(0),
    }
}

fn color_to_string(c: &crate::core::raw_defs::CddaColor) -> String {
    match c {
        crate::core::raw_defs::CddaColor::Named(s) => s.clone(),
        crate::core::raw_defs::CddaColor::Multi(v) => v.join(","),
        crate::core::raw_defs::CddaColor::Structured(s) => s.fg.clone().unwrap_or_default(),
    }
}

pub fn flags_to_vec(f: &crate::core::raw_defs::StringOrArray) -> Vec<String> {
    f.all_strings()
        .into_iter()
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect()
}

fn materials_to_vec(m: &crate::core::raw_defs::MaterialList) -> Vec<String> {
    match m {
        crate::core::raw_defs::MaterialList::Array(arr) => arr
            .iter()
            .map(|r| match r {
                crate::core::raw_defs::MaterialRef::Single(id) => id.clone(),
                crate::core::raw_defs::MaterialRef::Composite(c) => c.r#type.clone(),
                crate::core::raw_defs::MaterialRef::Map(m) => {
                    m.keys().cloned().next().unwrap_or_default()
                }
            })
            .collect(),
        crate::core::raw_defs::MaterialList::Map(map) => map.keys().cloned().collect(),
    }
}

/// Convert a slice of raw `ComponentOption` lists into `RecipeComponentEntry` slots.
fn parse_component_slots(
    slots: &[Vec<crate::core::raw_defs::recipe::ComponentOption>],
) -> Vec<Vec<RecipeComponentEntry>> {
    slots
        .iter()
        .map(|slot| {
            slot.iter()
                .map(|opt| {
                    let (item_id, count) = match opt {
                        crate::core::raw_defs::recipe::ComponentOption::SimpleId(id) => {
                            (id.clone(), 1u32)
                        }
                        crate::core::raw_defs::recipe::ComponentOption::Simple(id, c) => {
                            (id.clone(), *c)
                        }
                        crate::core::raw_defs::recipe::ComponentOption::WithFlag(id, c, _) => {
                            (id.clone(), *c)
                        }
                        crate::core::raw_defs::recipe::ComponentOption::Object(o) => {
                            (o.item.clone(), o.count.unwrap_or(1))
                        }
                    };
                    RecipeComponentEntry {
                        item_id,
                        count,
                        recovered: false,
                    }
                })
                .collect()
        })
        .collect()
}

/// Extract a specific damage type amount from a `Vec<DamageByType>`.
/// DamageByType lives in crate::core::raw_defs::monster.
fn extract_monster_melee_damage(
    damage_vec: &[crate::core::raw_defs::monster::DamageByType],
    damage_type: &str,
) -> i32 {
    damage_vec
        .iter()
        .find(|d| d.damage_type == damage_type)
        .map(|d| d.amount as i32)
        .unwrap_or(0)
}

// ===========================================================================
// Builder
// ===========================================================================

/// Populate the DefinitionWorld from the loaded DefRegistry.
///
/// Spawns definition entities directly into the given `World`,
/// marked with `IsDef`.  Returns the `DefinitionWorld` index.
///
/// Uses `subtypes` from the JSON to determine which composable ECS components
/// each definition entity gets.  Items with `"subtypes": ["AMMO"]` get
/// `AmmoData` added alongside their base components.  Items with
/// `"subtypes": ["ARMOR", "TOOL"]` get both `ArmourData` and `ToolData`.
pub fn build_def_world(
    world: &mut World,
    def_registry: &crate::data::DefRegistry,
    spawn_all: bool,
) -> DefinitionWorld {
    let mut def_world = DefinitionWorld::empty();

    if spawn_all {
        // ── Item definitions ──────────────────────────────────────────────
        for (def_id, item) in &def_registry.items {
            let id_str = def_id.as_str().to_string();
            let subtypes: Vec<String> = item
                .subtypes
                .as_ref()
                .map(|v| v.iter().map(|s| s.to_uppercase()).collect())
                .unwrap_or_default();

            // ── Build component list based on subtypes ──────────────────

            // Every item gets: IsDef + DefStrId + base data + item-specific base
            let entity = world
                .spawn((
                    IsDef,
                    DefStrId(id_str.clone()),
                    ItemName(
                        item.name
                            .as_ref()
                            .map(|n| n.singular().to_string())
                            .unwrap_or_default(),
                    ),
                    ItemDescription(
                        item.description
                            .as_ref()
                            .map(|d| d.singular().to_string())
                            .unwrap_or_default(),
                    ),
                    ItemWeight(item.weight.map(|w| w.as_grams() as u32).unwrap_or(0)),
                    ItemVolume(crate::Volume::as_milliliters(&item.volume) as u32),
                    ItemSymbol(item.symbol.chars().next().unwrap_or('#')),
                    ItemColor(item.color.as_ref().map(color_to_string).unwrap_or_default()),
                    ItemMaterials(materials_to_vec(&item.material)),
                    crate::data::flags::ItemFlags::new(),
                    ItemPrice {
                        price: item.price.as_ref().map(extract_price).unwrap_or(0),
                        price_postapoc: item
                            .price_postapoc
                            .as_ref()
                            .map(extract_price)
                            .unwrap_or(0),
                    },
                    ItemPhase(match item.phase {
                        crate::core::raw_defs::item::Phase::Solid => {
                            crate::core::components::def::Phase::Solid
                        }
                        crate::core::raw_defs::item::Phase::Liquid => {
                            crate::core::components::def::Phase::Liquid
                        }
                        crate::core::raw_defs::item::Phase::Gas => {
                            crate::core::components::def::Phase::Gas
                        }
                        crate::core::raw_defs::item::Phase::Plasma => {
                            crate::core::components::def::Phase::Plasma
                        }
                    }),
                    ItemStackSize(item.stack_size.unwrap_or(1)),
                    ItemCategory(item.category.clone().unwrap_or_default()),
                    ItemQualities(item.qualities.as_ref().map_or(Vec::new(), |q| {
                        q.iter().map(|tq| (tq.id.clone(), tq.level)).collect()
                    })),
                ))
                .id();

            // ── AMMO subtype ────────────────────────────────────────────
            if subtypes.iter().any(|s| s == "AMMO") {
                // Extract damage from the ammo `damage` field (RawValue).
                // CDDA formats: a bare number, {"damage_type":"bullet", "amount":25},
                // or [{"damage_type":"bullet", "amount":25}].
                let ammo_damage = item
                    .damage
                    .as_ref()
                    .and_then(|raw| extract_ammo_damage(raw))
                    .unwrap_or(0);

                world.entity_mut(entity).insert(AmmoData {
                    ammo_type: item
                        .ammo_type
                        .as_ref()
                        .map(|sa| sa.first_or_default().to_string())
                        .unwrap_or_default(),
                    damage: ammo_damage,
                    pierce: item.pierce.unwrap_or(0),
                    range: item.range.unwrap_or(0),
                    dispersion: item.dispersion.unwrap_or(0),
                    recoil: item.recoil.unwrap_or(0),
                    count: i32::try_from(item.charges.unwrap_or(1)).expect("ammo count overflow"),
                    // CDDA uses `container` for the casing item ID on ammo
                    casing: item.container.clone(),
                    effects: Vec::new(),
                    stack_size: item.stack_size.unwrap_or(1) as u32,
                });
            }

            // ── GUN subtype ─────────────────────────────────────────────
            if subtypes.iter().any(|s| s == "GUN") {
                world.entity_mut(entity).insert(GunData {
                    skill: String::new(),
                    ammo_type: item
                        .ammo_type
                        .as_ref()
                        .map(|sa| sa.first_or_default().to_string())
                        .unwrap_or_default(),
                    dispersion: i32::try_from(item.charges.unwrap_or(0))
                        .expect("gun dispersion overflow"),
                    recoil: i32::try_from(item.charges_per_use.unwrap_or(0))
                        .expect("gun recoil overflow"),
                    reload_time: i32::try_from(item.charges.unwrap_or(0))
                        .expect("reload time overflow"),
                    clip_size: i32::try_from(item.max_charges.unwrap_or(0))
                        .expect("clip size overflow"),
                    // CDDA uses charges_per_use for burst size
                    burst: item.charges_per_use.unwrap_or(0) as u32,
                    ammo_effects: Vec::new(),
                });
            }

            // ── ARMOR / PET_ARMOR subtype ───────────────────────────────
            if subtypes.iter().any(|s| s == "ARMOR") || subtypes.iter().any(|s| s == "PET_ARMOR") {
                let parts = item
                    .armor
                    .as_ref()
                    .map(|armor_vec| {
                        armor_vec
                            .iter()
                            .map(|bp| {
                                let enc = bp
                                    .encumbrance
                                    .as_ref()
                                    .map(|e| match e {
                                        crate::core::raw_defs::EncumbranceOrRange::Single(v) => {
                                            *v as i32
                                        }
                                        crate::core::raw_defs::EncumbranceOrRange::Range(v) => {
                                            v.first().copied().unwrap_or(0) as i32
                                        }
                                    })
                                    .unwrap_or(0);
                                ArmourPart {
                                    body_part: bp
                                        .covers
                                        .as_ref()
                                        .map(|c| match c {
                                            crate::core::raw_defs::StringOrArray::Single(s) => {
                                                s.clone()
                                            }
                                            crate::core::raw_defs::StringOrArray::Multi(v) => {
                                                v.join(", ")
                                            }
                                        })
                                        .unwrap_or_default(),
                                    coverage: bp.coverage.unwrap_or(0) as u8,
                                    encumbrance: enc,
                                    warmth: 0,
                                    layers: bp.layers.clone().unwrap_or_default(),
                                    specifically_covers: bp.specifically_covers.clone().unwrap_or_default(),
                                    material: bp
                                        .material
                                        .as_ref()
                                        .map(|mats| {
                                            mats.iter()
                                                .map(|m| (
                                                    m.r#type.clone(),
                                                    m.thickness.unwrap_or(0.0),
                                                    m.covered_by_mat.unwrap_or(100) as f64,
                                                ))
                                                .collect()
                                        })
                                        .unwrap_or_default(),
                                }
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                world.entity_mut(entity).insert(ArmourData {
                    parts,
                    material_thickness: item.material_thickness.unwrap_or(0.0) as f32,
                    env_protection: [0; 5],
                });
            }

            // ── COMESTIBLE subtype ──────────────────────────────────────
            if subtypes.iter().any(|s| s == "COMESTIBLE") {
                world.entity_mut(entity).insert(FoodData {
                    calories: i32::try_from(item.calories.unwrap_or(0)).expect("calories overflow"),
                    quench: item.quench.unwrap_or(0),
                    fun: item.fun.unwrap_or(0),
                    healthy: 0,
                    stim: 0,
                    spoils_in: item
                        .spoils_in
                        .as_ref()
                        .map(|d| match d {
                            crate::core::raw_defs::CddaDuration::Number(n) => *n,
                            crate::core::raw_defs::CddaDuration::Text(s) => s
                                .split_whitespace()
                                .next()
                                .and_then(|w| w.parse::<u32>().ok())
                                .unwrap_or(0),
                        })
                        .unwrap_or(0),
                    comestible_type: item
                        .comestible_type
                        .as_ref()
                        .map(|ct| format!("{:?}", ct))
                        .unwrap_or_else(|| "INVALID".to_string()),
                });
            }

            // ── TOOL subtype ────────────────────────────────────────────
            if subtypes.iter().any(|s| s == "TOOL") {
                world.entity_mut(entity).insert(ToolData {
                    max_charges: i32::try_from(item.max_charges.unwrap_or(0))
                        .expect("tool max_charges overflow"),
                    charges_per_use: i32::try_from(item.charges_per_use.unwrap_or(0))
                        .expect("tool charges_per_use overflow"),
                    turns_per_charge: 1,
                    ammo_type: item
                        .tool_ammo
                        .as_ref()
                        .map(|sa| sa.first_or_default().to_string()),
                    revert_to: None,
                    power_draw: None,
                });
            }

            // ── BOOK subtype ────────────────────────────────────────────
            if subtypes.iter().any(|s| s == "BOOK") {
                world.entity_mut(entity).insert(BookData {
                    skill: String::new(),
                    required_level: 0,
                    max_level: item.max_charges.unwrap_or(0) as u8,
                    fun: item.fun.unwrap_or(0),
                    intelligence: item.charges.unwrap_or(0) as u8,
                    time: item.charges.unwrap_or(0),
                    chapters: 0,
                });
            }

            // ── MAGAZINE subtype ────────────────────────────────────────
            if subtypes.iter().any(|s| s == "MAGAZINE") {
                world.entity_mut(entity).insert(MagazineData {
                    ammo_type: item
                        .ammo_type
                        .as_ref()
                        .map(|sa| sa.first_or_default().to_string())
                        .unwrap_or_default(),
                    capacity: i32::try_from(item.max_charges.unwrap_or(0))
                        .expect("mag capacity overflow"),
                    reload_time: i32::try_from(item.charges.unwrap_or(0))
                        .expect("mag reload_time overflow"),
                    linkage: None,
                    default_ammo: String::new(),
                });
            }

            // ── GUNMOD subtype ──────────────────────────────────────────
            if subtypes.iter().any(|s| s == "GUNMOD") {
                world.entity_mut(entity).insert(GunModData {
                    install_time: item.charges.unwrap_or(0),
                });
            }

            // ── Melee weapon (any item with melee_damage) ───────────────
            if item.melee_damage.is_some() {
                let (dmg_bash, dmg_cut) = match &item.melee_damage {
                    Some(crate::core::raw_defs::MeleeDamage::BashOnly(b)) => (*b, 0),
                    Some(crate::core::raw_defs::MeleeDamage::ByType(map)) => (
                        map.get("bash").copied().unwrap_or(0),
                        map.get("cut").copied().unwrap_or(0),
                    ),
                    Some(crate::core::raw_defs::MeleeDamage::TypedArray(arr)) => {
                        let mut b = 0;
                        let mut c = 0;
                        for td in arr {
                            match td.damage_type.as_str() {
                                "bash" => b = td.amount,
                                "cut" => c = td.amount,
                                _ => {}
                            }
                        }
                        (b, c)
                    }
                    None => (0, 0),
                };
                let to_hit_val = item
                    .to_hit
                    .as_ref()
                    .map(|t| match t {
                        crate::core::raw_defs::ToHit::Number(n) => *n,
                        crate::core::raw_defs::ToHit::Struct {
                            grip,
                            length,
                            surface,
                            balance,
                        } => {
                            // Rough approximation of CDDA's to-hit calculation
                            // from structured weapon properties.
                            let mut total = 0i32;
                            if let Some(g) = grip.as_deref() {
                                total += match g {
                                    "weapon" => 0,
                                    "solid" => 20,
                                    "none" => -20,
                                    _ => 0,
                                };
                            }
                            if let Some(l) = length.as_deref() {
                                total += match l {
                                    "hand" => 0,
                                    "short" => -10,
                                    "long" => 10,
                                    _ => 0,
                                };
                            }
                            if let Some(s) = surface.as_deref() {
                                total += match s {
                                    "any" | "regular" | "every" => 0,
                                    "point" | "line" => -20,
                                    _ => 0,
                                };
                            }
                            if let Some(b) = balance.as_deref() {
                                total += match b {
                                    "neutral" => 3,
                                    "good" => 6,
                                    "clumsy" => -2,
                                    _ => 0,
                                };
                            }
                            total
                        }
                    })
                    .unwrap_or(0);

                world.entity_mut(entity).insert(WeaponData {
                    damage_bash: dmg_bash,
                    damage_cut: dmg_cut,
                    damage_stab: 0,
                    to_hit: to_hit_val,
                    moves_per_attack: 100,
                    reach: 1,
                    techniques: item.techniques.as_ref().cloned().unwrap_or_default(),
                    dice: 0,
                    dice_sides: 0,
                    skill: String::new(),
                });
            }

            // ── Container (any item with pocket_data) ──────────────────
            if item.pocket_data.is_some() {
                let pockets: Vec<PocketTemplate> = item
                    .pocket_data
                    .as_ref()
                    .map(|pd| {
                        pd.iter()
                            .map(|p| {
                                // max_volume can come from two CDDA fields:
                                //   - max_volume (a Volume, already deserialized)
                                //   - max_contains_volume (a string like "2 L")
                                // Try max_volume first, fall back to parsing the
                                // max_contains_volume string.
                                let max_vol = p
                                    .max_volume
                                    .map(|v| crate::Volume::as_milliliters(&v) as u32)
                                    .or_else(|| {
                                        p.max_contains_volume
                                            .as_ref()
                                            .and_then(|s| parse_volume_string_to_ml(s))
                                    })
                                    .unwrap_or(0);

                                PocketTemplate {
                                    pocket_type: format!("{:?}", p.pocket_type),
                                    max_volume: max_vol,
                                    max_weight: p
                                        .max_weight
                                        .map(|w| w.as_grams() as u32)
                                        .or_else(|| {
                                            p.max_contains_weight
                                                .as_ref()
                                                .and_then(|s| parse_weight_string_to_grams(s))
                                        })
                                        .unwrap_or(0),
                                    max_item_length: 0,
                                    sealed: p.sealed.unwrap_or(false),
                                    rigid: p.rigid.unwrap_or(false),
                                    holster: p.holster.unwrap_or(false),
                                    ablative: p.ablative.unwrap_or(false),
                                    description: p.description.clone().unwrap_or_default(),
                                    flag_restriction: p.flag_restriction.clone().unwrap_or_default(),
                                }
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                let total_volume: u32 = pockets.iter().map(|p| p.max_volume).sum();
                let total_weight: u32 = pockets.iter().map(|p| p.max_weight).sum();

                world.entity_mut(entity).insert(ContainerData {
                    pockets,
                    max_volume: total_volume,
                    max_weight: total_weight,
                });
            }

            def_world.register(id_str, entity);
        }

        // ── Monster definitions ───────────────────────────────────────────
        for (def_id, monster) in &def_registry.monsters {
            let id_str = def_id.as_str().to_string();

            let species_list = flags_to_vec(&monster.species);

            let monster_entity = world
                .spawn((
                    IsDef,
                    DefStrId(id_str.clone()),
                    MonsterName(
                        monster
                            .name
                            .as_ref()
                            .map(|n| n.singular().to_string())
                            .unwrap_or_default(),
                    ),
                    MonsterDescription(
                        monster
                            .description
                            .as_ref()
                            .map(|d| d.singular().to_string())
                            .unwrap_or_default(),
                    ),
                    MonsterStats {
                        hp: monster.hp.max(1),
                        speed: if monster.speed > 0 {
                            monster.speed
                        } else {
                            100
                        },
                        attack_cost: 100,
                        dodge: monster.melee_dice as i32,
                        morale: monster.morale,
                        aggression: monster.aggression,
                        melee_skill: monster.melee_skill,
                        melee_dice: monster.melee_dice as i32,
                        melee_dice_sides: monster.melee_dice_sides as i32,
                        grab_strength: monster.grab_strength.unwrap_or(0),
                        bleed_rate: monster.bleed_rate.unwrap_or(0),
                        diff: monster.diff.unwrap_or(0),
                    },
                    MonsterMelee {
                        dice: monster.melee_dice as u32,
                        dice_sides: monster.melee_dice_sides as u32,
                        damage_bash: extract_monster_melee_damage(&monster.melee_damage, "bash"),
                        damage_cut: extract_monster_melee_damage(&monster.melee_damage, "cut"),
                        damage_stab: extract_monster_melee_damage(&monster.melee_damage, "stab"),
                        to_hit: 0,
                    },
                    MonsterVision {
                        day: monster.vision_day as u32,
                        night: monster.vision_night as u32,
                    },
                    crate::data::flags::MonsterFlags::new(),
                    MonsterSpecies(species_list),
                    MonsterDefaultFaction(monster.default_faction.clone().unwrap_or_default()),
                    MonsterBodyType(monster.bodytype.clone().unwrap_or_default()),
                ))
                .id();

            if let Some(armor) = &monster.armor {
                world.entity_mut(monster_entity).insert(MonsterArmour {
                    bash: armor.bash,
                    cut: armor.cut,
                    bullet: armor.bullet,
                    stab: armor.stab,
                    fire: armor.heat,
                    acid: armor.acid,
                    electric: armor.electric,
                    cold: armor.cold,
                });
            }

            def_world.register(id_str, monster_entity);
        }

        // ── Terrain definitions ───────────────────────────────────────────
        for (def_id, terrain) in &def_registry.terrain {
            let id_str = def_id.as_str().to_string();
            let e = world
                .spawn((
                    IsDef,
                    DefStrId(id_str.clone()),
                    TerrainName(id_str.clone()),
                    TerrainSymbol(terrain.symbol.chars().next().unwrap_or('#')),
                    TerrainColor(
                        terrain
                            .color
                            .as_ref()
                            .map(color_to_string)
                            .unwrap_or_default(),
                    ),
                    TerrainMoveCost(terrain.move_cost.max(0)),
                    crate::data::flags::TerrainFlags::new(),
                    TerrainLightEmitted(terrain.light_emitted.unwrap_or(0)),
                    TerrainHasCeiling(terrain.has_ceiling.unwrap_or(false)),
                    TerrainConnectsTo(flags_to_vec(&terrain.connects_to)),
                ))
                .id();
            def_world.register(id_str, e);
        }

        // ── Furniture definitions ─────────────────────────────────────────
        for (def_id, furniture) in &def_registry.furniture {
            let id_str = def_id.as_str().to_string();
            let e = world
                .spawn((
                    IsDef,
                    DefStrId(id_str.clone()),
                    FurnitureName(
                        furniture
                            .name
                            .as_ref()
                            .map(|n| n.singular().to_string())
                            .unwrap_or_default(),
                    ),
                    FurnitureSymbol(furniture.symbol.chars().next().unwrap_or('#')),
                    FurnitureColor(
                        furniture
                            .color
                            .as_ref()
                            .map(color_to_string)
                            .unwrap_or_default(),
                    ),
                    crate::data::flags::FurnitureFlags::new(),
                    FurnitureMoveCostMod(furniture.move_cost_mod.unwrap_or(0)),
                    FurnitureCoverage(furniture.coverage.unwrap_or(0)),
                    FurnitureLightEmitted(furniture.light_emitted.unwrap_or(0)),
                    FurnitureMaxVolume(
                        furniture
                            .max_volume
                            .map(|v| crate::Volume::as_milliliters(&v) as u32)
                            .unwrap_or(0),
                    ),
                ))
                .id();
            def_world.register(id_str, e);
        }

        // ── Recipe definitions ────────────────────────────────────────────
        // Build a quick lookup: requirement ID → RequirementDef.
        let req_lookup: std::collections::HashMap<
            &str,
            &crate::core::raw_defs::requirement::RequirementDef,
        > = def_registry
            .requirements
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_ref()))
            .collect();

        // Only keep recipes whose result is a real item — filters out faction
        // camp blueprints, construction recipes, practice recipes, etc.
        let item_ids: std::collections::HashSet<&str> =
            def_registry.items.keys().map(|k| k.as_str()).collect();

        let mut recipe_entities: Vec<Entity> = Vec::new();
        for (_def_id, recipe) in &def_registry.recipes {
            // Skip abstract recipes and those with no result item.
            if recipe.abstract_.unwrap_or(false) {
                continue;
            }
            let result_id = match recipe.result.as_ref() {
                Some(id) => id.as_str().to_string(),
                None => continue,
            };
            // Skip non-item results (blueprints, construction, practice, etc.)
            if !item_ids.contains(result_id.as_str()) {
                continue;
            }

            let result_count = recipe.result_mult.unwrap_or(1).max(1);

            let time_turns = recipe
                .time
                .as_ref()
                .map(|t| t.as_turns().max(0) as u32)
                .unwrap_or(0);

            let entity = world
                .spawn((
                    IsDef,
                    IsRecipeDef,
                    RecipeResult(result_id),
                    RecipeResultCount(result_count),
                    RecipeTime(time_turns),
                    RecipeDifficulty(recipe.difficulty),
                ))
                .id();

            if let Some(skill) = &recipe.skill_used {
                world
                    .entity_mut(entity)
                    .insert(RecipeSkillUsed(skill.as_str().to_string()));
            }

            if let Some(cat) = &recipe.category {
                world.entity_mut(entity).insert(RecipeCategory(cat.clone()));
            }
            if let Some(sub) = &recipe.subcategory {
                world
                    .entity_mut(entity)
                    .insert(RecipeSubcategory(sub.clone()));
            }

            let autolearn = match &recipe.autolearn {
                Some(crate::core::raw_defs::recipe::Autolearn::Bool(b)) => *b,
                Some(crate::core::raw_defs::recipe::Autolearn::Skills(v)) => !v.is_empty(),
                None => false,
            };
            world.entity_mut(entity).insert(RecipeAutolearn(autolearn));

            // Qualities: flatten alternatives, taking the first of each slot.
            if let Some(quals) = &recipe.qualities {
                let flat: Vec<(String, u32)> = quals
                    .iter()
                    .filter_map(|q| match q {
                        crate::core::raw_defs::recipe::QualityEntry::Single(qr) => {
                            Some((qr.id.clone(), qr.level))
                        }
                        crate::core::raw_defs::recipe::QualityEntry::Alternative(alts) => {
                            alts.first().map(|qr| (qr.id.clone(), qr.level))
                        }
                    })
                    .collect();
                if !flat.is_empty() {
                    world.entity_mut(entity).insert(RecipeQualities(flat));
                }
            }

            // Components: inline direct slots + resolved `using` requirement templates.
            {
                let mut all_slots: Vec<Vec<RecipeComponentEntry>> = Vec::new();

                // Direct component slots from this recipe's `components` field.
                if let Some(slots) = &recipe.components {
                    let parsed = parse_component_slots(slots);
                    all_slots.extend(parsed);
                }

                // Resolve `using` references by inlining requirement component lists.
                if let Some(using_list) = &recipe.using {
                    for entry in using_list {
                        let req_id = &entry.0;
                        let multiplier = entry.1;
                        if let Some(req) = req_lookup.get(req_id.as_str()) {
                            if let Some(comp_val) = &req.components {
                                if let Ok(slots) =
                                    serde_json::from_value::<
                                        Vec<Vec<crate::core::raw_defs::recipe::ComponentOption>>,
                                    >(comp_val.clone())
                                {
                                    let scaled = parse_component_slots(&slots)
                                        .into_iter()
                                        .map(|slot| {
                                            slot.into_iter()
                                                .map(|mut e| {
                                                    e.count = ((e.count as f64 * multiplier).ceil()
                                                        as u32)
                                                        .max(1);
                                                    e
                                                })
                                                .collect()
                                        })
                                        .collect::<Vec<_>>();
                                    all_slots.extend(scaled);
                                }
                            }
                        }
                    }
                }

                if !all_slots.is_empty() {
                    world.entity_mut(entity).insert(RecipeComponents(all_slots));
                }
            }

            recipe_entities.push(entity);
        }
        world.insert_resource(RecipeIndex(recipe_entities));
    }
    // ── Body part definitions ─────────────────────────────────────────
    for (def_id, bp) in &def_registry.body_parts {
        let id_str = def_id.as_str().to_string();

        // Extract numeric values from serde_json::Value
        let hit_size: f32 = bp
            .hit_size
            .as_ref()
            .and_then(|v| {
                v.as_f64()
                    .map(|f| f as f32)
                    .or_else(|| v.as_u64().map(|u| u as f32))
            })
            .unwrap_or(1.0);
        let hit_difficulty: f32 = bp
            .hit_difficulty
            .as_ref()
            .and_then(|v| {
                v.as_f64()
                    .map(|f| f as f32)
                    .or_else(|| v.as_u64().map(|u| u as f32))
            })
            .unwrap_or(0.0);

        let entity = world
            .spawn((
                IsDef,
                DefStrId(id_str.clone()),
                BodyPartDefId(id_str.clone()),
                BodyPartName(
                    bp.name
                        .as_ref()
                        .map(|n| n.singular().to_string())
                        .unwrap_or_default(),
                ),
                BodyPartHitSize(hit_size),
                BodyPartHitDifficulty(hit_difficulty),
                BodyPartBaseHp(bp.base_hp.unwrap_or(60) as f32),
                BodyPartDrenchCapacity(bp.drench_capacity.unwrap_or(100)),
                BodyPartSide(bp.side.clone().unwrap_or_else(|| "both".to_string())),
            ))
            .id();

        // Legacy ID
        if let Some(legacy) = &bp.legacy_id {
            world
                .entity_mut(entity)
                .insert(BodyPartLegacyId(legacy.clone()));
        }

        // Vital marker
        if bp.is_vital.unwrap_or(false) {
            world.entity_mut(entity).insert(IsVital);
        }

        // Limb type → capability markers
        if let Some(limb_type) = &bp.limb_type {
            match limb_type.as_str() {
                "hand" | "arm" | "tentacle" => {
                    world.entity_mut(entity).insert(CanGrasp);
                }
                "leg" | "foot" | "fin" => {
                    world.entity_mut(entity).insert(CanWalk);
                }
                "sensor" | "eye" => {
                    world.entity_mut(entity).insert(CanSee);
                }
                "mouth" | "beak" => {
                    world.entity_mut(entity).insert(CanBite);
                }
                "wing" => {
                    world.entity_mut(entity).insert(CanFly);
                }
                _ => {}
            }
        }

        def_world.register(id_str, entity);
    }

    // Second pass: wire up parent-child relationships.
    // sub_parts (explicit children in JSON) and main_part (parent reference).
    for (def_id, bp) in &def_registry.body_parts {
        let child = def_world.entity_by_str(def_id.as_str());

        // 1. Wire sub_parts → ParentPart on children
        if let Some(sub_ids) = &bp.sub_parts {
            if let Some(parent) = def_world.entity_by_str(def_id.as_str()) {
                for child_id in sub_ids {
                    if let Some(child_entity) = def_world.entity_by_str(child_id) {
                        if child_entity != parent && world.get::<ParentPart>(child_entity).is_none()
                        {
                            world.entity_mut(child_entity).insert(ParentPart(parent));
                        }
                    }
                }
            }
        }

        // 2. Wire main_part → ParentPart (for parts not in sub_parts, e.g. eyes → head)
        if let Some(child_entity) = child {
            if let Some(main_part_id) = &bp.main_part {
                if let Some(parent) = def_world.entity_by_str(main_part_id) {
                    if child_entity != parent && world.get::<ParentPart>(child_entity).is_none() {
                        world.entity_mut(child_entity).insert(ParentPart(parent));
                    }
                }
            }
        }
    }

    def_world
}

// ===========================================================================
// CityBuildings — resource wrapper for worldgen access
// ===========================================================================

/// Thin wrapper to store city_building definitions as a Bevy resource.
/// Extracted from `DefRegistry` after loading so worldgen can access them.
#[derive(Resource, Debug, Clone)]
pub struct CityBuildings(
    pub  std::collections::HashMap<
        crate::core::id::DefId<crate::core::raw_defs::city_building::CityBuildingDef>,
        std::sync::Arc<crate::core::raw_defs::city_building::CityBuildingDef>,
    >,
);

// ===========================================================================
// Startup system — load JSON data and build DefinitionWorld
// ===========================================================================

pub fn load_data_system(world: &mut World) {
    use crate::data::loader::Loader;
    use tracing::info;

    info!("Data loading deferred until player starts game");

    let data_dirs = world.resource::<StartupConfig>().data_dirs.clone();

    world.resource_mut::<LoadingStatus>().current_phase = "Scanning JSON files...".into();
    info!("Loading data from {:?}", data_dirs);

    let mut loader = Loader::new(data_dirs);

    world.resource_mut::<LoadingStatus>().current_phase = "Ingesting raw definitions...".into();
    let raw_map = loader.ingest_all();
    let total_raw: usize = raw_map.values().map(|v| v.len()).sum();
    world.resource_mut::<LoadingStatus>().total_defs = total_raw;
    info!("Ingested {} raw definitions", total_raw);

    world.resource_mut::<LoadingStatus>().current_phase =
        "Resolving copy-from inheritance...".into();
    match loader.load() {
        Ok(registry) => {
            let count = registry.total_count();
            info!("Data loading complete: {} resolved definitions", count);

            world.resource_mut::<LoadingStatus>().current_phase =
                "Building definition entities...".into();
            let def_world = build_def_world(world, &registry, true);
            crate::data::populate_flags::populate_def_flags(world, &registry, &def_world);
            crate::data::schema_gen::collect_and_generate_schemas(world);
            info!(
                "DefinitionWorld: {} items, {} terrain, {} furniture, {} monsters",
                registry.items.len(),
                registry.terrain.len(),
                registry.furniture.len(),
                registry.monsters.len(),
            );

            // Store the city_buildings for dev-worldgen access
            world.insert_resource(CityBuildings(registry.city_buildings.clone()));

            world.insert_resource(def_world);
            world.insert_resource(GameTime::default());

            world.resource_mut::<LoadingStatus>().current_phase = "Complete".into();
            world.resource_mut::<LoadingStatus>().total_defs = count;
            world
                .resource_mut::<NextState<AppState>>()
                .set(AppState::WorldGen);
        }
        Err(errors) => {
            for err in &errors {
                tracing::warn!("Data loading error: {:?}", err);
            }
            info!(
                "Data loading finished with {} non-fatal errors, continuing...",
                errors.len()
            );
            world
                .resource_mut::<NextState<AppState>>()
                .set(AppState::WorldGen);
        }
    }
}

// ===========================================================================
// Worldgen system - dev-worldgen: one of every building
// ===========================================================================

pub fn worldgen_system(world: &mut World) {
    use tracing::info;

    let has_defs = world.get_resource::<DefinitionWorld>().is_some();

    // --- Dev-worldgen: populate WorldMap with one of every city building ---
    if has_defs {
        let city_buildings = world.remove_resource::<CityBuildings>();
        let config = world
            .get_resource::<crate::worldgen::dev::DevWorldgenConfig>()
            .cloned()
            .unwrap_or_default();

        if let Some(cb) = city_buildings {
            let building_count = cb.0.len();
            info!(
                "Dev-worldgen: generating showcase with {} city buildings...",
                building_count
            );

            let mut world_map = world.resource_mut::<crate::worldgen::setup::WorldMapResource>();
            let placed =
                crate::worldgen::dev::generate_dev_worldmap(&mut world_map.0, &cb.0, &config);
            info!(
                "Dev-worldgen complete: {} buildings placed, {} bubbles created",
                placed,
                world_map.0.bubble_count(),
            );
        }
    }

    if has_defs {
        let pos = WorldPos::new(0, 0, crate::ZLevel::new(0));
        world.spawn((
            PlayerData {
                name: "Survivor".into(),
                gender: Gender::Male,
                age: 25,
                height: 175,
                blood_type: "O+".into(),
                profession: None,
                scenario: None,
            },
            IsAlive,
            WorldPosition(pos),
            Creature {
                def_id: "player".into(),
                name: "Survivor".into(),
                species: crate::SpeciesId::from(0u32),
                symbol: '@',
            },
            Health {
                current: 100,
                max: 100,
            },
            Faction {
                id: crate::FactionId::from(0u32),
            },
            Solid,
            ActionPoints { current: 100, speed: 100 },
        ));
        info!("Spawned player at origin (0,0). Use the map to explore all buildings.");
    }
    world
        .resource_mut::<NextState<AppState>>()
        .set(AppState::InGame);
}
