//! Debug registry viewer — browse all definition registries.
//!
//! Opens with F3 in gameplay. Three-panel layout:
//!   LEFT: registry categories   MIDDLE: entry IDs   RIGHT: detail panel
//!
//! The detail panel is split into TWO sub-panels side by side:
//!   LEFT HALF: Raw JSON text (from the original file data)
//!   RIGHT HALF: Parsed Rust struct fields (field-by-field display)
//!
//! Reads directly from the `DefRegistryResource` (a Bevy Resource saved after
//! data loading), plus `RawDefinitionValues` for the raw JSON, token registries,
//! and DefinitionWorld for additional categories.

use bevy::prelude::*;
use bevy_ecs::world::World;
use bevy_state::state_scoped::DespawnOnExit;
use cdda_components::input::BindableAction;
use cdda_context::ctx::Ctx;
use cdda_context::screen::CddaScreen;
use cdda_data::def_registry_resource::DefRegistryResource;
use cdda_data::def_world::DefinitionWorld;
use cdda_data::raw_values::RawDefinitionValues;
use serde_json::Value;

use crate::render::theme::{self, UiTheme};

// ===========================================================================
// Data types
// ===========================================================================

/// A single entry in a registry.
#[derive(Debug, Clone)]
struct RegistryEntry {
    id: String,
    /// Raw JSON text (from original file data).
    raw_json: String,
    /// Parsed Rust struct fields, formatted as field-by-field lines.
    parsed_fields: String,
    /// Human-readable round-trip / coverage status (see [`check_entry_consistency`]).
    status: String,
}

/// Unerroneously re-serialize a parsed def value and compare it to the source
/// raw JSON.
///
/// Produces a short status summary used by the registry viewer and the CLI
/// `consistency` command:
///
/// - **Internal round-trip**: the parsed struct is serialized to JSON
///   (`to_json`) and that JSON is deserialized back to a `Value` and serialized
///   again; if the second serialization equals the first, serialize ⇄ deserialize
///   is idempotent for this def. This isolates bugs in *our* serde impls from
///   differences introduced by `copy-from` / default-filling during loading.
/// - **Source coverage**: which keys present in the *actual file JSON* are
///   missing from the parsed struct's re-serialization, and which parsed keys
///   have no source counterpart. A non-empty `missing_from_parsed` means the
///   raw data is being silently dropped by our structs.
fn check_consistency(parsed: &Value, raw: &Value) -> String {
    let mut flags: Vec<String> = Vec::new();

    // 1) Idempotence of parse → serialize → parse → serialize.
    let se = serde_json::to_string(parsed).unwrap_or_default();
    let re = serde_json::from_str::<Value>(&se)
        .ok()
        .and_then(|v| serde_json::to_string(&v).ok());
    if re.as_deref() == Some(se.as_str()) {
        flags.push("round-trip ok".to_string());
    } else {
        flags.push("ROUND-TRIP MISMATCH".to_string());
    }

    // 2) Source coverage: fields in raw JSON missing from parsed.
    let raw_obj = raw.as_object();
    let parsed_obj = parsed.as_object();
    if let (Some(raw_map), Some(parsed_map)) = (raw_obj, parsed_obj) {
        let missing = raw_map
            .keys()
            .filter(|k| !parsed_map.contains_key(*k))
            .count();
        let extra = parsed_map
            .keys()
            .filter(|k| !raw_map.contains_key(*k))
            .count();
        flags.push(format!("missing {missing} key(s)"));
        flags.push(format!("extra {extra} key(s)"));
        if missing > 0 {
            flags.push("⚠ DROPPED-FIELDS".to_string());
        }
    }

    flags.join(" · ")
}

/// One registry category in the left panel.
#[derive(Debug, Clone)]
pub(crate) struct RegistryCategoryData {
    name: String,
    count: usize,
}

// ===========================================================================
// Helper: format parsed struct fields
// ===========================================================================

/// Format a `serde_json::Value` as a field-by-field string (one line per key).
fn format_parsed_fields(value: &Value) -> String {
    match value {
        Value::Object(map) => {
            let mut lines: Vec<String> = Vec::with_capacity(map.len());
            for (key, val) in map {
                let val_str = match val {
                    Value::Null => "null".to_string(),
                    Value::Bool(b) => b.to_string(),
                    Value::Number(n) => n.to_string(),
                    Value::String(s) => {
                        if s.len() > 120 {
                            let truncated: String = s.chars().take(120).collect();
                            format!("\"{}…\"", truncated)
                        } else {
                            format!("\"{}\"", s)
                        }
                    }
                    Value::Array(arr) => {
                        let items: Vec<String> = arr
                            .iter()
                            .map(|v| match v {
                                Value::String(s) => s.clone(),
                                other => format!("{}", other),
                            })
                            .collect();
                        if items.len() > 20 {
                            let display: Vec<&str> = items.iter().map(|s| s.as_str()).collect();
                            format!("[{} … {} more]", display[..20].join(", "), items.len() - 20)
                        } else {
                            format!("[{}]", items.join(", "))
                        }
                    }
                    Value::Object(_) => "{ ... }".to_string(),
                };
                lines.push(format!("{}: {}", key, val_str));
            }
            lines.join("\n")
        }
        _ => format!("{}", value),
    }
}

// ===========================================================================
// Helper: map display category name to raw JSON type name
// ===========================================================================

