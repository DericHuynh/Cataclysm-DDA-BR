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

use cdda_components::def::*;
use cdda_components::item::{ItemQualities, QualityId};
use cdda_components::recipe::RecipeIndex;
use cdda_components::SkillId;

use crate::interner::{
    AmmoTypeRegistry, BodyPartRegistry, ComestibleRegistry, ItemTypeRegistry, QualityRegistry,
    SkillRegistry,
};

use bevy_ecs::prelude::*;

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
fn extract_ammo_damage(raw: &cdda_core_types::core::raw_defs::RawValue) -> Option<i32> {
    use cdda_core_types::core::raw_defs::RawValue;
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

fn extract_price(p: &cdda_core_types::core::raw_defs::CddaPrice) -> u64 {
    match p {
        cdda_core_types::core::raw_defs::CddaPrice::Numeric(c) => *c as u64,
        cdda_core_types::core::raw_defs::CddaPrice::Text(s) => s
            .split_whitespace()
            .next()
            .and_then(|w| w.parse::<f64>().ok())
            .map(|v| (v * 100.0).round() as u64)
            .unwrap_or(0),
    }
}

fn color_to_string(c: &cdda_core_types::core::raw_defs::CddaColor) -> String {
    match c {
        cdda_core_types::core::raw_defs::CddaColor::Named(s) => s.clone(),
        cdda_core_types::core::raw_defs::CddaColor::Multi(v) => v.join(","),
        cdda_core_types::core::raw_defs::CddaColor::Structured(s) => {
            s.fg.clone().unwrap_or_default()
        }
    }
}

pub fn flags_to_vec(f: &cdda_core_types::core::raw_defs::StringOrArray) -> Vec<String> {
    f.all_strings()
        .into_iter()
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect()
}

fn materials_to_vec(m: &cdda_core_types::core::raw_defs::MaterialList) -> Vec<String> {
    match m {
        cdda_core_types::core::raw_defs::MaterialList::Array(arr) => arr
            .iter()
            .map(|r| match r {
                cdda_core_types::core::raw_defs::MaterialRef::Single(id) => id.clone(),
                cdda_core_types::core::raw_defs::MaterialRef::Composite(c) => c.r#type.clone(),
                cdda_core_types::core::raw_defs::MaterialRef::Map(m) => {
                    m.keys().cloned().next().unwrap_or_default()
                }
            })
            .collect(),
        cdda_core_types::core::raw_defs::MaterialList::Map(map) => map.keys().cloned().collect(),
    }
}

/// Convert a slice of raw `ComponentOption` lists into `RecipeComponentEntry` slots.
fn parse_component_slots(
    slots: &[Vec<cdda_core_types::core::raw_defs::recipe::ComponentOption>],
    reg: &mut ItemTypeRegistry,
) -> Vec<Vec<RecipeComponentEntry>> {
    slots
        .iter()
        .map(|slot| {
            slot.iter()
                .map(|opt| {
                    let (item_id, count) = match opt {
                        cdda_core_types::core::raw_defs::recipe::ComponentOption::SimpleId(id) => {
                            (id.clone(), 1u32)
                        }
                        cdda_core_types::core::raw_defs::recipe::ComponentOption::Simple(id, c) => {
                            (id.clone(), *c)
                        }
                        cdda_core_types::core::raw_defs::recipe::ComponentOption::WithFlag(
                            id,
                            c,
                            _,
                        ) => (id.clone(), *c),
                        cdda_core_types::core::raw_defs::recipe::ComponentOption::Object(o) => {
                            (o.item.clone(), o.count.unwrap_or(1))
                        }
                    };
                    RecipeComponentEntry {
                        item_id: reg.intern(&item_id),
                        count,
                        recovered: false,
                    }
                })
                .collect()
        })
        .collect()
}

/// Extract a specific damage type amount from a `Vec<DamageByType>`.
/// DamageByType lives in cdda_core_types::core::raw_defs::monster.
fn extract_monster_melee_damage(
    damage_vec: &[cdda_core_types::core::raw_defs::monster::DamageByType],
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
    def_registry: &crate::DefRegistry,
    spawn_all: bool,
) -> DefinitionWorld {
    let mut def_world = DefinitionWorld::empty();

    if spawn_all {
        build_item_defs(world, def_registry, &mut def_world);
        build_monster_defs(world, def_registry, &mut def_world);
        build_terrain_defs(world, def_registry, &mut def_world);
        build_furniture_defs(world, def_registry, &mut def_world);
        build_recipe_defs(world, def_registry, &mut def_world);
    }
    build_body_part_defs(world, def_registry, &mut def_world);

    def_world
}

fn build_item_defs(
    world: &mut World,
    def_registry: &crate::DefRegistry,
    def_world: &mut DefinitionWorld,
) {
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
                ItemVolume(
                    cdda_core_types::core::units::Volume::as_milliliters(&item.volume) as u32,
                ),
                ItemSymbol(item.symbol.chars().next().unwrap_or('#')),
                ItemColor(item.color.as_ref().map(color_to_string).unwrap_or_default()),
                ItemMaterials(materials_to_vec(&item.material)),
                crate::flags::ItemFlags::new(),
                ItemPrice {
                    price: item.price.as_ref().map(extract_price).unwrap_or(0),
                    price_postapoc: item.price_postapoc.as_ref().map(extract_price).unwrap_or(0),
                },
                ItemPhase(match item.phase {
                    cdda_core_types::core::raw_defs::item::Phase::Solid => {
                        cdda_components::def::Phase::Solid
                    }
                    cdda_core_types::core::raw_defs::item::Phase::Liquid => {
                        cdda_components::def::Phase::Liquid
                    }
                    cdda_core_types::core::raw_defs::item::Phase::Gas => {
                        cdda_components::def::Phase::Gas
                    }
                    cdda_core_types::core::raw_defs::item::Phase::Plasma => {
                        cdda_components::def::Phase::Plasma
                    }
                }),
                ItemStackSize(item.stack_size.unwrap_or(1)),
                ItemCategory(item.category.clone().unwrap_or_default()),
            ))
            .id();
        // Intern quality strings and add ItemQualities after spawn
        {
            let qualities: Vec<(QualityId, i32)> = item
                .qualities
                .as_ref()
                .map(|q| {
                    let mut reg = world.resource_mut::<QualityRegistry>();
                    q.iter().map(|tq| (reg.intern(&tq.id), tq.level)).collect()
                })
                .unwrap_or_default();
            if !qualities.is_empty() {
                world.entity_mut(entity).insert(ItemQualities(qualities));
            }
        }

        // ── AMMO subtype ────────────────────────────────────────────
        if subtypes.iter().any(|s| s == "AMMO") {
            let ammo_damage = item
                .damage
                .as_ref()
                .and_then(|raw| extract_ammo_damage(raw))
                .unwrap_or(0);

            let ammo_type = world.resource_mut::<AmmoTypeRegistry>().intern(
                &item
                    .ammo_type
                    .as_ref()
                    .map(|sa| sa.first_or_default().to_string())
                    .unwrap_or_default(),
            );

            world.entity_mut(entity).insert(AmmoData {
                ammo_type,
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
            let gun_ammo = world.resource_mut::<AmmoTypeRegistry>().intern(
                &item
                    .ammo_type
                    .as_ref()
                    .map(|sa| sa.first_or_default().to_string())
                    .unwrap_or_default(),
            );
            world.entity_mut(entity).insert(GunData {
                skill: SkillId(0),
                ammo_type: gun_ammo,
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
                                    cdda_core_types::core::raw_defs::EncumbranceOrRange::Single(
                                        v,
                                    ) => *v as i32,
                                    cdda_core_types::core::raw_defs::EncumbranceOrRange::Range(
                                        v,
                                    ) => v.first().copied().unwrap_or(0) as i32,
                                })
                                .unwrap_or(0);
                            let body_part_str = bp
                                .covers
                                .as_ref()
                                .map(|c| match c {
                                    cdda_core_types::core::raw_defs::StringOrArray::Single(s) => {
                                        s.clone()
                                    }
                                    cdda_core_types::core::raw_defs::StringOrArray::Multi(v) => {
                                        v.join(", ")
                                    }
                                })
                                .unwrap_or_default();
                            let body_part_token = world
                                .resource_mut::<BodyPartRegistry>()
                                .intern(&body_part_str);
                            ArmourPart {
                                body_part: body_part_token,
                                coverage: bp.coverage.unwrap_or(0) as u8,
                                encumbrance: enc,
                                warmth: 0,
                                layers: bp.layers.clone().unwrap_or_default(),
                                specifically_covers: bp
                                    .specifically_covers
                                    .clone()
                                    .unwrap_or_default(),
                                material: bp
                                    .material
                                    .as_ref()
                                    .map(|mats| {
                                        mats.iter()
                                            .map(|m| {
                                                (
                                                    m.r#type.clone(),
                                                    m.thickness.unwrap_or(0.0),
                                                    m.covered_by_mat.unwrap_or(100) as f64,
                                                )
                                            })
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
            let comestible = world.resource_mut::<ComestibleRegistry>().intern(
                &item
                    .comestible_type
                    .as_ref()
                    .map(|ct| format!("{:?}", ct))
                    .unwrap_or_else(|| "INVALID".to_string()),
            );
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
                        cdda_core_types::core::raw_defs::CddaDuration::Number(n) => *n,
                        cdda_core_types::core::raw_defs::CddaDuration::Text(s) => s
                            .split_whitespace()
                            .next()
                            .and_then(|w| w.parse::<u32>().ok())
                            .unwrap_or(0),
                    })
                    .unwrap_or(0),
                comestible_type: comestible,
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
            let book_skill = world
                .resource_mut::<SkillRegistry>()
                .intern(&item.read_skill.clone().unwrap_or_default());
            world.entity_mut(entity).insert(BookData {
                skill: book_skill,
                required_level: item.required_level.unwrap_or(0) as u8,
                max_level: item.max_level.unwrap_or(0) as u8,
                // `read_fun` is the book-specific field; fall back to `fun`
                // (used by comestibles) if read_fun is not set.
                fun: item.read_fun.or(item.fun).unwrap_or(0),
                intelligence: item.intelligence.unwrap_or(0) as u8,
                // time field from JSON is a Time string ("30 m", "1 h" etc.)
                // which parses into turns. Convert turns → minutes for storage.
                time: item.time.map(|t| (t.as_turns() / 60) as u32).unwrap_or(0),
                chapters: item.chapters.unwrap_or(0),
                martial_art: item.martial_art.clone().unwrap_or_default(),
            });
        }

        // ── MAGAZINE subtype ────────────────────────────────────────
        if subtypes.iter().any(|s| s == "MAGAZINE") {
            let mag_ammo = world.resource_mut::<AmmoTypeRegistry>().intern(
                &item
                    .ammo_type
                    .as_ref()
                    .map(|sa| sa.first_or_default().to_string())
                    .unwrap_or_default(),
            );
            world.entity_mut(entity).insert(MagazineData {
                ammo_type: mag_ammo,
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
                Some(cdda_core_types::core::raw_defs::MeleeDamage::BashOnly(b)) => (*b, 0),
                Some(cdda_core_types::core::raw_defs::MeleeDamage::ByType(map)) => (
                    map.get("bash").copied().unwrap_or(0),
                    map.get("cut").copied().unwrap_or(0),
                ),
                Some(cdda_core_types::core::raw_defs::MeleeDamage::TypedArray(arr)) => {
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
                    cdda_core_types::core::raw_defs::ToHit::Number(n) => *n,
                    cdda_core_types::core::raw_defs::ToHit::Struct {
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
                skill: SkillId(0),
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
                                .map(|v| {
                                    cdda_core_types::core::units::Volume::as_milliliters(&v) as u32
                                })
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
}

// ── Monster definitions ───────────────────────────────────────────
fn build_monster_defs(
    world: &mut World,
    def_registry: &crate::DefRegistry,
    def_world: &mut DefinitionWorld,
) {
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
                crate::flags::MonsterFlags::new(),
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
}

// ── Terrain definitions ───────────────────────────────────────────
fn build_terrain_defs(
    world: &mut World,
    def_registry: &crate::DefRegistry,
    def_world: &mut DefinitionWorld,
) {
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
                crate::flags::TerrainFlags::new(),
                TerrainLightEmitted(terrain.light_emitted.unwrap_or(0)),
                TerrainHasCeiling(terrain.has_ceiling.unwrap_or(false)),
                TerrainConnectsTo(flags_to_vec(&terrain.connects_to)),
            ))
            .id();
        def_world.register(id_str, e);
    }
}

// ── Furniture definitions ─────────────────────────────────────────
fn build_furniture_defs(
    world: &mut World,
    def_registry: &crate::DefRegistry,
    def_world: &mut DefinitionWorld,
) {
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
                crate::flags::FurnitureFlags::new(),
                FurnitureMoveCostMod(furniture.move_cost_mod.unwrap_or(0)),
                FurnitureCoverage(furniture.coverage.unwrap_or(0)),
                FurnitureLightEmitted(furniture.light_emitted.unwrap_or(0)),
                FurnitureMaxVolume(
                    furniture
                        .max_volume
                        .map(|v| cdda_core_types::core::units::Volume::as_milliliters(&v) as u32)
                        .unwrap_or(0),
                ),
            ))
            .id();
        def_world.register(id_str, e);
    }
}

// ── Recipe definitions ────────────────────────────────────────────
fn build_recipe_defs(
    world: &mut World,
    def_registry: &crate::DefRegistry,
    def_world: &mut DefinitionWorld,
) {
    // Build a quick lookup: requirement ID → RequirementDef.
    let req_lookup: std::collections::HashMap<
        &str,
        &cdda_core_types::core::raw_defs::requirement::RequirementDef,
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
            let skill_token = world
                .resource_mut::<SkillRegistry>()
                .intern(&skill.as_str().to_string());
            world
                .entity_mut(entity)
                .insert(RecipeSkillUsed(skill_token));
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
            Some(cdda_core_types::core::raw_defs::recipe::Autolearn::Bool(b)) => *b,
            Some(cdda_core_types::core::raw_defs::recipe::Autolearn::Skills(v)) => !v.is_empty(),
            None => false,
        };
        world.entity_mut(entity).insert(RecipeAutolearn(autolearn));

        // Qualities: flatten alternatives, taking the first of each slot.
        if let Some(quals) = &recipe.qualities {
            let flat: Vec<(QualityId, u32)> = quals
                .iter()
                .filter_map(|q| match q {
                    cdda_core_types::core::raw_defs::recipe::QualityEntry::Single(qr) => {
                        let id = world.resource_mut::<QualityRegistry>().intern(&qr.id);
                        Some((id, qr.level))
                    }
                    cdda_core_types::core::raw_defs::recipe::QualityEntry::Alternative(alts) => {
                        alts.first().map(|qr| {
                            let id = world.resource_mut::<QualityRegistry>().intern(&qr.id);
                            (id, qr.level)
                        })
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
                let parsed =
                    parse_component_slots(slots, &mut world.resource_mut::<ItemTypeRegistry>());
                all_slots.extend(parsed);
            }

            // Resolve `using` references by inlining requirement component lists.
            if let Some(using_list) = &recipe.using {
                for entry in using_list {
                    let req_id = &entry.0;
                    let multiplier = entry.1;
                    if let Some(req) = req_lookup.get(req_id.as_str()) {
                        if let Some(comp_val) = &req.components {
                            if let Ok(slots) = serde_json::from_value::<
                                Vec<Vec<cdda_core_types::core::raw_defs::recipe::ComponentOption>>,
                            >(comp_val.clone())
                            {
                                let scaled = parse_component_slots(
                                    &slots,
                                    &mut world.resource_mut::<ItemTypeRegistry>(),
                                )
                                .into_iter()
                                .map(|slot| {
                                    slot.into_iter()
                                        .map(|mut e| {
                                            e.count = ((e.count as f64 * multiplier).ceil() as u32)
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
fn build_body_part_defs(
    world: &mut World,
    def_registry: &crate::DefRegistry,
    def_world: &mut DefinitionWorld,
) {
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

        let bp_id = world.resource_mut::<BodyPartRegistry>().intern(&id_str);
        let bp_side = world
            .resource_mut::<BodyPartRegistry>()
            .intern(&bp.side.clone().unwrap_or_else(|| "both".to_string()));
        let entity = world
            .spawn((
                IsDef,
                DefStrId(id_str.clone()),
                BodyPartDefId(bp_id),
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
                BodyPartSide(bp_side),
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
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DefRegistry;
    use cdda_core_types::core::id::DefId;

    /// Create an empty DefRegistry and register minimal component types.
    fn setup_world(world: &mut World) {
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
        world.register_component::<ItemStackSize>();
        world.register_component::<ItemCategory>();
        world.register_component::<ItemPrice>();
        world.register_component::<crate::flags::ItemFlags>();
        world.register_component::<ItemQualities>();
        world.register_component::<WeaponData>();
        world.register_component::<AmmoData>();
        world.register_component::<GunData>();
        world.register_component::<ArmourData>();
        world.register_component::<FoodData>();
        world.register_component::<ToolData>();
        world.register_component::<BookData>();
        world.register_component::<MagazineData>();
        world.register_component::<GunModData>();
        world.register_component::<ContainerData>();
        world.register_component::<MonsterName>();
        world.register_component::<MonsterDescription>();
        world.register_component::<MonsterStats>();
        world.register_component::<MonsterMelee>();
        world.register_component::<MonsterVision>();
        world.register_component::<MonsterArmour>();
        world.register_component::<MonsterSpecies>();
        world.register_component::<MonsterDefaultFaction>();
        world.register_component::<MonsterBodyType>();
        world.register_component::<crate::flags::MonsterFlags>();
        world.register_component::<TerrainName>();
        world.register_component::<TerrainSymbol>();
        world.register_component::<TerrainColor>();
        world.register_component::<TerrainMoveCost>();
        world.register_component::<TerrainLightEmitted>();
        world.register_component::<TerrainHasCeiling>();
        world.register_component::<TerrainConnectsTo>();
        world.register_component::<crate::flags::TerrainFlags>();
        world.register_component::<FurnitureName>();
        world.register_component::<FurnitureSymbol>();
        world.register_component::<FurnitureColor>();
        world.register_component::<FurnitureMoveCostMod>();
        world.register_component::<FurnitureCoverage>();
        world.register_component::<FurnitureLightEmitted>();
        world.register_component::<FurnitureMaxVolume>();
        world.register_component::<crate::flags::FurnitureFlags>();
        world.register_component::<BodyPartDefId>();
        world.register_component::<BodyPartName>();
        world.register_component::<BodyPartHitSize>();
        world.register_component::<BodyPartHitDifficulty>();
        world.register_component::<BodyPartBaseHp>();
        world.register_component::<BodyPartDrenchCapacity>();
        world.register_component::<BodyPartSide>();
        world.register_component::<IsVital>();
        world.register_component::<CanGrasp>();
        world.register_component::<CanWalk>();
        world.register_component::<CanSee>();
        world.register_component::<CanBite>();
        world.register_component::<CanFly>();
        world.register_component::<ParentPart>();
        world.register_component::<SubParts>();
        world.register_component::<BodyPartLegacyId>();
        world.init_resource::<BodyPartRegistry>();
        world.init_resource::<AmmoTypeRegistry>();
        world.init_resource::<ComestibleRegistry>();
        world.init_resource::<ItemTypeRegistry>();
        world.init_resource::<QualityRegistry>();
        world.init_resource::<SkillRegistry>();
    }

    // =======================================================================
    // DefinitionWorld tests
    // =======================================================================

    #[test]
    fn def_world_empty() {
        let dw = DefinitionWorld::empty();
        assert_eq!(dw.len(), 0);
        assert!(dw.entity_by_str("anything").is_none());
        assert!(dw.iter().next().is_none());
    }

    #[test]
    fn def_world_register_and_lookup() {
        let mut world = World::new();
        world.register_component::<IsDef>();
        let mut dw = DefinitionWorld::empty();

        let e = world.spawn(IsDef).id();
        dw.register("test_item".into(), e);

        assert_eq!(dw.len(), 1);
        assert_eq!(dw.entity_by_str("test_item"), Some(e));
        assert!(dw.entity_by_str("missing").is_none());
    }

    #[test]
    fn def_world_iter() {
        let mut world = World::new();
        world.register_component::<IsDef>();
        let mut dw = DefinitionWorld::empty();

        let e1 = world.spawn(IsDef).id();
        let e2 = world.spawn(IsDef).id();
        dw.register("a".into(), e1);
        dw.register("b".into(), e2);

        let pairs: Vec<(&str, Entity)> = dw.iter().collect();
        assert_eq!(pairs.len(), 2);
    }

    // =======================================================================
    // build_body_part_defs — always runs, so we test it directly
    // =======================================================================

    #[test]
    fn build_body_part_defs_empty_registry() {
        let mut world = World::new();
        setup_world(&mut world);
        let reg = DefRegistry::empty();
        let mut dw = DefinitionWorld::empty();

        // Should not panic with empty registry
        build_body_part_defs(&mut world, &reg, &mut dw);
        assert_eq!(dw.len(), 0);
    }

    // =======================================================================
    // Individual builder tests (with minimal DefRegistry)
    // =======================================================================

    /// Test that each builder function handles an empty registry without panicking.
    #[test]
    fn build_item_defs_empty_registry() {
        let mut world = World::new();
        setup_world(&mut world);
        let reg = DefRegistry::empty();
        let mut dw = DefinitionWorld::empty();
        build_item_defs(&mut world, &reg, &mut dw);
        assert_eq!(dw.len(), 0);
    }

    #[test]
    fn build_terrain_defs_empty_registry() {
        let mut world = World::new();
        setup_world(&mut world);
        let reg = DefRegistry::empty();
        let mut dw = DefinitionWorld::empty();
        build_terrain_defs(&mut world, &reg, &mut dw);
        assert_eq!(dw.len(), 0);
    }

    #[test]
    fn build_furniture_defs_empty_registry() {
        let mut world = World::new();
        setup_world(&mut world);
        let reg = DefRegistry::empty();
        let mut dw = DefinitionWorld::empty();
        build_furniture_defs(&mut world, &reg, &mut dw);
        assert_eq!(dw.len(), 0);
    }

    #[test]
    fn build_monster_defs_empty_registry() {
        let mut world = World::new();
        setup_world(&mut world);
        let reg = DefRegistry::empty();
        let mut dw = DefinitionWorld::empty();
        build_monster_defs(&mut world, &reg, &mut dw);
        assert_eq!(dw.len(), 0);
    }

    #[test]
    fn build_recipe_defs_empty_registry() {
        let mut world = World::new();
        setup_world(&mut world);
        let reg = DefRegistry::empty();
        let mut dw = DefinitionWorld::empty();
        build_recipe_defs(&mut world, &reg, &mut dw);
        assert_eq!(dw.len(), 0);
    }

    /// Integration: all empty-registry builders run without conflict.
    #[test]
    fn build_all_empty_registries() {
        let mut world = World::new();
        setup_world(&mut world);
        let reg = DefRegistry::empty();
        let mut dw = DefinitionWorld::empty();
        build_item_defs(&mut world, &reg, &mut dw);
        build_monster_defs(&mut world, &reg, &mut dw);
        build_terrain_defs(&mut world, &reg, &mut dw);
        build_furniture_defs(&mut world, &reg, &mut dw);
        build_recipe_defs(&mut world, &reg, &mut dw);
        build_body_part_defs(&mut world, &reg, &mut dw);
        assert_eq!(dw.len(), 0);
    }
}
