//! Dynamic JSON Schema generation with runtime-injected enums.
//!
//! 1. Generates a base schema from Rust types via `schemars`.
//! 2. Patches the JSON tree to inject `enum` values for fields that
//!    reference runtime data (flags, copy-from IDs, delete targets).
//! 3. Writes the final schema to disk so modders get LSP autocomplete.
//!
//! Modders reference the schema in their JSON:
//! ```json
//! {
//!   "$schema": "../../schemas/item_mod.schema.json",
//!   "id": "my_sword",
//!   "copy-from": "broadsword",
//!   "delete": { "flags": ["HEAVY"] }
//! }
//! ```

use bevy_app::{App, Plugin};
use bevy_ecs::prelude::*;
use schemars::schema::RootSchema;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::path::PathBuf;
use tracing::info;

// ---------------------------------------------------------------------------
// ModRegistry
// ---------------------------------------------------------------------------

#[derive(Resource, Debug, Clone, Default)]
pub struct ModRegistry {
    pub all_flags: HashSet<String>,
    pub all_item_ids: HashSet<String>,
}

// ---------------------------------------------------------------------------
// Schema structs
// ---------------------------------------------------------------------------

/// A CDDA item definition, as modders write it in JSON.
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ItemModSchema {
    pub id: String,
    #[serde(rename = "copy-from")]
    pub copy_from: Option<String>,
    #[serde(default)]
    pub flags: Option<Vec<String>>,
    #[serde(default)]
    pub delete: Option<DeletePatch>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct DeletePatch {
    #[serde(default)]
    pub flags: Option<Vec<String>>,
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct CddaSchemaPlugin;

impl Plugin for CddaSchemaPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ModRegistry>();
    }
}

// ---------------------------------------------------------------------------
// collect_mod_registry_v2
// ---------------------------------------------------------------------------

macro_rules! collect_flags {
    ($world:expr, $out:expr, $reg:ty) => {
        if let Some(r) = $world.get_resource::<$reg>() {
            for flag in r.0.flags() {
                $out.insert(flag.to_string());
            }
        }
    };
}

pub fn collect_mod_registry_v2(world: &mut World) {
    let mut flags = HashSet::new();
    collect_flags!(world, flags, crate::flags::ItemFlagRegistry);
    collect_flags!(world, flags, crate::flags::MonsterFlagRegistry);
    collect_flags!(world, flags, crate::flags::TerrainFlagRegistry);
    collect_flags!(world, flags, crate::flags::FurnitureFlagRegistry);
    collect_flags!(world, flags, crate::flags::MeleeFlagRegistry);
    collect_flags!(world, flags, crate::flags::ArmorFlagRegistry);
    collect_flags!(world, flags, crate::flags::GunFlagRegistry);

    let mut ids = HashSet::new();
    if let Some(def_world) = world.get_resource::<crate::def_world::DefinitionWorld>() {
        for (id, _) in def_world.iter() {
            ids.insert(id.to_string());
        }
    }

    // Ensure ModRegistry exists (may not if CddaSchemaPlugin wasn't added).
    if world.get_resource::<ModRegistry>().is_none() {
        world.insert_resource(ModRegistry::default());
    }
    let mut registry = world.resource_mut::<ModRegistry>();
    registry.all_flags = flags;
    registry.all_item_ids = ids;

    info!(
        "ModRegistry: {} flags, {} def IDs",
        registry.all_flags.len(),
        registry.all_item_ids.len(),
    );
}

// ---------------------------------------------------------------------------
// generate_dynamic_schemas
// ---------------------------------------------------------------------------

pub fn generate_dynamic_schemas(registry: Res<ModRegistry>) {
    let out_dir = schema_output_dir();
    let _ = std::fs::create_dir_all(&out_dir);

    let item_schema = generate_schema_for::<ItemModSchema>();
    let mut item_json = serde_json::to_value(&item_schema).expect("ItemModSchema should serialize");

    patch_enum(
        &mut item_json,
        "properties/flags/items",
        &registry.all_flags,
    );
    patch_enum(
        &mut item_json,
        "properties/delete/properties/flags/items",
        &registry.all_flags,
    );
    patch_enum(&mut item_json, "properties/id", &registry.all_item_ids);
    patch_enum(
        &mut item_json,
        "properties/copy-from",
        &registry.all_item_ids,
    );

    write_schema_file(&out_dir.join("item_mod.schema.json"), &item_json);

    info!(
        "Dynamic schemas written to {:?} ({} flags, {} ids)",
        out_dir,
        registry.all_flags.len(),
        registry.all_item_ids.len(),
    );
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn generate_schema_for<T: JsonSchema>() -> RootSchema {
    schemars::gen::SchemaGenerator::default().into_root_schema_for::<T>()
}

fn patch_enum(schema: &mut Value, json_path: &str, values: &HashSet<String>) {
    let target = match navigate_mut(schema, json_path) {
        Some(v) => v,
        None => {
            tracing::warn!("Schema path not found: {json_path}");
            return;
        }
    };
    let mut sorted: Vec<&String> = values.iter().collect();
    sorted.sort();
    *target = json!({ "type": "string", "enum": sorted });
}

fn navigate_mut<'v>(root: &'v mut Value, path: &str) -> Option<&'v mut Value> {
    let mut current = root;
    for segment in path.split('/') {
        current = current.get_mut(segment)?;
    }
    Some(current)
}