/// Given a DefRegistry field name (e.g. "items") or category display name
/// (e.g. "Items"), return the type key used in `RawDefinitionValues`.
fn raw_type_key_for_field(field_name: &str) -> &str {
    match field_name {
        "items" => "ITEM",
        "monsters" => "MONSTER",
        "terrain" => "terrain",
        "furniture" => "furniture",
        "recipes" => "recipe",
        "item_groups" => "item_group",
        "mapgen" => "mapgen",
        "nested_mapgen" => "nested_mapgen",
        "palettes" => "palette",
        "overmap_terrains" => "overmap_terrain",
        "overmap_specials" => "overmap_special",
        "overmap_connections" => "overmap_connection",
        "overmap_locations" => "overmap_location",
        "overmap_land_use_codes" => "overmap_land_use_code",
        "fields" => "field_type",
        "vehicle_parts" => "vehicle_part",
        "vehicle_part_locations" => "vehicle_part_location",
        "vehicle_part_categories" => "vehicle_part_category",
        "mutations" => "mutation",
        "mutation_categories" => "mutation_category",
        "trait_groups" => "trait_group",
        "bionics" => "bionic",
        "effects" => "effect_type",
        "factions" => "faction",
        "scenarios" => "scenario",
        "materials" => "material",
        "skills" => "skill",
        "traps" => "trap",
        "start_locations" => "start_location",
        "json_flags" => "json_flag",
        "ascii_art" => "ascii_art",
        "construction_groups" => "construction_group",
        "item_actions" => "item_action",
        "techniques" => "technique",
        "ammunition_types" => "ammunition_type",
        "morale_types" => "morale_type",
        "scent_types" => "scent_type",
        "movement_modes" => "movement_mode",
        "mood_faces" => "mood_face",
        "achievements" => "achievement",
        "body_parts" => "body_part",
        "dreams" => "dream",
        "emits" => "emit",
        "event_statistics" => "event_statistic",
        "harvests" => "harvest",
        "item_migrations" => "MIGRATION",
        "monster_groups" => "monstergroup",
        "mutation_types" => "mutation_type",
        "nested_categories" => "nested_category",
        "practices" => "practice",
        "professions" => "profession",
        "proficiencies" => "proficiency",
        "scores" => "score",
        "species" => "SPECIES",
        "sub_body_parts" => "sub_body_part",
        "uncrafts" => "uncraft",
        "vitamins" => "vitamin",
        "talk_topics" => "talk_topic",
        "widgets" => "widget",
        "effects_on_condition" => "effect_on_condition",
        "constructions" => "construction",
        "snippets" => "snippet",
        "npcs" => "npc",
        "npc_classes" => "npc_class",
        "requirements" => "requirement",
        "spells" => "SPELL",
        "vehicles" => "vehicle",
        "city_buildings" => "city_building",
        "mission_definitions" => "mission_definition",
        "event_transformations" => "event_transformation",
        "martial_arts" => "martial_art",
        "monster_attacks" => "monster_attack",
        "weakpoint_sets" => "weakpoint_set",
        "recipe_groups" => "recipe_group",
        "monster_flags" => "monster_flag",
        "activity_types" => "activity_type",
        "ammo_effects" => "ammo_effect",
        "tool_qualities" => "tool_quality",
        "faults" => "fault",
        "map_extras" => "map_extra",
        "fault_fixes" => "fault_fix",
        "ter_furn_transforms" => "ter_furn_transform",
        "connect_groups" => "connect_group",
        "attack_vectors" => "attack_vector",
        "region_terrain_furnitures" => "region_terrain_furniture",
        "item_categories" => "ITEM_CATEGORY",
        "oter_visions" => "oter_vision",
        "profession_item_substitutions" => "profession_item_substitutions",
        "character_mods" => "character_mod",
        "weapon_categories" => "weapon_category",
        "rotatable_symbols" => "rotatable_symbol",
        "oter_id_migrations" => "oter_id_migration",
        "climbing_aids" => "climbing_aid",
        "conducts" => "conduct",
        "weather_types" => "weather_type",
        "proficiency_categories" => "proficiency_category",
        "faction_missions" => "faction_mission",
        "fault_groups" => "fault_group",
        "jmath_functions" => "jmath_function",
        "body_graphs" => "body_graph",
        "limb_scores" => "limb_score",
        "construction_categories" => "construction_category",
        "recipe_categories" => "recipe_category",
        "addiction_types" => "addiction_type",
        "region_settings" => "region_settings",
        "gates" => "gate",
        "damage_types" => "damage_type",
        "anatomies" => "anatomy",
        "end_screens" => "end_screen",
        // Fallback: use as-is (lowercase)
        other => other,
    }
}

// ===========================================================================
// Registry collection: all fields from DefRegistry
// ===========================================================================

