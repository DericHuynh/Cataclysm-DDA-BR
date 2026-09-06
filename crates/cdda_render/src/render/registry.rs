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
use cdda_context::ctx::Ctx;
use cdda_context::screen::CddaScreen;
use cdda_data::def_registry_resource::DefRegistryResource;
use cdda_data::def_world::DefinitionWorld;
use cdda_data::raw_values::RawDefinitionValues;
use cdda_input::vocabulary::BindableAction;
use cdda_ui::{
    sync_virtual_pane, FocusedRow, InactiveScrollPane, RetainedRows, RowCell, TextRow, VirtualList,
};
use serde_json::Value;

use crate::render::theme::{self, UiTheme};

// ===========================================================================
// Data types
// ===========================================================================

/// A single entry in a registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RegistryEntry {
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
#[derive(Debug, Clone, PartialEq, Eq)]
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

            let Ok(entity_ref) = world.get_entity(entity) else {
                return RegistryEntry {
                    id,
                    raw_json: String::new(),
                    parsed_fields: "Definition entity is no longer available".into(),
                    status: "stale definition index".into(),
                };
            };
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

#[derive(Resource, Default, Clone, Copy, PartialEq, Eq)]
pub struct RegistryViewerState {
    pub category_index: usize,
    pub entry_index: usize,
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
}

/// Source projection; selection and pane focus never invalidate this resource.
#[derive(Resource, Default, PartialEq, Eq)]
pub struct RegistryCatalog {
    categories: Vec<RegistryCategoryData>,
    all_entries: Vec<Vec<RegistryEntry>>,
}

/// Exclusive access is limited to projecting the heterogeneous import registries.
/// Active UI input/render systems consume the resulting typed resource.
pub fn refresh_registry_catalog(world: &mut World) {
    let state = world
        .get_resource::<RegistryViewerState>()
        .copied()
        .unwrap_or_default();
    let selected = world
        .get_resource::<RegistryCatalog>()
        .map(|model| {
            (
                model
                    .categories
                    .get(state.category_index)
                    .map(|c| c.name.clone()),
                model
                    .all_entries
                    .get(state.category_index)
                    .and_then(|v| v.get(state.entry_index))
                    .map(|e| e.id.clone()),
            )
        })
        .unwrap_or_default();
    let (categories, all_entries) = refresh_categories(world);
    let catalog = RegistryCatalog {
        categories,
        all_entries,
    };
    let category_index = selected
        .0
        .as_ref()
        .and_then(|name| catalog.categories.iter().position(|c| &c.name == name))
        .unwrap_or_else(|| {
            state
                .category_index
                .min(catalog.categories.len().saturating_sub(1))
        });
    let entries = catalog
        .all_entries
        .get(category_index)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let entry_index = selected
        .1
        .as_ref()
        .and_then(|id| entries.iter().position(|e| &e.id == id))
        .unwrap_or_else(|| state.entry_index.min(entries.len().saturating_sub(1)));
    let next = RegistryViewerState {
        category_index,
        entry_index,
        ..state
    };
    if let Some(mut current) = world.get_resource_mut::<RegistryCatalog>() {
        current.set_if_neq(catalog);
    } else {
        world.insert_resource(catalog);
    }
    if let Some(mut current) = world.get_resource_mut::<RegistryViewerState>() {
        current.set_if_neq(next);
    } else {
        world.insert_resource(next);
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

    // Typed Update systems are registered by CddaRenderPlugin.
}

// ===========================================================================
// Spawn
// ===========================================================================

pub fn spawn_registry_viewer(world: &mut World) {
    refresh_registry_catalog(world);

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
            theme::SurfacePaint(theme::Role::Canvas),
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
                theme::SurfacePaint(theme::Role::Raised),
            ))
            .with_children(|header| {
                header.spawn((
                    RegistryHeading,
                    Text::default(),
                    TextFont {
                        font_size: 20.0,
                        ..default()
                    },
                    theme::TextPaint(theme::Role::Text),
                ));
                header.spawn((
                    RegistryCounter,
                    Text::default(),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                    theme::TextPaint(theme::Role::Muted),
                ));
            });

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
                        RegistryPane(RegistryViewerState::PANE_CATEGORY),
                        RetainedRows::<RegistryRowKey>::default(),
                        RegistryListState::default(),
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
                        theme::SurfacePaint(theme::Role::Surface),
                        theme::BorderPaint(theme::Role::Border),
                    ));
                    // MIDDLE: Entry list
                    body.spawn((
                        RegEntryListPanel,
                        RegistryPane(RegistryViewerState::PANE_ENTRY),
                        RetainedRows::<RegistryRowKey>::default(),
                        RegistryListState::default(),
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
                        theme::SurfacePaint(theme::Role::Surface),
                        theme::BorderPaint(theme::Role::Border),
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
                        theme::SurfacePaint(theme::Role::Surface),
                    ))
                    .with_children(|detail| {
                        spawn_detail_pane(detail, RegistryViewerState::PANE_RAW);
                        spawn_detail_pane(detail, RegistryViewerState::PANE_PARSED);
                    });
                });

            root.spawn((
                Node {
                    width: Val::Percent(100.0),
                    padding: UiRect::axes(Val::Px(20.0), Val::Px(8.0)),
                    border: UiRect::top(Val::Px(1.0)),
                    ..default()
                },
                theme::SurfacePaint(theme::Role::Raised),
                theme::BorderPaint(theme::Role::Border),
            ))
            .with_child((
                Text::new(
                    "Tab / Shift+Tab: pane | Arrows: navigate | PgUp/PgDn: page | Home/End: first/last | Esc: close",
                ),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                theme::TextPaint(theme::Role::Muted),
                crate::render::FooterHint,
            ));
        });
}