fn write_schema_file(path: &PathBuf, value: &Value) {
    let json_str = serde_json::to_string_pretty(value).expect("Schema value should serialize");
    if let Err(e) = std::fs::write(path, &json_str) {
        tracing::error!("Cannot write schema file {:?}: {e}", path);
    } else {
        info!("  wrote {}", path.display());
    }
}

fn schema_output_dir() -> PathBuf {
    let cwd = std::env::current_dir().unwrap_or_default();
    let candidate = cwd.join("assets/schemas");
    if candidate.exists() || cwd.join("assets").exists() {
        return candidate;
    }
    if let Ok(dir) = std::env::var("CARGO_MANIFEST_DIR") {
        return PathBuf::from(dir).join("../../data/schemas");
    }
    candidate
}

// ---------------------------------------------------------------------------
// collect_and_generate_schemas
// ---------------------------------------------------------------------------

pub fn collect_and_generate_schemas(world: &mut World) {
    collect_mod_registry_v2(world);
    let registry = world.resource::<ModRegistry>().clone();
    write_schemas_to_dir(&schema_output_dir(), &registry);
}

// ---------------------------------------------------------------------------
// generate_schemas_for_mod (CLI / headless)
// ---------------------------------------------------------------------------

/// Load data from `data_dirs`, discover all flags and IDs, and write
/// schema files to `out_base/<mod_name>/`.
///
/// `data_dirs` should include the core data directory plus any mod
/// directories whose content should appear in the autocomplete enums.
/// Typically: `[core_dir, mod_dir]` for a mod, or `[core_dir]` for core.
pub fn generate_schemas_for_mod(
    mod_name: &str,
    data_dirs: &[PathBuf],
    out_base: &PathBuf,
) -> Result<(), Vec<String>> {
    use crate::loader::Loader;

    let mut loader = Loader::new(data_dirs.to_vec());
    let _raw = loader.ingest_all();

    let registry = match loader.load() {
        Ok(def_registry) => {
            let mut flags = HashSet::new();
            let mut ids = HashSet::new();

            for (_def_id, item) in &def_registry.items {
                ids.insert(item.id.as_str().to_string());
                for flag in &item.flags {
                    flags.insert(flag.clone());
                }
            }
            for (_def_id, monster) in &def_registry.monsters {
                ids.insert(monster.id.as_str().to_string());
                for flag in &monster.flags {
                    flags.insert(flag.clone());
                }
            }
            for (_def_id, terrain) in &def_registry.terrain {
                ids.insert(terrain.id.as_str().to_string());
                for flag in crate::def_world::flags_to_vec(&terrain.flags) {
                    flags.insert(flag);
                }
            }
            for (_def_id, furniture) in &def_registry.furniture {
                ids.insert(furniture.id.as_str().to_string());
                for flag in crate::def_world::flags_to_vec(&furniture.flags) {
                    flags.insert(flag);
                }
            }

            ModRegistry {
                all_flags: flags,
                all_item_ids: ids,
            }
        }
        Err(errors) => {
            return Err(errors.iter().map(|e| format!("{:?}", e)).collect());
        }
    };

    let out_dir = out_base.join(mod_name);
    write_schemas_to_dir(&out_dir, &registry);
    Ok(())
}