/// Collect ALL categories from the `DefRegistryResource`.
///
/// Each field in `DefRegistry` becomes one category.  The `mapgen` field
/// is `HashMap<String, Vec<Arc<MapgenDef>>>` and needs special handling;
/// all others are `HashMap<DefId<T>, Arc<T>>` and use the standard macro.
fn collect_all_from_def_registry(world: &World) -> Vec<(RegistryCategoryData, Vec<RegistryEntry>)> {
    let Some(reg) = world.get_resource::<DefRegistryResource>() else {
        return Vec::new();
    };
    let reg = &reg.0;
    let raw_values = world.get_resource::<RawDefinitionValues>();

    // Standard macro for HashMap<DefId<T>, Arc<T>> fields.
    macro_rules! def_category {
        ($field:ident, $name:expr) => {{
            let raw_type_key = raw_type_key_for_field(stringify!($field));
            let mut entries: Vec<RegistryEntry> = reg
                .$field
                .iter()
                .map(|(id, val)| {
                    // Raw JSON from RawDefinitionValues
                    let raw_json = raw_values
                        .and_then(|rv| rv.get_raw(raw_type_key, id.as_str()))
                        .map(|v| serde_json::to_string_pretty(v).unwrap_or_default())
                        .unwrap_or_else(|| "".to_string());

                    // Parsed struct fields (re-serialized from the Rust value)
                    let parsed_value = serde_json::to_value(val.as_ref()).unwrap_or_default();
                    let parsed_fields = format_parsed_fields(&parsed_value);

                    // Round-trip / coverage check against the actual source JSON.
                    let status =
                        match raw_values.and_then(|rv| rv.get_raw(raw_type_key, id.as_str())) {
                            Some(raw) => check_consistency(&parsed_value, raw),
                            None => "no source JSON".to_string(),
                        };

                    RegistryEntry {
                        id: id.as_str().to_string(),
                        raw_json,
                        parsed_fields,
                        status,
                    }
                })
                .collect();
            entries.sort_by(|a, b| a.id.cmp(&b.id));
            (
                RegistryCategoryData {
                    name: $name.to_string(),
                    count: entries.len(),
                },
                entries,
            )
        }};
    }

    // mapgen is HashMap<String, Vec<Arc<MapgenDef>>> — serialize each def
    // manually since Arc<T> without serde `rc` feature doesn't implement Serialize.
    macro_rules! def_category_mapgen {
        ($field:ident, $name:expr) => {{
            let raw_type_key = raw_type_key_for_field(stringify!($field));
            let mut entries: Vec<RegistryEntry> = reg
                .$field
                .iter()
                .map(|(id, val)| {
                    // Raw JSON from RawDefinitionValues
                    let raw_json = raw_values
                        .and_then(|rv| rv.get_raw(raw_type_key, id))
                        .map(|v| serde_json::to_string_pretty(v).unwrap_or_default())
                        .unwrap_or_else(|| "".to_string());

                    // Parsed struct fields (manual construction)
                    let items_json: Vec<String> = val
                        .iter()
                        .map(|v| serde_json::to_string_pretty(v.as_ref()).unwrap_or_default())
                        .collect();
                    let parsed_fields = format!("[\n{}\n]", items_json.join(",\n"));

                    // Round-trip status for the mapgen entry.
                    let raw_map = raw_values.and_then(|rv| rv.get_raw(raw_type_key, id));
                    let status = match raw_map {
                        Some(_) => "mapgen present".to_string(),
                        None => "no source JSON".to_string(),
                    };

                    RegistryEntry {
                        id: id.to_string(),
                        raw_json,
                        parsed_fields,
                        status,
                    }
                })
                .collect();
            entries.sort_by(|a, b| a.id.cmp(&b.id));
            (
                RegistryCategoryData {
                    name: $name.to_string(),
                    count: entries.len(),
                },
                entries,
            )
        }};
    }

    vec![
        def_category!(items, "Items"),
        def_category!(monsters, "Monsters"),
        def_category!(terrain, "Terrain"),
        def_category!(furniture, "Furniture"),
        def_category!(recipes, "Recipes"),
        def_category!(item_groups, "Item Groups"),
        def_category_mapgen!(mapgen, "Mapgen"),
        def_category!(nested_mapgen, "Nested Mapgen"),
        def_category!(palettes, "Palettes"),
        def_category!(overmap_terrains, "Overmap Terrains"),
        def_category!(overmap_specials, "Overmap Specials"),
        def_category!(overmap_connections, "Overmap Connections"),
        def_category!(overmap_locations, "Overmap Locations"),
        def_category!(overmap_land_use_codes, "Overmap Land Use Codes"),
        def_category!(fields, "Fields"),
        def_category!(vehicle_parts, "Vehicle Parts"),
        def_category!(vehicle_part_locations, "Vehicle Part Locations"),
        def_category!(vehicle_part_categories, "Vehicle Part Categories"),
        def_category!(mutations, "Mutations"),
        def_category!(mutation_categories, "Mutation Categories"),
        def_category!(trait_groups, "Trait Groups"),
        def_category!(bionics, "Bionics"),
        def_category!(effects, "Effects"),
        def_category!(factions, "Factions"),
        def_category!(scenarios, "Scenarios"),
        def_category!(materials, "Materials"),
        def_category!(skills, "Skills"),
        def_category!(traps, "Traps"),
        def_category!(start_locations, "Start Locations"),
        def_category!(json_flags, "JSON Flags"),
        def_category!(ascii_art, "ASCII Art"),
        def_category!(construction_groups, "Construction Groups"),
        def_category!(item_actions, "Item Actions"),
        def_category!(techniques, "Techniques"),
        def_category!(ammunition_types, "Ammunition Types"),
        def_category!(morale_types, "Morale Types"),
        def_category!(scent_types, "Scent Types"),
        def_category!(movement_modes, "Movement Modes"),
        def_category!(mood_faces, "Mood Faces"),
        def_category!(achievements, "Achievements"),
        def_category!(body_parts, "Body Parts"),
        def_category!(dreams, "Dreams"),
        def_category!(emits, "Emits"),
        def_category!(event_statistics, "Event Statistics"),
        def_category!(harvests, "Harvests"),
        def_category!(item_migrations, "Item Migrations"),
        def_category!(monster_groups, "Monster Groups"),
        def_category!(mutation_types, "Mutation Types"),
        def_category!(nested_categories, "Nested Categories"),
        def_category!(practices, "Practices"),
        def_category!(professions, "Professions"),
        def_category!(proficiencies, "Proficiencies"),
        def_category!(scores, "Scores"),
        def_category!(species, "Species"),
        def_category!(sub_body_parts, "Sub Body Parts"),
        def_category!(uncrafts, "Uncrafts"),
        def_category!(vitamins, "Vitamins"),
        def_category!(talk_topics, "Talk Topics"),
        def_category!(widgets, "Widgets"),
        def_category!(effects_on_condition, "Effects on Condition"),
        def_category!(constructions, "Constructions"),
        def_category!(snippets, "Snippets"),
        def_category!(npcs, "NPCs"),
        def_category!(npc_classes, "NPC Classes"),
        def_category!(requirements, "Requirements"),
        def_category!(spells, "Spells"),
        def_category!(vehicles, "Vehicles"),
        def_category!(city_buildings, "City Buildings"),
        def_category!(mission_definitions, "Mission Definitions"),
        def_category!(event_transformations, "Event Transformations"),
        def_category!(martial_arts, "Martial Arts"),
        def_category!(monster_attacks, "Monster Attacks"),
        def_category!(weakpoint_sets, "Weakpoint Sets"),
        def_category!(recipe_groups, "Recipe Groups"),
        def_category!(monster_flags, "Monster Flags"),
        def_category!(activity_types, "Activity Types"),
        def_category!(ammo_effects, "Ammo Effects"),
        def_category!(tool_qualities, "Tool Qualities"),
        def_category!(faults, "Faults"),
        def_category!(map_extras, "Map Extras"),
        def_category!(fault_fixes, "Fault Fixes"),
        def_category!(ter_furn_transforms, "Ter/Furn Transforms"),
        def_category!(connect_groups, "Connect Groups"),
        def_category!(attack_vectors, "Attack Vectors"),
        def_category!(region_terrain_furnitures, "Region Terrain Furnitures"),
        def_category!(item_categories, "Item Categories"),
        def_category!(oter_visions, "Oter Visions"),
        def_category!(
            profession_item_substitutions,
            "Profession Item Substitutions"
        ),
        def_category!(character_mods, "Character Mods"),
        def_category!(weapon_categories, "Weapon Categories"),
        def_category!(rotatable_symbols, "Rotatable Symbols"),
        def_category!(oter_id_migrations, "Oter ID Migrations"),
        def_category!(climbing_aids, "Climbing Aids"),
        def_category!(conducts, "Conducts"),
        def_category!(weather_types, "Weather Types"),
        def_category!(proficiency_categories, "Proficiency Categories"),
        def_category!(faction_missions, "Faction Missions"),
        def_category!(fault_groups, "Fault Groups"),
        def_category!(jmath_functions, "JMath Functions"),
        def_category!(body_graphs, "Body Graphs"),
        def_category!(limb_scores, "Limb Scores"),
        def_category!(construction_categories, "Construction Categories"),
        def_category!(recipe_categories, "Recipe Categories"),
        def_category!(addiction_types, "Addiction Types"),
        def_category!(region_settings, "Region Settings"),
        def_category!(gates, "Gates"),
        def_category!(damage_types, "Damage Types"),
        def_category!(anatomies, "Anatomies"),
        def_category!(end_screens, "End Screens"),
    ]
}

// ===========================================================================
// Token registry categories (appended after DefRegistry categories)
// ===========================================================================

