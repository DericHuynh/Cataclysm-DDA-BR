use crate::raw_defs::*;
use crate::raw_types::DefId;
use crate::registry::DefRegistry;
use crate::resolve;
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// A raw, unprocessed JSON definition with its source file tracking.
#[derive(Debug, Clone)]
pub struct RawDef {
    /// The `"type"` field value (e.g. "ITEM", "MONSTER", "terrain").
    #[allow(dead_code)]
    pub type_name: String,
    /// The `"id"` or identifying field, extracted from the JSON.
    pub id: Option<String>,
    /// The raw JSON value (the full object).
    pub value: Value,
    /// Source file path for error reporting.
    pub source: PathBuf,
}

/// Error types for data loading and resolution.
#[derive(Debug, thiserror::Error)]
pub enum LoaderError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parse error in {path}: {detail}")]
    JsonParse { path: PathBuf, detail: String },

    #[error("Unknown definition type: {type_name} (in {path})")]
    UnknownType { type_name: String, path: PathBuf },

    #[error("Missing required field 'id' in {path}")]
    MissingId { path: PathBuf },

    #[error("Circular copy-from dependency: {chain:?}")]
    CircularCopyFrom { chain: Vec<String> },

    #[error("Missing copy-from target '{target}' referenced by '{source_path}'")]
    MissingCopyFromTarget { target: String, source_path: String },
}

/// The two-pass JSON loader.
///
/// # Pass 1 — Ingest
/// Walks `data/` directories, reads all `.json` files, parses them into
/// `RawDef` values keyed by their `"type"` field.
///
/// # Pass 2 — Resolve
/// For each def type, topologically sorts by `copy-from` dependency,
/// resolves inheritance chains using `resolve::resolve_copy_from`,
/// filters out abstract definitions, and deserializes into typed structs.
pub struct Loader {
    /// Raw definitions from pass 1, grouped by type.
    raw_by_type: HashMap<String, Vec<RawDef>>,
    /// Directories to scan for JSON files.
    data_dirs: Vec<PathBuf>,
    /// Type aliases: map type names like "GUN", "AMMO" to their canonical type "ITEM".
    type_aliases: HashMap<String, String>,
}

impl Loader {
    /// Create a new loader with the given data directories.
    pub fn new(data_dirs: Vec<PathBuf>) -> Self {
        Loader {
            raw_by_type: HashMap::new(),
            data_dirs,
            type_aliases: Self::default_type_aliases(),
        }
    }

    /// Add a single data directory.
    pub fn with_dir(mut self, dir: PathBuf) -> Self {
        self.data_dirs.push(dir);
        self
    }

    /// Expose the raw-by-type map for inspection or schema validation.
    pub fn raw_by_type(&self) -> &HashMap<String, Vec<RawDef>> {
        &self.raw_by_type
    }

    /// Build the default mapping of CDDA JSON type strings to canonical types.
    ///
    /// CDDA has many ITEM subtypes (GUN, AMMO, COMESTIBLE, etc.) which all
    /// share the same `ItemDef` structure but use different `"type"` values.
    /// This mapping canonicalizes them.
    fn default_type_aliases() -> HashMap<String, String> {
        let mut m = HashMap::new();

        // ITEM subtypes → ITEM
        for subtype in &[
            "GUN",
            "AMMO",
            "COMESTIBLE",
            "TOOL",
            "TOOLMOD",
            "TOOL_ARMOR",
            "GUNMOD",
            "MAGAZINE",
            "BATTERY",
            "ENGINE",
            "GENERIC",
            "PET_ARMOR",
            "WHEEL",
            "BOOK",
        ] {
            m.insert(subtype.to_string(), "ITEM".to_string());
        }

        m
    }