/// Write schema files to a specific output directory.
fn write_schemas_to_dir(out_dir: &PathBuf, registry: &ModRegistry) {
    let _ = std::fs::create_dir_all(out_dir);

    let item_schema = generate_schema_for::<ItemModSchema>();
    let mut item_json = serde_json::to_value(&item_schema).expect("ItemModSchema should serialize");

    patch_enum(
        &mut item_json,
        "properties/flags/items",
        &registry.all_flags,
    );
    patch_enum(
        &mut item_json,
        "properties/delete/properties/flags/items",
        &registry.all_flags,
    );
    patch_enum(&mut item_json, "properties/id", &registry.all_item_ids);
    patch_enum(
        &mut item_json,
        "properties/copy-from",
        &registry.all_item_ids,
    );

    write_schema_file(&out_dir.join("item_mod.schema.json"), &item_json);

    info!(
        "Dynamic schemas written to {:?} ({} flags, {} ids)",
        out_dir,
        registry.all_flags.len(),
        registry.all_item_ids.len(),
    );
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // $schema tolerance
    // -----------------------------------------------------------------------

    /// Modders add `"$schema"` to their JSON for LSP autocomplete.
    /// The loader must ignore it and parse the rest correctly.
    #[test]
    fn dollar_schema_is_ignored_during_deserialization() {
        let json = json!({
            "$schema": "../../schemas/item_mod.schema.json",
            "id": "my_sword",
            "copy-from": "broadsword",
            "flags": ["FLAMING", "DURABLE_MELEE"],
            "delete": { "flags": ["HEAVY"] }
        });

        let item: ItemModSchema =
            serde_json::from_value(json).expect("deserialization should succeed with $schema");

        assert_eq!(item.id, "my_sword");
        assert_eq!(item.copy_from.as_deref(), Some("broadsword"));
        assert_eq!(
            item.flags,
            Some(vec!["FLAMING".into(), "DURABLE_MELEE".into()])
        );
        let del = item.delete.expect("delete field should be present");
        assert_eq!(del.flags, Some(vec!["HEAVY".into()]));
    }

    /// `$schema` should also be tolerated when there's no delete block.
    #[test]
    fn dollar_schema_without_delete() {
        let json = json!({
            "$schema": "../schemas/item_mod.schema.json",
            "id": "simple_item",
            "flags": ["FIRE"]
        });

        let item: ItemModSchema =
            serde_json::from_value(json).expect("deserialization should succeed");

        assert_eq!(item.id, "simple_item");
        assert_eq!(item.flags, Some(vec!["FIRE".into()]));
        assert!(item.delete.is_none());
        assert!(item.copy_from.is_none());
    }

    /// `$schema` should work even when it's the only extra field on a minimal def.
    #[test]
    fn dollar_schema_on_minimal_def() {
        let json = json!({
            "$schema": "schemas/item_mod.schema.json",
            "id": "minimal"
        });

        let item: ItemModSchema =
            serde_json::from_value(json).expect("deserialization should succeed");

        assert_eq!(item.id, "minimal");
        assert!(item.flags.is_none());
        assert!(item.delete.is_none());
        assert!(item.copy_from.is_none());
    }

    // -----------------------------------------------------------------------
    // Full ItemDef tolerance (the real struct modders extend)
    // -----------------------------------------------------------------------

    /// The real `ItemDef` struct must also tolerate `$schema`.
    #[test]
    fn dollar_schema_on_real_item_def() {
        use cdda_core_types::core::raw_defs::item::ItemDef;

        let json = json!({
            "$schema": "../../schemas/item_mod.schema.json",
            "id": "test_sword",
            "name": {"str": "Test Sword"},
            "volume": "250 ml",
            "flags": ["FLAMING"]
        });

        let def: ItemDef = serde_json::from_value(json)
            .expect("ItemDef deserialization should succeed with $schema");

        assert_eq!(def.id.as_str(), "test_sword");
        assert_eq!(def.flags, vec!["FLAMING"]);
    }

    // -----------------------------------------------------------------------
    // Enum injection
    // -----------------------------------------------------------------------

    #[test]
    fn patch_enum_replaces_type_string_with_enum_array() {
        let mut schema = json!({
            "properties": {
                "flags": {
                    "items": { "type": "string" }
                }
            }
        });

        let mut values = HashSet::new();
        values.insert("FIRE".into());
        values.insert("WET".into());

        patch_enum(&mut schema, "properties/flags/items", &values);

        let items = &schema["properties"]["flags"]["items"];
        assert_eq!(items["type"], "string");
        let arr = items["enum"].as_array().expect("enum should be an array");
        assert_eq!(arr.len(), 2);
        assert!(arr.contains(&json!("FIRE")));
        assert!(arr.contains(&json!("WET")));
    }

    #[test]
    fn patch_enum_missing_path_is_harmless() {
        let mut schema = json!({ "properties": {} });
        let values = HashSet::new();
        // Should not panic.
        patch_enum(&mut schema, "properties/nonexistent/items", &values);
    }
}