fn collect_token_categories(world: &World) -> Vec<(RegistryCategoryData, Vec<RegistryEntry>)> {
    let mut cats: Vec<(RegistryCategoryData, Vec<RegistryEntry>)> = Vec::new();

    // Skills
    let skills_entries: Vec<RegistryEntry> = world
        .get_resource::<cdda_data::interner::SkillRegistry>()
        .map(|r| {
            let mut v: Vec<RegistryEntry> = r
                .iter()
                .map(|(n, id)| RegistryEntry {
                    id: n.to_string(),
                    raw_json: String::new(),
                    parsed_fields: format!("SkillId({})", id.0),
                    status: "derived (token)".to_string(),
                })
                .collect();
            v.sort_by(|a, b| a.id.cmp(&b.id));
            v
        })
        .unwrap_or_default();
    cats.push((
        RegistryCategoryData {
            name: "Skills (Token)".to_string(),
            count: skills_entries.len(),
        },
        skills_entries,
    ));

    // Ammo Types
    let ammo_entries: Vec<RegistryEntry> = world
        .get_resource::<cdda_data::interner::AmmoTypeRegistry>()
        .map(|r| {
            let mut v: Vec<RegistryEntry> = r
                .iter()
                .map(|(n, id)| RegistryEntry {
                    id: n.to_string(),
                    raw_json: String::new(),
                    parsed_fields: format!("AmmoTypeId({})", id.0),
                    status: "derived (token)".to_string(),
                })
                .collect();
            v.sort_by(|a, b| a.id.cmp(&b.id));
            v
        })
        .unwrap_or_default();
    cats.push((
        RegistryCategoryData {
            name: "Ammo Types (Token)".to_string(),
            count: ammo_entries.len(),
        },
        ammo_entries,
    ));

    // Body Parts
    let bp_entries: Vec<RegistryEntry> = world
        .get_resource::<cdda_data::interner::BodyPartRegistry>()
        .map(|r| {
            let mut v: Vec<RegistryEntry> = r
                .iter()
                .map(|(n, id)| RegistryEntry {
                    id: n.to_string(),
                    raw_json: String::new(),
                    parsed_fields: format!("BodyPartId({})", id.0),
                    status: "derived (token)".to_string(),
                })
                .collect();
            v.sort_by(|a, b| a.id.cmp(&b.id));
            v
        })
        .unwrap_or_default();
    cats.push((
        RegistryCategoryData {
            name: "Body Parts (Token)".to_string(),
            count: bp_entries.len(),
        },
        bp_entries,
    ));

    // Comestibles
    let com_entries: Vec<RegistryEntry> = world
        .get_resource::<cdda_data::interner::ComestibleRegistry>()
        .map(|r| {
            let mut v: Vec<RegistryEntry> = r
                .iter()
                .map(|(n, id)| RegistryEntry {
                    id: n.to_string(),
                    raw_json: String::new(),
                    parsed_fields: format!("ComestibleId({})", id.0),
                    status: "derived (token)".to_string(),
                })
                .collect();
            v.sort_by(|a, b| a.id.cmp(&b.id));
            v
        })
        .unwrap_or_default();
    cats.push((
        RegistryCategoryData {
            name: "Comestibles (Token)".to_string(),
            count: com_entries.len(),
        },
        com_entries,
    ));

    // Qualities
    let qual_entries: Vec<RegistryEntry> = world
        .get_resource::<cdda_data::interner::QualityRegistry>()
        .map(|r| {
            let mut v: Vec<RegistryEntry> = r
                .iter()
                .map(|(n, id)| RegistryEntry {
                    id: n.to_string(),
                    raw_json: String::new(),
                    parsed_fields: format!("QualityId({})", id.0),
                    status: "derived (token)".to_string(),
                })
                .collect();
            v.sort_by(|a, b| a.id.cmp(&b.id));
            v
        })
        .unwrap_or_default();
    cats.push((
        RegistryCategoryData {
            name: "Qualities (Token)".to_string(),
            count: qual_entries.len(),
        },
        qual_entries,
    ));

    // Item Types
    let it_entries: Vec<RegistryEntry> = world
        .get_resource::<cdda_data::interner::ItemTypeRegistry>()
        .map(|r| {
            let mut v: Vec<RegistryEntry> = r
                .iter()
                .map(|(n, id)| RegistryEntry {
                    id: n.to_string(),
                    raw_json: String::new(),
                    parsed_fields: format!("ItemTypeId({})", id.0),
                    status: "derived (token)".to_string(),
                })
                .collect();
            v.sort_by(|a, b| a.id.cmp(&b.id));
            v
        })
        .unwrap_or_default();
    cats.push((
        RegistryCategoryData {
            name: "Item Types (Token)".to_string(),
            count: it_entries.len(),
        },
        it_entries,
    ));

    cats
}

// ===========================================================================
// DefinitionWorld category (appended last)
// ===========================================================================

fn collect_def_world_entries(world: &World) -> (RegistryCategoryData, Vec<RegistryEntry>) {
    use cdda_components::def::*;
    let Some(def_world) = world.get_resource::<DefinitionWorld>() else {
        return (
            RegistryCategoryData {
                name: "Definition World".to_string(),
                count: 0,
            },
            Vec::new(),
        );
    };
    let mut entries: Vec<RegistryEntry> = def_world
        .iter()
        .map(|(category, id, entity)| {
            let id = format!("{category:?}::{id}");

            let entity_ref = world.entity(entity);
            let type_info = if entity_ref.contains::<WeaponData>() {
                "weapon"
            } else if entity_ref.contains::<AmmoData>() {
                "ammo"
            } else if entity_ref.contains::<GunData>() {
                "gun"
            } else if entity_ref.contains::<ArmourData>() {
                "armor"
            } else if entity_ref.contains::<FoodData>() {
                "food"
            } else if entity_ref.contains::<ToolData>() {
                "tool"
            } else if entity_ref.contains::<BookData>() {
                "book"
            } else if entity_ref.contains::<MagazineData>() {
                "magazine"
            } else if entity_ref.contains::<GunModData>() {
                "gunmod"
            } else if entity_ref.contains::<ContainerData>() {
                "container"
            } else if entity_ref.contains::<MonsterStats>() {
                "monster"
            } else if entity_ref.contains::<TerrainMoveCost>() {
                "terrain"
            } else if entity_ref.contains::<FurnitureMoveCostMod>() {
                "furniture"
            } else if entity_ref.contains::<IsRecipeDef>() {
                "recipe"
            } else if entity_ref.contains::<BodyPartDefId>() {
                "body_part"
            } else {
                "unknown"
            };
            RegistryEntry {
                id: id.to_string(),
                raw_json: String::new(),
                parsed_fields: format!("Type: {}\nEntity: {:?}", type_info, entity),
                status: "def-world entity".to_string(),
            }
        })
        .collect();
    entries.sort_by(|a, b| a.id.cmp(&b.id));
    (
        RegistryCategoryData {
            name: "Definition World".to_string(),
            count: entries.len(),
        },
        entries,
    )
}

// ===========================================================================
// Refresh — rebuild categories + entries from World
// ===========================================================================

fn refresh_categories(world: &World) -> (Vec<RegistryCategoryData>, Vec<Vec<RegistryEntry>>) {
    let mut cats: Vec<RegistryCategoryData> = Vec::new();
    let mut all_entries: Vec<Vec<RegistryEntry>> = Vec::new();

    // 1. DefRegistry categories
    for (cat, entries) in collect_all_from_def_registry(world) {
        cats.push(cat);
        all_entries.push(entries);
    }

    // 2. Token registries
    for (cat, entries) in collect_token_categories(world) {
        cats.push(cat);
        all_entries.push(entries);
    }

    // 3. Definition World (optional, appended last)
    let (cat, entries) = collect_def_world_entries(world);
    cats.push(cat);
    all_entries.push(entries);

    (cats, all_entries)
}

// ===========================================================================
// State
// ===========================================================================