    /// Run the full two-pass load process.
    ///
    /// Returns a fully-resolved `DefRegistry`.
    pub fn load(&mut self) -> Result<DefRegistry, Vec<LoaderError>> {
        let mut errors: Vec<LoaderError> = Vec::new();

        // Pass 1: Ingest all JSON files
        let dirs = self.data_dirs.clone();
        for dir in &dirs {
            self.ingest_directory(dir, &mut errors);
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        let total_raw = self.raw_by_type.values().map(|v| v.len()).sum::<usize>();
        info!(
            "Pass 1 complete: {} raw definitions ingested across {} types",
            total_raw,
            self.raw_by_type.len()
        );

        // Canonicalize type aliases
        self.canonicalize_types();

        // Pass 2: Resolve definitions into typed structs
        let registry = self.resolve_all(&mut errors);

        if !errors.is_empty() {
            warn!(
                "Pass 2 completed with {} non-fatal errors. Returning partial registry.",
                errors.len()
            );
        }

        // Always return the registry, even with errors — field-level
        // deserialization failures shouldn't prevent loading other defs.
        if errors.is_empty() {
            Ok(registry)
        } else {
            // Return errors alongside the partial registry
            Ok(registry)
        }
    }

    /// Merge type aliases into their canonical types.
    /// e.g., "GUN" raw defs are merged into "ITEM".
    fn canonicalize_types(&mut self) {
        let aliases: Vec<(String, String)> = self
            .type_aliases
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        for (alias, canonical) in aliases {
            if let Some(raws) = self.raw_by_type.remove(&alias) {
                info!(
                    "Canonicalizing {} {} defs as {}",
                    raws.len(),
                    alias,
                    canonical
                );
                self.raw_by_type.entry(canonical).or_default().extend(raws);
            }
        }
    }

    /// Pass 1: recursively walk a directory, parse all .json files.
    fn ingest_directory(&mut self, dir: &Path, errors: &mut Vec<LoaderError>) {
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(e) => {
                errors.push(LoaderError::Io(e));
                return;
            }
        };

        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    errors.push(LoaderError::Io(e));
                    continue;
                }
            };

            let path = entry.path();

            // Skip non-JSON and non-directory files
            if path.is_dir() {
                self.ingest_directory(&path, errors);
            } else if path.extension().map_or(false, |ext| ext == "json") {
                // Skip modinfo files (they describe mods, not game defs)
                let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if file_name == "modinfo.json" || file_name == "mod_tileset.json" {
                    return;
                }
                self.ingest_file(&path, errors);
            }
        }
    }

    /// Parse a single JSON file into raw defs.
    fn ingest_file(&mut self, path: &Path, errors: &mut Vec<LoaderError>) {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                errors.push(LoaderError::Io(e));
                return;
            }
        };

        // Parse the JSON — CDDA files are JSON arrays of objects
        let value: Value = match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(e) => {
                errors.push(LoaderError::JsonParse {
                    path: path.to_path_buf(),
                    detail: e.to_string(),
                });
                return;
            }
        };

        let items: &Vec<Value> = match value.as_array() {
            Some(arr) => arr,
            None => {
                // Single object (unusual but possible)
                if let Some(obj) = value.as_object() {
                    self.process_raw_def(obj, path, errors);
                }
                return;
            }
        };

        for item in items {
            if let Some(obj) = item.as_object() {
                self.process_raw_def(obj, path, errors);
            }
        }
    }

    /// Process a single JSON object as a raw definition.
    fn process_raw_def(
        &mut self,
        obj: &serde_json::Map<String, Value>,
        path: &Path,
        _errors: &mut Vec<LoaderError>,
    ) {
        let type_name = match obj.get("type").and_then(|v| v.as_str()) {
            Some(t) => t.to_string(),
            None => return, // skip objects without type
        };

        // Extract the identifying field
        // Most CDDA defs have an "id" field, but some (like recipes) use "result"
        let id = obj
            .get("id")
            .and_then(|v| v.as_str().map(|s| s.to_string()));

        let raw = RawDef {
            type_name: type_name.clone(),
            id,
            value: Value::Object(obj.clone()),
            source: path.to_path_buf(),
        };

        self.raw_by_type.entry(type_name).or_default().push(raw);
    }

    // ========================================================================
    // Pass 2: Resolution
    // ========================================================================

    /// Build a map from def ID string to raw JSON Value for copy-from resolution.
    ///
    /// For types where the identifying key is "id", this extracts that field.
    /// For types like recipes where "result" may be the key, uses a fallback.
    fn build_raw_map(&self, type_name: &str) -> HashMap<String, Value> {
        let mut map = HashMap::new();

        let Some(raws) = self.raw_by_type.get(type_name) else {
            return map;
        };

        for raw in raws {
            // Try "id" first
            if let Some(id) = raw.value.get("id").and_then(|v| v.as_str()) {
                // If duplicate ID, later defs override earlier ones (last-write-wins)
                map.insert(id.to_string(), raw.value.clone());
            } else if let Some(result) = raw.value.get("result").and_then(|v| v.as_str()) {
                // Fallback for recipes and similar types
                map.insert(result.to_string(), raw.value.clone());
            } else if let Some(abstract_id) = raw.value.get("abstract").and_then(|v| v.as_str()) {
                // Some defs use "abstract" as the ID for abstract base defs
                map.insert(abstract_id.to_string(), raw.value.clone());
            } else if let Some(raw_id) = &raw.id {
                map.insert(raw_id.clone(), raw.value.clone());
            }
        }

        map
    }

    /// Extract the def ID from a resolved JSON value for insertion into the registry.
    ///
    /// Tries "id" first, then "result" (for recipes), then "abstract".
    fn extract_def_id(resolved: &Value) -> Option<String> {
        resolved
            .get("id")
            .and_then(|v| v.as_str())
            .or_else(|| resolved.get("result").and_then(|v| v.as_str()))
            .or_else(|| resolved.get("abstract").and_then(|v| v.as_str()))
            .map(|s| s.to_string())
    }

    /// Resolve a single type category using the full copy-from pipeline.
    ///
    /// This is the core of Pass 2:
    /// 1. Build a map of id→raw_value
    /// 2. Topologically sort by copy-from dependencies
    /// 3. Resolve each def (merge parent fields + apply extend/delete/relative/proportional)
    /// 4. Filter out abstract defs
    /// 5. Deserialize into the typed struct
    /// 6. Insert into the registry
    fn resolve_type_with_pipeline<T>(
        &self,
        type_name: &str,
        map: &mut HashMap<DefId<T>, Arc<T>>,
        errors: &mut Vec<LoaderError>,
    ) where
        T: serde::de::DeserializeOwned + std::fmt::Debug,
        DefId<T>: std::hash::Hash + Eq + Clone + From<String>,
    {
        let raw_map = self.build_raw_map(type_name);

        if raw_map.is_empty() {
            debug!("No definitions found for type '{}'", type_name);
            return;
        }

        // Step 1: Topological sort by copy-from dependency.
        // If cycles are detected, we still try to process non-cyclical defs.
        let sorted_ids = match resolve::topological_sort(&raw_map) {
            Ok(ids) => ids,
            Err(cycles) => {
                for cycle in &cycles {
                    errors.push(LoaderError::CircularCopyFrom {
                        chain: cycle.clone(),
                    });
                }
                warn!(
                    "{}: {} circular copy-from cycles detected; processing remaining defs",
                    type_name,
                    cycles.len()
                );
                // Fall back to processing defs in insertion order (skip those
                // that fail due to missing copy-from parents).
                raw_map.keys().map(|k| k.as_str()).collect::<Vec<_>>()
            }
        };

        let mut loaded_count = 0;
        let mut abstract_count = 0;

        // Step 2-6: Resolve each def in dependency order
        for &def_key in &sorted_ids {
            let mut chain = Vec::new();

            // Resolve the full copy-from chain
            let resolved_value = match resolve::resolve_copy_from(def_key, &raw_map, &mut chain) {
                Ok(v) => v,
                Err(msg) => {
                    errors.push(LoaderError::MissingCopyFromTarget {
                        target: msg,
                        source_path: def_key.to_string(),
                    });
                    continue;
                }
            };

            // Step 4: Normalize the resolved JSON before deserialization.
            // - If "id" is missing but "abstract" is present and is a string, copy it as the id.
            // - If "abstract" is a boolean true, skip this def (abstract template).
            let abstract_bool = resolved_value
                .get("abstract_")
                .or_else(|| resolved_value.get("abstract"));

            let is_abstract_template = match abstract_bool {
                Some(Value::Bool(true)) => true,
                _ => false,
            };

            if is_abstract_template {
                abstract_count += 1;
                debug!("Skipping abstract def '{}' (type={})", def_key, type_name);
                continue;
            }

            // If the def has "abstract": "some_name" instead of "id": "some_name",
            // promote the abstract field value to id for deserialization.
            let has_id = resolved_value.get("id").and_then(|v| v.as_str()).is_some();
            let normalized_value = if !has_id {
                if let Some(abs_id) = resolved_value.get("abstract").and_then(|v| v.as_str()) {
                    let mut obj = resolved_value.as_object().cloned().unwrap_or_default();
                    obj.insert("id".to_string(), Value::String(abs_id.to_string()));
                    Value::Object(obj)
                } else {
                    resolved_value.clone()
                }
            } else {
                resolved_value.clone()
            };

            // Step 5: Deserialize the resolved JSON into the typed struct
            match serde_json::from_value::<T>(normalized_value.clone()) {
                Ok(def) => {
                    // Extract the final ID (may differ from the map key for recipes etc.)
                    let final_id = Self::extract_def_id(&normalized_value)
                        .unwrap_or_else(|| def_key.to_string());

                    map.insert(DefId::from(final_id), Arc::new(def));
                    loaded_count += 1;
                }
                Err(e) => {
                    // Try to give a helpful error message
                    let id_hint = Self::extract_def_id(&normalized_value)
                        .unwrap_or_else(|| def_key.to_string());
                    errors.push(LoaderError::JsonParse {
                        path: PathBuf::from(type_name),
                        detail: format!("Failed to deserialize {} '{}': {}", type_name, id_hint, e),
                    });
                }
            }
        }

        info!(
            "Loaded {} {} definitions ({} abstract skipped)",
            loaded_count, type_name, abstract_count
        );
    }

    /// Pass 2: resolve all raw definitions into typed structs.
    fn resolve_all(&self, errors: &mut Vec<LoaderError>) -> DefRegistry {
        let mut registry = DefRegistry::empty();

        // ---- Items ----
        self.resolve_type_with_pipeline::<ItemDef>("ITEM", &mut registry.items, errors);

        // ---- Monsters ----
        self.resolve_type_with_pipeline::<MonsterDef>("MONSTER", &mut registry.monsters, errors);

        // ---- Terrain / Furniture ----
        self.resolve_type_with_pipeline::<TerrainDef>("terrain", &mut registry.terrain, errors);
        self.resolve_type_with_pipeline::<FurnitureDef>(
            "furniture",
            &mut registry.furniture,
            errors,
        );

        // ---- Recipes ----
        self.resolve_type_with_pipeline::<RecipeDef>("recipe", &mut registry.recipes, errors);

        // ---- Item groups / Palettes ----
        self.resolve_type_with_pipeline::<ItemGroupDef>(
            "item_group",
            &mut registry.item_groups,
            errors,
        );
        self.resolve_type_with_pipeline::<MapgenPaletteDef>(
            "palette",
            &mut registry.palettes,
            errors,
        );

        // ---- Overmap ----
        self.resolve_type_with_pipeline::<OvermapTerrainDef>(
            "overmap_terrain",
            &mut registry.overmap_terrains,
            errors,
        );
        self.resolve_type_with_pipeline::<OvermapSpecialDef>(
            "overmap_special",
            &mut registry.overmap_specials,
            errors,
        );
        self.resolve_type_with_pipeline::<OvermapConnectionDef>(
            "overmap_connection",
            &mut registry.overmap_connections,
            errors,
        );
        self.resolve_type_with_pipeline::<OvermapLocationDef>(
            "overmap_location",
            &mut registry.overmap_locations,
            errors,
        );
        self.resolve_type_with_pipeline::<OvermapLandUseCodeDef>(
            "overmap_land_use_code",
            &mut registry.overmap_land_use_codes,
            errors,
        );

        // ---- Fields ----
        self.resolve_type_with_pipeline::<FieldDef>("field_type", &mut registry.fields, errors);

        // ---- Vehicle parts ----
        self.resolve_type_with_pipeline::<VehiclePartDef>(
            "vehicle_part",
            &mut registry.vehicle_parts,
            errors,
        );
        self.resolve_type_with_pipeline::<VehiclePartLocationDef>(
            "vehicle_part_location",
            &mut registry.vehicle_part_locations,
            errors,
        );
        self.resolve_type_with_pipeline::<VehiclePartCategoryDef>(
            "vehicle_part_category",
            &mut registry.vehicle_part_categories,
            errors,
        );

        // ---- Mutations ----
        self.resolve_type_with_pipeline::<MutationDef>("mutation", &mut registry.mutations, errors);
        self.resolve_type_with_pipeline::<MutationCategoryDef>(
            "mutation_category",
            &mut registry.mutation_categories,
            errors,
        );
        self.resolve_type_with_pipeline::<TraitGroupDef>(
            "trait_group",
            &mut registry.trait_groups,
            errors,
        );

        // ---- Bionics ----
        self.resolve_type_with_pipeline::<BionicDef>("bionic", &mut registry.bionics, errors);

        // ---- Effects ----
        self.resolve_type_with_pipeline::<EffectDef>("effect_type", &mut registry.effects, errors);

        // ---- Factions / Scenarios ----
        self.resolve_type_with_pipeline::<FactionDef>("faction", &mut registry.factions, errors);
        self.resolve_type_with_pipeline::<ScenarioDef>("scenario", &mut registry.scenarios, errors);

        // ---- Materials / Skills ----
        self.resolve_type_with_pipeline::<MaterialDef>("material", &mut registry.materials, errors);
        self.resolve_type_with_pipeline::<SkillDef>("skill", &mut registry.skills, errors);

        // ---- Traps / Start Locations ----
        self.resolve_type_with_pipeline::<TrapDef>("trap", &mut registry.traps, errors);
        self.resolve_type_with_pipeline::<StartLocationDef>(
            "start_location",
            &mut registry.start_locations,
            errors,
        );

        // ---- Mapgen (deferred for Stage 2) ----
        self.resolve_mapgen(&mut registry, errors);

        // ---- Log skipped types ----
        self.log_skipped_types();

        info!(
            "Pass 2 complete: {} total definitions across {} categories",
            registry.total_count(),
            registry.category_count()
        );

        registry
    }

    /// Log types that exist in the raw data but don't have a registered handler.
    fn log_skipped_types(&self) {
        let handled_types: std::collections::HashSet<&str> = [
            "ITEM",
            "MONSTER",
            "terrain",
            "furniture",
            "recipe",
            "item_group",
            "palette",
            "overmap_terrain",
            "overmap_special",
            "overmap_connection",
            "overmap_location",
            "overmap_land_use_code",
            "field_type",
            "vehicle_part",
            "vehicle_part_location",
            "vehicle_part_category",
            "mutation",
            "mutation_category",
            "trait_group",
            "bionic",
            "effect_type",
            "faction",
            "scenario",
            "material",
            "skill",
            "trap",
            "start_location",
            "mapgen",
        ]
        .iter()
        .copied()
        .collect();

        for type_name in self.raw_by_type.keys() {
            if !handled_types.contains(type_name.as_str()) {
                let count = self.raw_by_type[type_name].len();
                debug!(
                    "Skipped type '{}' ({} defs) — no registered handler",
                    type_name, count
                );
            }
        }
    }

    /// Resolve mapgen definitions (special: multiple per OMT).
    fn resolve_mapgen(&self, _registry: &mut DefRegistry, _errors: &mut Vec<LoaderError>) {
        // TODO: Mapgen defs need special handling because they key by `om_terrain`
        //       which can be a single ID, a list of IDs, or absent.
        //       This will be implemented when the mapgen pipeline is active.
        if self.raw_by_type.contains_key("mapgen") {
            let count = self.raw_by_type["mapgen"].len();
            debug!("Mapgen resolution deferred ({} defs, Stage 2)", count);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // -----------------------------------------------------------------------
    // Loader construction
    // -----------------------------------------------------------------------

    /// Verify a new Loader starts with no data and no directories.
    #[test]
    fn loader_new_is_empty() {
        // Arrange
        let dirs: Vec<PathBuf> = vec![];

        // Act
        let loader = Loader::new(dirs);

        // Assert
        assert!(loader.data_dirs.is_empty());
        assert!(loader.raw_by_type.is_empty());
        assert!(!loader.type_aliases.is_empty()); // default aliases populated
    }

    /// Verify with_dir() appends directories.
    #[test]
    fn with_dir_adds_directory() {
        // Arrange
        let mut loader = Loader::new(vec![]);
        let dir = PathBuf::from("/fake/path");

        // Act
        loader = loader.with_dir(dir.clone());

        // Assert
        assert_eq!(loader.data_dirs.len(), 1);
        assert_eq!(loader.data_dirs[0], dir);
    }

    // -----------------------------------------------------------------------
    // Type alias canonicalization
    // -----------------------------------------------------------------------

    /// Verify CDDA ITEM subtypes (GUN, AMMO, etc.) are mapped to "ITEM".
    #[test]
    fn default_type_aliases_maps_item_subtypes() {
        // Arrange — type aliases built in constructor

        // Act
        let aliases = Loader::default_type_aliases();

        // Assert
        assert_eq!(aliases.get("GUN"), Some(&"ITEM".to_string()));
        assert_eq!(aliases.get("AMMO"), Some(&"ITEM".to_string()));
        assert_eq!(aliases.get("COMESTIBLE"), Some(&"ITEM".to_string()));
        assert_eq!(aliases.get("TOOL"), Some(&"ITEM".to_string()));
        assert_eq!(aliases.get("MAGAZINE"), Some(&"ITEM".to_string()));
        assert_eq!(aliases.get("BOOK"), Some(&"ITEM".to_string()));
    }

    /// Verify canonicalize_types() merges GUN definitions into ITEM.
    #[test]
    fn canonicalize_types_merges_aliases() {
        // Arrange
        let mut loader = Loader::new(vec![]);
        let raw_gun = RawDef {
            type_name: "GUN".into(),
            id: Some("glock".into()),
            value: json!({"type": "GUN", "id": "glock"}),
            source: PathBuf::from("guns.json"),
        };
        let raw_item = RawDef {
            type_name: "ITEM".into(),
            id: Some("rock".into()),
            value: json!({"type": "ITEM", "id": "rock"}),
            source: PathBuf::from("misc.json"),
        };
        loader
            .raw_by_type
            .entry("GUN".to_string())
            .or_default()
            .push(raw_gun);
        loader
            .raw_by_type
            .entry("ITEM".to_string())
            .or_default()
            .push(raw_item);

        // Act
        loader.canonicalize_types();

        // Assert
        assert!(
            !loader.raw_by_type.contains_key("GUN"),
            "GUN should be removed"
        );
        assert_eq!(loader.raw_by_type.get("ITEM").map(|v| v.len()), Some(2));
    }

    // -----------------------------------------------------------------------
    // extract_def_id
    // -----------------------------------------------------------------------

    /// Should return "id" field first.
    #[test]
    fn extract_def_id_prefers_id_field() {
        // Arrange
        let value = json!({"id": "thing", "result": "other_id"});

        // Act
        let id = Loader::extract_def_id(&value);

        // Assert
        assert_eq!(id, Some("thing".to_string()));
    }

    /// Should fall back to "result" when no "id".
    #[test]
    fn extract_def_id_falls_back_to_result() {
        // Arrange
        let value = json!({"result": "crafted_item"});

        // Act
        let id = Loader::extract_def_id(&value);

        // Assert
        assert_eq!(id, Some("crafted_item".to_string()));
    }

    /// Should fall back to "abstract" when no "id" or "result".
    #[test]
    fn extract_def_id_falls_back_to_abstract() {
        // Arrange
        let value = json!({"abstract": "base_def"});

        // Act
        let id = Loader::extract_def_id(&value);

        // Assert
        assert_eq!(id, Some("base_def".to_string()));
    }

    /// Returns None when no identifying field exists.
    #[test]
    fn extract_def_id_returns_none_when_no_id() {
        // Arrange
        let value = json!({"name": "just a name"});

        // Act
        let id = Loader::extract_def_id(&value);

        // Assert
        assert_eq!(id, None);
    }

    // -----------------------------------------------------------------------
    // build_raw_map
    // -----------------------------------------------------------------------

    /// Should build a map keyed by "id".
    #[test]
    fn build_raw_map_keys_by_id() {
        // Arrange
        let mut loader = Loader::new(vec![]);
        loader.raw_by_type.insert(
            "test_type".into(),
            vec![
                RawDef {
                    type_name: "test_type".into(),
                    id: Some("alpha".into()),
                    value: json!({"id": "alpha", "volume": "250 ml"}),
                    source: PathBuf::new(),
                },
                RawDef {
                    type_name: "test_type".into(),
                    id: Some("beta".into()),
                    value: json!({"id": "beta", "volume": "500 ml"}),
                    source: PathBuf::new(),
                },
            ],
        );

        // Act
        let map = loader.build_raw_map("test_type");

        // Assert
        assert_eq!(map.len(), 2);
        assert!(map.contains_key("alpha"));
        assert!(map.contains_key("beta"));
    }

    /// Should fall back to "result" when "id" is missing.
    #[test]
    fn build_raw_map_falls_back_to_result() {
        // Arrange
        let mut loader = Loader::new(vec![]);
        loader.raw_by_type.insert(
            "recipe".into(),
            vec![RawDef {
                type_name: "recipe".into(),
                id: None,
                value: json!({"result": "soup", "difficulty": 1}),
                source: PathBuf::new(),
            }],
        );

        // Act
        let map = loader.build_raw_map("recipe");

        // Assert
        assert_eq!(map.len(), 1);
        assert!(map.contains_key("soup"));
    }

    /// Should use last-write-wins for duplicate IDs.
    #[test]
    fn build_raw_map_last_write_wins() {
        // Arrange
        let mut loader = Loader::new(vec![]);
        loader.raw_by_type.insert(
            "test_type".into(),
            vec![
                RawDef {
                    type_name: "test_type".into(),
                    id: Some("dupe".into()),
                    value: json!({"id": "dupe", "volume": "100 ml"}),
                    source: PathBuf::from("first.json"),
                },
                RawDef {
                    type_name: "test_type".into(),
                    id: Some("dupe".into()),
                    value: json!({"id": "dupe", "volume": "200 ml"}),
                    source: PathBuf::from("second.json"),
                },
            ],
        );

        // Act
        let map = loader.build_raw_map("test_type");

        // Assert
        assert_eq!(map.len(), 1);
        assert_eq!(
            map.get("dupe")
                .and_then(|v| v.get("volume").and_then(|v| v.as_str())),
            Some("200 ml")
        );
    }

    // -----------------------------------------------------------------------
    // LoaderError formatting
    // -----------------------------------------------------------------------

    #[test]
    fn loader_error_io_displays() {
        // Arrange
        let err = LoaderError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "gone"));

        // Act
        let msg = err.to_string();

        // Assert
        assert!(msg.contains("IO error"));
        assert!(msg.contains("gone"));
    }

    #[test]
    fn loader_error_circular_displays_chain() {
        // Arrange
        let err = LoaderError::CircularCopyFrom {
            chain: vec!["a".into(), "b".into(), "a".into()],
        };

        // Act
        let msg = err.to_string();

        // Assert
        assert!(msg.contains("Circular"));
        assert!(msg.contains("a"));
        assert!(msg.contains("b"));
    }

    // -----------------------------------------------------------------------
    // process_raw_def
    // -----------------------------------------------------------------------

    /// Objects without a "type" field should be skipped silently.
    #[test]
    fn process_raw_def_skips_typeless_objects() {
        // Arrange
        let mut loader = Loader::new(vec![]);
        let obj = json!({"id": "orphan", "name": "No Type"})
            .as_object()
            .unwrap()
            .clone();

        // Act
        loader.process_raw_def(&obj, Path::new("test.json"), &mut vec![]);

        // Assert
        assert!(loader.raw_by_type.is_empty());
    }

    /// Objects with a "type" field should be ingested.
    #[test]
    fn process_raw_def_ingests_typed_object() {
        // Arrange
        let mut loader = Loader::new(vec![]);
        let obj = json!({"type": "ITEM", "id": "rock", "name": "Rock"})
            .as_object()
            .unwrap()
            .clone();

        // Act
        loader.process_raw_def(&obj, Path::new("items.json"), &mut vec![]);

        // Assert
        assert_eq!(loader.raw_by_type.len(), 1);
        assert_eq!(loader.raw_by_type["ITEM"].len(), 1);
        assert_eq!(loader.raw_by_type["ITEM"][0].id, Some("rock".to_string()));
    }
}