/// Detail labels are fixed siblings above their native scrolling content.
fn spawn_detail_pane(parent: &mut ChildSpawnerCommands, pane: usize) {
    parent
        .spawn((
            Node {
                width: Val::Percent(50.0),
                min_width: Val::Px(200.0),
                min_height: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                flex_basis: Val::Px(0.0),
                border: UiRect::right(Val::Px(1.0)),
                ..default()
            },
            theme::SurfacePaint(theme::Role::Canvas),
            theme::BorderPaint(theme::Role::Border),
        ))
        .with_children(|column| {
            column.spawn((
                RegistryDetailHeading(pane),
                Text::new(if pane == RegistryViewerState::PANE_RAW {
                    "RAW JSON"
                } else {
                    "PARSED STRUCT FIELDS"
                }),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                theme::TextPaint(theme::Role::Text),
                Node {
                    padding: UiRect::all(Val::Px(8.0)),
                    flex_shrink: 0.0,
                    ..default()
                },
            ));
            let mut body = column.spawn((
                RegistryPane(pane),
                crate::render::scroll::KeyboardScroll,
                ScrollPosition::default(),
                Node {
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    min_height: Val::Px(0.0),
                    padding: UiRect::all(Val::Px(8.0)),
                    overflow: Overflow::scroll_y(),
                    ..default()
                },
                theme::SurfacePaint(theme::Role::Canvas),
            ));
            if pane == RegistryViewerState::PANE_RAW {
                body.insert(RegRawJsonPanel);
            } else {
                body.insert(RegParsedFieldsPanel);
            }
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

/// Each pane owns its widget state. Input and presentation never acquire the World.
#[derive(Component)]
struct RegistryPane(usize);
#[derive(Component)]
struct RegistryHeading;
#[derive(Component)]
struct RegistryCounter;
#[derive(Component)]
struct RegistryDetailHeading(usize);
#[derive(Component, Default)]
struct RegistryListState {
    category: Option<String>,
}
#[derive(Component, PartialEq, Eq)]
struct RenderedDetail(u32, usize, usize, theme::ThemePreset);
type RegistryRowKey = (String, Option<String>);

/// Read-model refresh gate, including addition/removal of optional source resources.
#[derive(bevy_ecs::system::SystemParam)]
pub struct RegistrySources<'w> {
    defs: Option<Res<'w, DefRegistryResource>>,
    raw: Option<Res<'w, RawDefinitionValues>>,
    world: Option<Res<'w, DefinitionWorld>>,
    skills: Option<Res<'w, cdda_data::interner::SkillRegistry>>,
    ammo: Option<Res<'w, cdda_data::interner::AmmoTypeRegistry>>,
    body: Option<Res<'w, cdda_data::interner::BodyPartRegistry>>,
    food: Option<Res<'w, cdda_data::interner::ComestibleRegistry>>,
    qualities: Option<Res<'w, cdda_data::interner::QualityRegistry>>,
    items: Option<Res<'w, cdda_data::interner::ItemTypeRegistry>>,
}
pub fn registry_sources_changed(
    s: RegistrySources,
    mut seen: Local<Option<[Option<u32>; 9]>>,
) -> bool {
    let versions = [
        s.defs.as_ref().map(|r| r.last_changed().get()),
        s.raw.as_ref().map(|r| r.last_changed().get()),
        s.world.as_ref().map(|r| r.last_changed().get()),
        s.skills.as_ref().map(|r| r.last_changed().get()),
        s.ammo.as_ref().map(|r| r.last_changed().get()),
        s.body.as_ref().map(|r| r.last_changed().get()),
        s.food.as_ref().map(|r| r.last_changed().get()),
        s.qualities.as_ref().map(|r| r.last_changed().get()),
        s.items.as_ref().map(|r| r.last_changed().get()),
    ];
    if *seen == Some(versions) {
        return false;
    }
    *seen = Some(versions);
    true
}

pub fn registry_input(
    keys: Res<ButtonInput<KeyCode>>,
    catalog: Res<RegistryCatalog>,
    mut state: ResMut<RegistryViewerState>,
) {
    let mut next = *state;
    if keys.just_pressed(KeyCode::Tab) {
        let backwards = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
        next.pane = (next.pane
            + if backwards {
                RegistryViewerState::PANE_COUNT - 1
            } else {
                1
            })
            % RegistryViewerState::PANE_COUNT;
    }
    let up = keys.just_pressed(KeyCode::ArrowUp);
    let down = keys.just_pressed(KeyCode::ArrowDown);
    let left = keys.just_pressed(KeyCode::ArrowLeft);
    let right = keys.just_pressed(KeyCode::ArrowRight);
    let page_up = keys.just_pressed(KeyCode::PageUp);
    let page_down = keys.just_pressed(KeyCode::PageDown);
    let home = keys.just_pressed(KeyCode::Home);
    let end = keys.just_pressed(KeyCode::End);
    let category_move = next.pane == RegistryViewerState::PANE_CATEGORY
        && (up || down || left || right || page_up || page_down || home || end)
        || next.pane == RegistryViewerState::PANE_ENTRY && (left || right);
    if category_move && !catalog.categories.is_empty() {
        let count = catalog.categories.len();
        next.category_index = if home {
            0
        } else if end {
            count - 1
        } else if page_up {
            next.category_index.saturating_sub(10)
        } else if page_down {
            next.category_index.saturating_add(10).min(count - 1)
        } else if up || left {
            (next.category_index + count - 1) % count
        } else {
            (next.category_index + 1) % count
        };
        next.entry_index = 0;
    } else if next.pane == RegistryViewerState::PANE_ENTRY {
        let len = catalog
            .all_entries
            .get(next.category_index)
            .map_or(0, Vec::len);
        let step = if page_up || page_down { 30 } else { 1 };
        next.entry_index = if home {
            0
        } else if end {
            len.saturating_sub(1)
        } else if up || page_up {
            next.entry_index.saturating_sub(step)
        } else if down || page_down {
            next.entry_index
                .saturating_add(step)
                .min(len.saturating_sub(1))
        } else {
            next.entry_index
        };
    } else if (next.pane == RegistryViewerState::PANE_RAW
        || next.pane == RegistryViewerState::PANE_PARSED)
        && (left || right)
    {
        next.pane = if next.pane == RegistryViewerState::PANE_RAW {
            RegistryViewerState::PANE_PARSED
        } else {
            RegistryViewerState::PANE_RAW
        };
    }
    state.set_if_neq(next);
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

#[derive(bevy_ecs::system::SystemParam)]
pub struct RegistryPanels<'w, 's> {
    lists: Query<
        'w,
        's,
        (
            Entity,
            &'static RegistryPane,
            &'static mut VirtualList,
            &'static mut FocusedRow,
            &'static mut ScrollPosition,
            &'static ComputedNode,
            &'static mut RetainedRows<RegistryRowKey>,
            &'static mut RegistryListState,
        ),
    >,
    tint: Query<
        'w,
        's,
        (
            Entity,
            &'static RegistryPane,
            &'static mut theme::SurfacePaint,
            Option<&'static InactiveScrollPane>,
        ),
    >,
    title: Query<
        'w,
        's,
        (
            &'static mut Text,
            &'static mut theme::TextPaint,
            Option<&'static RegistryHeading>,
            Option<&'static RegistryDetailHeading>,
        ),
        Or<(
            With<RegistryHeading>,
            With<RegistryCounter>,
            With<RegistryDetailHeading>,
        )>,
    >,
    details: Query<
        'w,
        's,
        (
            Entity,
            &'static RegistryPane,
            Option<&'static RenderedDetail>,
            &'static mut ScrollPosition,
        ),
        Without<VirtualList>,
    >,
}

pub fn update_registry_viewer(
    mut commands: Commands,
    catalog: Res<RegistryCatalog>,
    state: Res<RegistryViewerState>,
    theme: Res<UiTheme>,
    mut panels: RegistryPanels,
) {
    let category = catalog.categories.get(state.category_index);
    let category_name = category.map(|c| c.name.as_str()).unwrap_or("");
    let entries = catalog
        .all_entries
        .get(state.category_index)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    for (entity, pane, mut list, mut focus, mut position, computed, mut rows, mut shown) in
        &mut panels.lists
    {
        let is_category = pane.0 == RegistryViewerState::PANE_CATEGORY;
        let source = if is_category { "" } else { category_name };
        let reset = shown.category.as_deref() != Some(source);
        if !reset
            && !state.is_changed()
            && !catalog.is_changed()
            && !theme.is_changed()
            && !list.is_changed()
        {
            continue;
        }
        if reset {
            shown.category = Some(source.to_string());
        }
        let total = if is_category {
            catalog.categories.len()
        } else {
            entries.len()
        };
        let selected = if is_category {
            state.category_index
        } else {
            state.entry_index
        };
        sync_virtual_pane(
            &mut list,
            &mut focus,
            &mut position,
            computed,
            total,
            selected,
            reset,
        );
        let values = (list.window.0..list.window.1)
            .map(|i| {
                let (key, label) = if is_category {
                    let cat = &catalog.categories[i];
                    (
                        (cat.name.clone(), None),
                        format!("{}  ({})", cat.name, cat.count),
                    )
                } else {
                    (
                        (category_name.to_string(), Some(entries[i].id.clone())),
                        entries[i].id.clone(),
                    )
                };
                (
                    key,
                    TextRow {
                        node: Node {
                            padding: UiRect::horizontal(Val::Px(10.0)),
                            ..list.row_node()
                        },
                        background: if i == selected {
                            theme.item_focus_bg()
                        } else if i % 2 == 0 {
                            theme.color(theme::Role::Surface)
                        } else {
                            theme.color(theme::Role::Alternate)
                        },
                        border: Color::NONE,
                        cells: vec![RowCell::new(
                            label,
                            if is_category { 13.0 } else { 12.0 },
                            if is_category && i == selected {
                                theme.accent()
                            } else if is_category || i == selected {
                                theme.color(theme::Role::Text)
                            } else {
                                theme.color(theme::Role::Muted)
                            },
                        )],
                    },
                )
            })
            .collect::<Vec<_>>();
        let values = if total == 0 {
            vec![(
                (source.to_string(), None),
                TextRow {
                    node: list.row_node(),
                    background: theme.color(theme::Role::Surface),
                    border: Color::NONE,
                    cells: vec![RowCell::new(
                        "No entries",
                        14.0,
                        theme.color(theme::Role::Muted),
                    )],
                },
            )]
        } else {
            values
        };
        rows.sync(&mut commands, entity, &list, values);
    }
    for (entity, pane, mut bg, inactive) in &mut panels.tint {
        let active = pane.0 == state.pane;
        bg.set_if_neq(theme::SurfacePaint(if active {
            theme::Role::Raised
        } else {
            theme::Role::Surface
        }));
        if active && inactive.is_some() {
            commands.entity(entity).remove::<InactiveScrollPane>();
        }
        if !active && inactive.is_none() {
            commands.entity(entity).insert(InactiveScrollPane);
        }
    }
    for (mut text, mut color, heading, detail_heading) in &mut panels.title {
        let (label, tint) = if let Some(detail_heading) = detail_heading {
            (
                (if detail_heading.0 == RegistryViewerState::PANE_RAW {
                    "RAW JSON"
                } else {
                    "PARSED STRUCT FIELDS"
                })
                .into(),
                theme::Role::Accent,
            )
        } else if heading.is_some() {
            (
                category
                    .map(|c| format!("DEBUG: REGISTRY VIEWER — {} ({})", c.name, c.count))
                    .unwrap_or_else(|| "DEBUG: REGISTRY VIEWER".into()),
                theme::Role::Accent,
            )
        } else {
            (
                format!(
                    "cat {}/{} · pane: {}",
                    if catalog.categories.is_empty() {
                        0
                    } else {
                        state.category_index + 1
                    },
                    catalog.categories.len(),
                    pane_name(state.pane)
                ),
                theme::Role::Muted,
            )
        };
        text.set_if_neq(Text::new(label));
        color.set_if_neq(theme::TextPaint(tint));
    }
    for (panel, pane, rendered, mut scroll) in &mut panels.details {
        let key = RenderedDetail(
            catalog.last_changed().get(),
            state.category_index,
            state.entry_index,
            theme.preset,
        );
        if rendered == Some(&key) {
            continue;
        }
        // Reset detail scroll only when the selected record changes, not for theme/source refresh.
        if rendered.is_some_and(|old| old.1 != key.1 || old.2 != key.2) {
            if scroll.0 != Vec2::ZERO {
                scroll.0 = Vec2::ZERO;
            }
        }
        commands
            .entity(panel)
            .insert(key)
            .despawn_children()
            .with_children(|parent| {
                let raw = pane.0 == RegistryViewerState::PANE_RAW;
                let entry = entries.get(state.entry_index);
                if !raw {
                    if let Some(entry) = entry {
                        parent.spawn((
                            Text::new(format!("STATUS: {}", entry.status)),
                            TextFont {
                                font_size: 11.0,
                                ..default()
                            },
                            TextColor(if entry.status.contains("DROPPED") {
                                theme::TEXT_RED
                            } else if entry.status.contains("round-trip ok") {
                                theme::TEXT_GREEN
                            } else {
                                theme.color(theme::Role::Muted)
                            }),
                        ));
                    }
                }
                let text = match entry {
                    None => "Select an entry",
                    Some(e) if raw && e.raw_json.is_empty() => {
                        "No raw JSON available\n(possibly saved with a different type key)"
                    }
                    Some(e) if raw => &e.raw_json,
                    Some(e) if e.parsed_fields.is_empty() => "No parsed fields available",
                    Some(e) => &e.parsed_fields,
                };
                parent.spawn((
                    Text::new(text),
                    TextFont {
                        font_size: 11.0,
                        ..default()
                    },
                    TextColor(if raw {
                        theme.color(theme::Role::Text)
                    } else {
                        theme.label_color()
                    }),
                    Node {
                        flex_shrink: 0.0,
                        ..default()
                    },
                ));
            });
    }
}

#[cfg(test)]
#[path = "registry_tests.rs"]
mod presentation_tests;