#[derive(Resource, Default, Clone)]
pub(crate) struct RegistryViewerState {
    pub category_index: usize,
    pub entry_index: usize,
    pub categories: Vec<RegistryCategoryData>,
    pub all_entries: Vec<Vec<RegistryEntry>>,
    /// Which pane has keyboard focus: 0 = categories (left),
    /// 1 = entries (middle), 2 = raw JSON (detail left), 3 = parsed fields
    /// (detail right). Tab cycles; arrows navigate *within* the focused pane.
    pub pane: usize,
}

impl RegistryViewerState {
    pub const PANE_CATEGORY: usize = 0;
    pub const PANE_ENTRY: usize = 1;
    pub const PANE_RAW: usize = 2;
    pub const PANE_PARSED: usize = 3;
    pub const PANE_COUNT: usize = 4;

    fn refresh(&mut self, world: &World) {
        let (cats, all_entries) = refresh_categories(world);
        self.categories = cats;
        self.all_entries = all_entries;

        // Clamp indices
        if !self.categories.is_empty() {
            self.category_index = self.category_index.min(self.categories.len() - 1);
        } else {
            self.category_index = 0;
        }
        let entries = self
            .all_entries
            .get(self.category_index)
            .map(|e| e.len())
            .unwrap_or(0);
        self.entry_index = self.entry_index.min(entries.saturating_sub(1));
    }
}

// ===========================================================================
// Marker components
// ===========================================================================

#[derive(Component)]
struct RegTitleBar;

#[derive(Component)]
struct RegCategoryPanel;

#[derive(Component)]
struct RegEntryListPanel;

#[derive(Component)]
struct RegDetailPanel;

#[derive(Component)]
struct RegRawJsonPanel;

#[derive(Component)]
struct RegParsedFieldsPanel;

// ===========================================================================
// CddaScreen
// ===========================================================================

pub struct RegistryScreen;

impl CddaScreen for RegistryScreen {
    const CTX: Ctx = Ctx::RegistryViewer;
    const ACTIONS: &'static [(&'static str, BindableAction)] = &[
        ("up", BindableAction::NavigateUp),
        ("down", BindableAction::NavigateDown),
    ];

    fn spawn(world: &mut World) {
        spawn_registry_viewer(world);
    }

    fn update(world: &mut World) {
        update_registry_viewer(world);
    }
}

// ===========================================================================
// Spawn
// ===========================================================================

pub fn spawn_registry_viewer(world: &mut World) {
    let mut state = RegistryViewerState::default();
    state.refresh(world);
    world.insert_resource(state);

    world
        .commands()
        .spawn((
            DespawnOnExit(Ctx::RegistryViewer),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(theme::BG),
        ))
        .with_children(|root| {
            root.spawn((
                RegTitleBar,
                Node {
                    width: Val::Percent(100.0),
                    padding: UiRect::axes(Val::Px(20.0), Val::Px(10.0)),
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(theme::HEADER_BG),
            ));

            root.spawn((Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                min_height: Val::Px(0.0),
                flex_direction: FlexDirection::Row,
                overflow: Overflow::clip(),
                ..default()
            },))
                .with_children(|body| {
                    // LEFT: Category panel
                    body.spawn((
                        RegCategoryPanel,
                        crate::render::scroll::KeyboardScroll,
                        crate::render::scroll::VirtualList {
                            row_height: 30.0,
                            ..default()
                        },
                        crate::render::scroll::FocusedRow::default(),
                        ScrollPosition::default(),
                        Node {
                            width: Val::Percent(20.0),
                            min_width: Val::Px(140.0),
                            flex_shrink: 0.0,
                            flex_grow: 0.0,
                            flex_direction: FlexDirection::Column,
                            overflow: Overflow::scroll_y(),
                            border: UiRect::right(Val::Px(1.0)),
                            ..default()
                        },
                        BackgroundColor(theme::PANEL_BG),
                        BorderColor::all(theme::DIVIDER),
                    ));
                    // MIDDLE: Entry list
                    body.spawn((
                        RegEntryListPanel,
                        crate::render::scroll::KeyboardScroll,
                        crate::render::scroll::VirtualList {
                            row_height: 26.0,
                            ..default()
                        },
                        crate::render::scroll::FocusedRow::default(),
                        ScrollPosition::default(),
                        Node {
                            width: Val::Percent(20.0),
                            min_width: Val::Px(140.0),
                            flex_shrink: 0.0,
                            flex_grow: 0.0,
                            flex_direction: FlexDirection::Column,
                            overflow: Overflow::scroll_y(),
                            border: UiRect::right(Val::Px(1.0)),
                            ..default()
                        },
                        BackgroundColor(theme::PANEL_BG),
                        BorderColor::all(theme::DIVIDER),
                    ));
                    // RIGHT: Detail panel (split into raw JSON + parsed fields)
                    body.spawn((
                        RegDetailPanel,
                        Node {
                            flex_grow: 1.0,
                            min_height: Val::Px(0.0),
                            flex_direction: FlexDirection::Row,
                            overflow: Overflow::clip(),
                            ..default()
                        },
                        BackgroundColor(theme::PANEL_BG),
                    ))
                    .with_children(|detail| {
                        // LEFT HALF: Raw JSON
                        detail.spawn((
                            RegRawJsonPanel,
                            crate::render::scroll::KeyboardScroll,
                            ScrollPosition::default(),
                            Node {
                                width: Val::Percent(50.0),
                                min_width: Val::Px(200.0),
                                flex_grow: 0.0,
                                flex_shrink: 0.0,
                                flex_direction: FlexDirection::Column,
                                padding: UiRect::all(Val::Px(8.0)),
                                overflow: Overflow::scroll_y(),
                                border: UiRect::right(Val::Px(1.0)),
                                ..default()
                            },
                            BackgroundColor(theme::BG),
                            BorderColor::all(theme::DIVIDER),
                        ));
                        // RIGHT HALF: Parsed fields
                        detail.spawn((
                            RegParsedFieldsPanel,
                            crate::render::scroll::KeyboardScroll,
                            ScrollPosition::default(),
                            Node {
                                flex_grow: 1.0,
                                min_width: Val::Px(200.0),
                                flex_direction: FlexDirection::Column,
                                padding: UiRect::all(Val::Px(8.0)),
                                overflow: Overflow::scroll_y(),
                                ..default()
                            },
                            BackgroundColor(theme::BG),
                        ));
                    });
                });

            root.spawn((
                Node {
                    width: Val::Percent(100.0),
                    padding: UiRect::axes(Val::Px(20.0), Val::Px(8.0)),
                    border: UiRect::top(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(theme::HEADER_BG),
                BorderColor::all(theme::DIVIDER),
            ))
            .with_child((
                Text::new(
                    "Tab: switch pane | → focus panes w/ PgUp/Dn/arrows | ←→: navigate | Esc close",
                ),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(theme::TEXT_DIM),
                crate::render::FooterHint,
            ));
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pane_ids_are_ordered() {
        assert_eq!(RegistryViewerState::PANE_CATEGORY, 0);
        assert_eq!(RegistryViewerState::PANE_ENTRY, 1);
        assert_eq!(RegistryViewerState::PANE_RAW, 2);
        assert_eq!(RegistryViewerState::PANE_PARSED, 3);
        assert_eq!(RegistryViewerState::PANE_COUNT, 4);
    }

    #[test]
    fn pane_names_report_each_pane() {
        assert_eq!(pane_name(RegistryViewerState::PANE_CATEGORY), "categories");
        assert_eq!(pane_name(RegistryViewerState::PANE_ENTRY), "entries");
        assert_eq!(pane_name(RegistryViewerState::PANE_RAW), "raw JSON");
        assert_eq!(pane_name(RegistryViewerState::PANE_PARSED), "parsed");
    }

    #[test]
    fn scroll_window_zero_is_identity() {
        let t = "a\nb\nc";
        assert_eq!(scroll_window(t, 0), t);
    }

    #[test]
    fn scroll_window_skips_lines() {
        let t = "a\nb\nc";
        assert_eq!(scroll_window(t, 1), "b\nc");
        assert_eq!(scroll_window(t, 2), "c");
    }

    #[test]
    fn scroll_window_clamps_to_end() {
        assert_eq!(scroll_window("only", 5), "");
    }
}

// ===========================================================================
// Update
// ===========================================================================

/// Return a line-window of `text` starting at `scroll` (in lines). Only used by
/// the inline tests; production paging now uses Bevy's native scroll.
#[cfg(test)]
fn scroll_window(text: &str, scroll: usize) -> String {
    if scroll == 0 {
        return text.to_string();
    }
    let lines: Vec<&str> = text.lines().collect();
    let start = scroll.min(lines.len());
    if start >= lines.len() {
        return String::new();
    }
    lines[start..].join("\n")
}

/// Apply a pane-focus tint to each of the four panel containers. Collects IDs
/// first so it can mutate the world while querying.
fn apply_pane_tint(world: &mut World, tint: &impl Fn(bool) -> Color) {
    let pane = world.resource::<RegistryViewerState>().pane;
    let jobs: [(fn(&mut World) -> Vec<Entity>, bool); 4] = [
        (
            |w| {
                w.query_filtered::<Entity, With<RegCategoryPanel>>()
                    .iter(w)
                    .collect()
            },
            pane == RegistryViewerState::PANE_CATEGORY,
        ),
        (
            |w| {
                w.query_filtered::<Entity, With<RegEntryListPanel>>()
                    .iter(w)
                    .collect()
            },
            pane == RegistryViewerState::PANE_ENTRY,
        ),
        (
            |w| {
                w.query_filtered::<Entity, With<RegRawJsonPanel>>()
                    .iter(w)
                    .collect()
            },
            pane == RegistryViewerState::PANE_RAW,
        ),
        (
            |w| {
                w.query_filtered::<Entity, With<RegParsedFieldsPanel>>()
                    .iter(w)
                    .collect()
            },
            pane == RegistryViewerState::PANE_PARSED,
        ),
    ];
    for (collect, active) in jobs {
        for e in collect(world) {
            world.entity_mut(e).insert(BackgroundColor(tint(active)));
            if active {
                world
                    .entity_mut(e)
                    .remove::<super::scroll::InactiveScrollPane>();
            } else {
                world
                    .entity_mut(e)
                    .insert(super::scroll::InactiveScrollPane);
            }
        }
    }
}

/// Human-readable label for a pane id (for the title-bar indicator).
fn pane_name(pane: usize) -> &'static str {
    match pane {
        RegistryViewerState::PANE_CATEGORY => "categories",
        RegistryViewerState::PANE_ENTRY => "entries",
        RegistryViewerState::PANE_RAW => "raw JSON",
        RegistryViewerState::PANE_PARSED => "parsed",
        _ => "?",
    }
}

/// For an index-navigated list pane: sync its `VirtualList` size + `FocusedRow`
/// to the focused index and return the visible row window `[start, end)`. The
/// pane's window is recomputed from its `ScrollPosition` each PostUpdate by
/// `scroll::update_virtual_windows`; this only feeds `total_rows` and sets the
/// focused row so the shared keep-visible scroll shows it.
fn virtual_list_for_panel(
    world: &mut World,
    panel: Entity,
    total_rows: usize,
    focused: usize,
) -> (usize, usize) {
    let mut q = world.query::<(
        &mut crate::render::scroll::VirtualList,
        &mut crate::render::scroll::FocusedRow,
    )>();
    let Ok((mut vl, mut fr)) = q.get_mut(world, panel) else {
        return (0, total_rows);
    };
    if vl.total_rows != total_rows {
        vl.total_rows = total_rows;
    }
    let window = vl.window;
    if fr.0 != focused {
        fr.0 = focused;
    }
    (
        window.0.min(total_rows),
        window.1.min(total_rows).max(window.0),
    )
}

/// Spawn a top/bottom spacer pair within a list pane so native scroll position
/// maps to the item index while only the window's rows are rendered.
fn spawn_virtual_spacers(
    cmd: &mut EntityWorldMut,
    start: usize,
    _end: usize,
    _total: usize,
    row_px: f32,
) {
    if start > 0 {
        cmd.with_children(|p| {
            p.spawn(Node {
                height: Val::Px(start as f32 * row_px),
                flex_shrink: 0.0,
                ..default()
            });
        });
    }
}

#[derive(Component, PartialEq)]
struct RenderedDetail(usize, usize, theme::ThemePreset);

pub fn update_registry_viewer(world: &mut World) {
    let theme = world.resource::<UiTheme>().clone();

    // Phase 1: Handle input
    let (up, down, left, right, tab, page_up, page_down) = {
        let keys = world.resource::<ButtonInput<KeyCode>>();
        (
            keys.just_pressed(KeyCode::ArrowUp),
            keys.just_pressed(KeyCode::ArrowDown),
            keys.just_pressed(KeyCode::ArrowLeft),
            keys.just_pressed(KeyCode::ArrowRight),
            keys.just_pressed(KeyCode::Tab),
            keys.just_pressed(KeyCode::PageUp),
            keys.just_pressed(KeyCode::PageDown),
        )
    };

    let mut changed = false;

    let shift_tab = {
        let keys = world.resource::<ButtonInput<KeyCode>>();
        keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight)
    };

    // --- Tab / Shift+Tab: cycle the focused pane -------------------------
    if tab {
        let mut state = world.resource_mut::<RegistryViewerState>();
        state.pane = if shift_tab {
            (state.pane + RegistryViewerState::PANE_COUNT - 1) % RegistryViewerState::PANE_COUNT
        } else {
            (state.pane + 1) % RegistryViewerState::PANE_COUNT
        };
        changed = true;
    }

    // Read current focus once so arrows route to the active pane.
    let active_pane = world.resource::<RegistryViewerState>().pane;

    match active_pane {
        // ── Categories pane: ← / → / ↑ / ↓ move category ────────────
        RegistryViewerState::PANE_CATEGORY => {
            if left || right || up || down {
                let dir: i32 = if left || up { -1 } else { 1 };
                let num_cats = world.resource::<RegistryViewerState>().categories.len();
                if num_cats > 0 {
                    let mut state = world.resource_mut::<RegistryViewerState>();
                    let new_cat =
                        ((state.category_index as i32 + dir).rem_euclid(num_cats as i32)) as usize;
                    state.category_index = new_cat;
                    state.entry_index = 0;
                    changed = true;
                }
            }
        }
        // ── Entries pane: ↑/↓/PgUp/PgDn move entry; ←/→ switch category ──
        RegistryViewerState::PANE_ENTRY => {
            if left || right {
                // Change category without leaving the entries pane.
                let dir: i32 = if left { -1 } else { 1 };
                let num_cats = world.resource::<RegistryViewerState>().categories.len();
                if num_cats > 0 {
                    let mut state = world.resource_mut::<RegistryViewerState>();
                    let new_cat =
                        ((state.category_index as i32 + dir).rem_euclid(num_cats as i32)) as usize;
                    state.category_index = new_cat;
                    state.entry_index = 0;
                    changed = true;
                }
            } else if up || down || page_up || page_down {
                let page = if page_up || page_down { 30 } else { 1 };
                let dir: i32 = if up || page_up { -1 } else { 1 };
                let mut state = world.resource_mut::<RegistryViewerState>();
                let entries_len = state
                    .all_entries
                    .get(state.category_index)
                    .map(|e| e.len())
                    .unwrap_or(0);
                if entries_len > 0 {
                    let delta = dir * page as i32;
                    let new_ent = (state.entry_index as i32 + delta)
                        .max(0)
                        .min((entries_len - 1) as i32) as usize;
                    if new_ent != state.entry_index {
                        state.entry_index = new_ent;
                        changed = true;
                    }
                }
            }
        }
        // ── Raw / parsed detail panes: ↑/↓/PgUp/PgDn scroll; ←/→ swap ──
        RegistryViewerState::PANE_RAW | RegistryViewerState::PANE_PARSED => {
            if left || right {
                let mut state = world.resource_mut::<RegistryViewerState>();
                state.pane = if state.pane == RegistryViewerState::PANE_RAW {
                    RegistryViewerState::PANE_PARSED
                } else {
                    RegistryViewerState::PANE_RAW
                };
                changed = true;
            }
        }
        _ => {}
    }

    // Refresh entries if category changed
    if changed {
        let _state = world.resource::<RegistryViewerState>();
    }

    let dirty = changed
        || world.is_resource_changed::<RegistryViewerState>()
        || world.is_resource_changed::<UiTheme>()
        || world
            .query_filtered::<Entity, Changed<crate::render::scroll::VirtualList>>()
            .iter(world)
            .next()
            .is_some();
    if !dirty {
        return;
    }

    // Copy only visible labels and the selected detail, never the whole catalog.
    let window = world
        .query_filtered::<&super::scroll::VirtualList, With<RegEntryListPanel>>()
        .iter(world)
        .next()
        .map_or((0, 0), |list| list.window);
    let (cat_idx, ent_idx, cats, current_entries, total_entries, selected_entry, active_pane) = {
        let s = world.resource::<RegistryViewerState>();
        let entries = s
            .all_entries
            .get(s.category_index)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let labels: Vec<_> = entries
            .iter()
            .enumerate()
            .skip(window.0)
            .take(window.1.saturating_sub(window.0))
            .map(|(i, e)| (i, e.id.clone()))
            .collect();
        (
            s.category_index,
            s.entry_index,
            s.categories.clone(),
            labels,
            entries.len(),
            entries.get(s.entry_index).cloned(),
            s.pane,
        )
    };

    // Pane-focus highlight: tint each panel container when it is the active
    // pane so the user can see where keyboard focus is. IDs are collected
    // first to avoid double-borrowing `world`.
    let tint = |active: bool| -> Color {
        if active {
            theme.accent().with_alpha(0.10)
        } else {
            Color::NONE
        }
    };
    apply_pane_tint(world, &tint);

    // ── Title ──────────────────────────────────────────────────────────────
    if let Some(title_e) = world
        .query_filtered::<Entity, With<RegTitleBar>>()
        .iter(world)
        .next()
    {
        let label = cats
            .get(cat_idx)
            .map(|c| format!("DEBUG: REGISTRY VIEWER — {} ({})", c.name, c.count))
            .unwrap_or_else(|| "DEBUG: REGISTRY VIEWER".to_string());
        world
            .entity_mut(title_e)
            .despawn_children()
            .with_children(|h| {
                h.spawn((
                    Text::new(label),
                    TextFont {
                        font_size: 20.0,
                        ..default()
                    },
                    TextColor(theme.accent()),
                ));
                h.spawn((
                    Text::new(format!(
                        "cat {}/{} · pane: {}",
                        cat_idx + 1,
                        cats.len(),
                        pane_name(active_pane)
                    )),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(theme::TEXT_DIM),
                ));
            });
    }

    // ── Category list (left) ──────────────────────────────────────────────
    if let Some(list_e) = world
        .query_filtered::<Entity, With<RegCategoryPanel>>()
        .iter(world)
        .next()
    {
        let total = cats.len();
        let (start, end) = virtual_list_for_panel(world, list_e, total, cat_idx);
        let mut list_cmd = world.entity_mut(list_e);
        list_cmd.despawn_children();
        spawn_virtual_spacers(&mut list_cmd, start, end, total, 30.0);
        list_cmd.with_children(|list| {
            for (i, cat) in cats.iter().enumerate().skip(start).take(end - start) {
                let selected = i == cat_idx;
                list.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(30.0),
                        flex_shrink: 0.0,
                        overflow: Overflow::clip(),
                        padding: UiRect::horizontal(Val::Px(10.0)),
                        ..default()
                    },
                    BackgroundColor(if selected {
                        theme.accent().with_alpha(0.3)
                    } else if i % 2 == 0 {
                        theme::PANEL_BG
                    } else {
                        theme::ROW_ALT_BG
                    }),
                ))
                .with_child((
                    Text::new(format!("{}  ({})", cat.name, cat.count)),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(if selected {
                        theme.accent()
                    } else {
                        theme::TEXT_BRIGHT
                    }),
                ));
            }
        });
        let remaining = total.saturating_sub(end);
        if remaining > 0 {
            list_cmd.with_children(|p| {
                p.spawn(Node {
                    height: Val::Px(remaining as f32 * 30.0),
                    flex_shrink: 0.0,
                    ..default()
                });
            });
        }
    }

    // ── Entry list (middle) ──────────────────────────────────────────────
    if let Some(list_e) = world
        .query_filtered::<Entity, With<RegEntryListPanel>>()
        .iter(world)
        .next()
    {
        let total = total_entries;
        let (start, end) = virtual_list_for_panel(world, list_e, total, ent_idx);
        let mut list_cmd = world.entity_mut(list_e);
        list_cmd.despawn_children();
        spawn_virtual_spacers(&mut list_cmd, start, end, total, 26.0);
        list_cmd.with_children(|list| {
            if total_entries == 0 {
                list.spawn(Node {
                    padding: UiRect::axes(Val::Px(12.0), Val::Px(10.0)),
                    ..default()
                })
                .with_child((
                    Text::new("No entries"),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(theme::TEXT_DIM),
                ));
                return;
            }
            for (i, label) in &current_entries {
                let i = *i;
                list.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(26.0),
                        flex_shrink: 0.0,
                        overflow: Overflow::clip(),
                        padding: UiRect::horizontal(Val::Px(10.0)),
                        ..default()
                    },
                    BackgroundColor(if i == ent_idx {
                        theme.accent().with_alpha(0.25)
                    } else if i % 2 == 0 {
                        theme::PANEL_BG
                    } else {
                        theme::ROW_ALT_BG
                    }),
                ))
                .with_child((
                    Text::new(label.clone()),
                    TextFont {
                        font_size: 12.0,
                        ..default()
                    },
                    TextColor(if i == ent_idx {
                        theme::TEXT_BRIGHT
                    } else {
                        theme::TEXT_DIM
                    }),
                ));
            }
        });
        let remaining = total.saturating_sub(end);
        if remaining > 0 {
            list_cmd.with_children(|p| {
                p.spawn(Node {
                    height: Val::Px(remaining as f32 * 26.0),
                    flex_shrink: 0.0,
                    ..default()
                });
            });
        }
    }

    // ── Detail panel (right) — two sub-panels ────────────────────────────
    let Some(entry) = selected_entry.as_ref() else {
        return;
    };

    let detail_key = RenderedDetail(cat_idx, ent_idx, theme.preset);
    if let Some(panel) = world
        .query_filtered::<Entity, With<RegRawJsonPanel>>()
        .iter(world)
        .next()
    {
        if world.get::<RenderedDetail>(panel) == Some(&detail_key)
            && !world.is_resource_changed::<RegistryViewerState>()
        {
            return;
        }
        world.entity_mut(panel).insert(detail_key);
    }

    // ── Raw JSON sub-panel (left half) ────────────────────────────────────
    if let Some(raw_e) = world
        .query_filtered::<Entity, With<RegRawJsonPanel>>()
        .iter(world)
        .next()
    {
        world
            .entity_mut(raw_e)
            .despawn_children()
            .with_children(|p| {
                // Header
                p.spawn((
                    Text::new("RAW JSON"),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(theme.accent2()),
                ));
                p.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(1.0),
                        margin: UiRect::vertical(Val::Px(4.0)),
                        ..default()
                    },
                    BackgroundColor(theme::TEXT_DIM.with_alpha(0.3)),
                ));
                // Raw JSON content — native scroll handles windowing now.
                let raw_text = if entry.raw_json.is_empty() {
                    "No raw JSON available\n(possibly saved with a different type key)".to_string()
                } else {
                    entry.raw_json.clone()
                };
                p.spawn(Node {
                    width: Val::Percent(100.0),
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    overflow: Overflow::scroll_y(),
                    ..default()
                })
                .with_child((
                    Text::new(raw_text),
                    TextFont {
                        font_size: 11.0,
                        ..default()
                    },
                    TextColor(theme::TEXT_BRIGHT),
                ));
            });
    }

    // ── Parsed fields sub-panel (right half) ──────────────────────────────
    if let Some(fields_e) = world
        .query_filtered::<Entity, With<RegParsedFieldsPanel>>()
        .iter(world)
        .next()
    {
        world
            .entity_mut(fields_e)
            .despawn_children()
            .with_children(|p| {
                // Header
                p.spawn((
                    Text::new("PARSED STRUCT FIELDS"),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(theme.accent2()),
                ));
                p.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(1.0),
                        margin: UiRect::vertical(Val::Px(4.0)),
                        ..default()
                    },
                    BackgroundColor(theme::TEXT_DIM.with_alpha(0.3)),
                ));
                // Round-trip / coverage status line
                p.spawn((
                    Text::new(format!("STATUS: {}", entry.status)),
                    TextFont {
                        font_size: 11.0,
                        ..default()
                    },
                    TextColor(if entry.status.contains("DROPPED") {
                        Color::srgb(1.0, 0.4, 0.3)
                    } else if entry.status.contains("round-trip ok") {
                        Color::srgb(0.4, 0.9, 0.4)
                    } else {
                        theme::TEXT_DIM
                    }),
                ));
                // Parsed fields content — native scroll handles windowing now.
                let fields_text = if entry.parsed_fields.is_empty() {
                    "No parsed fields available".to_string()
                } else {
                    entry.parsed_fields.clone()
                };
                p.spawn(Node {
                    width: Val::Percent(100.0),
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    overflow: Overflow::scroll_y(),
                    ..default()
                })
                .with_child((
                    Text::new(fields_text),
                    TextFont {
                        font_size: 11.0,
                        ..default()
                    },
                    TextColor(theme.label_color()),
                ));
            });
    }
}

#[cfg(test)]
mod presentation_tests {
    use super::super::scroll::*;
    use super::*;

    #[test]
    fn headless_registry_preserves_idle_entities_and_orders_spacers() {
        let mut app = App::new();
        app.init_resource::<UiTheme>()
            .init_resource::<ButtonInput<KeyCode>>();
        app.insert_resource(RegistryViewerState {
            all_entries: vec![(0..40_000)
                .map(|i| RegistryEntry {
                    id: format!("item_{i}"),
                    raw_json: "{}".into(),
                    parsed_fields: "fields".into(),
                    status: "ok".into(),
                })
                .collect()],
            ..default()
        });
        let panel = app
            .world_mut()
            .spawn((
                RegEntryListPanel,
                Node::default(),
                KeyboardScroll,
                FocusedRow(0),
                ScrollPosition::default(),
                VirtualList {
                    row_height: 26.0,
                    ..default()
                },
            ))
            .id();
        app.add_systems(Update, update_registry_viewer);
        app.add_systems(
            PostUpdate,
            (scroll_to_focused_row, update_virtual_windows).chain(),
        );
        for _ in 0..4 {
            app.update();
        }
        let children = app.world().get::<Children>(panel).unwrap().to_vec();
        for _ in 0..10 {
            app.update();
        }
        assert_eq!(
            app.world().get::<Children>(panel).unwrap().to_vec(),
            children
        );
        assert!(children.len() < 40);
        let last = *children.last().unwrap();
        assert_eq!(app.world().get::<Node>(last).unwrap().flex_shrink, 0.0);
        assert!(
            app.world().get::<Children>(last).is_none(),
            "bottom spacer must follow rows"
        );
        app.world_mut()
            .resource_mut::<RegistryViewerState>()
            .entry_index = 20_000;
        for _ in 0..3 {
            app.update();
        }
        let list = app.world().get::<VirtualList>(panel).unwrap();
        assert!(list.window.0 <= 20_000 && list.window.1 > 20_000);
        assert!(app.world().get::<Children>(panel).unwrap().len() < 40);
    }
}
