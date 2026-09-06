use crate::for_each_raw_def_kind;
use crate::registry::DefRegistry;
use crate::resolve;
use cdda_core_types::core::id::DefId;
use cdda_defs_raw::raw_defs::*;
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// A raw, unprocessed JSON definition with its source file tracking.
#[derive(Debug, Clone)]
pub struct RawDef {
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

    /// Run Pass 1 only: ingest all JSON files and return the raw-by-type map.
    ///
    /// Useful for mod loading where you want raw defs without resolving.
    /// After calling, `self` contains the ingested data and can still be
    /// fully resolved via `load()` on a subsequent call (with more dirs).
    pub fn ingest_all(&mut self) -> HashMap<String, Vec<RawDef>> {
        let mut errors: Vec<LoaderError> = Vec::new();
        let dirs = self.data_dirs.clone();
        for dir in &dirs {
            self.ingest_directory(dir, &mut errors);
        }
        self.canonicalize_types();
        self.raw_by_type.clone()
    }

    /// Ingest one more directory into the existing raw map and return only
    /// the newly ingested definitions.
    ///
    /// This is the mod-layering seam: the loader keeps the core raw
    /// definitions it already ingested, a mod dir is appended on top, and a
    /// subsequent [`Self::resolve`] layers the mod over core with
    /// last-write-wins per ID (and lets mod defs `copy-from` core defs).
    pub fn ingest_dir(&mut self, dir: &Path) -> (HashMap<String, Vec<RawDef>>, Vec<LoaderError>) {
        let before: HashMap<String, usize> = self
            .raw_by_type
            .iter()
            .map(|(k, v)| (k.clone(), v.len()))
            .collect();
        let mut errors: Vec<LoaderError> = Vec::new();
        self.ingest_directory(dir, &mut errors);
        self.canonicalize_types();
        let mut new_defs: HashMap<String, Vec<RawDef>> = HashMap::new();
        for (kind, raws) in &self.raw_by_type {
            let start = before.get(kind).copied().unwrap_or(0);
            if raws.len() > start {
                new_defs.insert(kind.clone(), raws[start..].to_vec());
            }
        }
        (new_defs, errors)
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

        self.resolve()
    }

    /// Ingest already-parsed JSON values (one entry per file) instead of
    /// reading `.json` from disk. This is the hot-reload seam: callers that
    /// source bytes through Bevy's asset reader (see `CddaDataPackLoader`)
    /// still go through exactly the same [`serde_json::Value`] ingestion and
    /// `copy-from` resolution pipeline as a disk-backed load.
    ///
    /// `files` is `(stable_source_path, top_level_json_values)` where the path
    /// is used only for diagnostics/error attribution.
    pub fn ingest_values(&mut self, files: Vec<(std::path::PathBuf, Vec<Value>)>) {
        let mut errors: Vec<LoaderError> = Vec::new();
        for (path, values) in files {
            for item in values {
                if let Some(obj) = item.as_object() {
                    self.process_raw_def(obj, &path, &mut errors);
                }
            }
        }
        self.canonicalize_types();
        if !errors.is_empty() {
            warn!(
                "Ingest via Bevy asset reader reported {} errors",
                errors.len()
            );
        }
    }

    /// Pass 2 only: resolve the already-ingested [`Self::raw_by_type`] into a
    /// [`DefRegistry`], without touching disk. Call after [`Loader::ingest_all`]
    /// or [`Loader::ingest_values`].
    pub fn resolve(&mut self) -> Result<DefRegistry, Vec<LoaderError>> {
        let mut errors: Vec<LoaderError> = Vec::new();

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

        let mut entries: Vec<_> = entries.collect();
        entries.sort_by_key(|entry| entry.as_ref().map(|entry| entry.path()).unwrap_or_default());
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
                    continue;
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
        // Most CDDA defs have an "id" field, but some (like recipes) use "result".
        // "id" may be a string or an array of strings; use the first element.
        let id = obj
            .get("id")
            .and_then(Self::first_id_from_value)
            .map(|s| s.to_string());

        let raw = RawDef {
            id,
            value: Value::Object(obj.clone()),
            source: path.to_path_buf(),
        };

        // Canonicalize at ingestion, preserving source/mod order even when an
        // override changes between aliases such as GENERIC and ITEM.
        let family = self
            .type_aliases
            .get(&type_name)
            .cloned()
            .unwrap_or(type_name);
        self.raw_by_type.entry(family).or_default().push(raw);
    }

    // ========================================================================
    // Pass 2: Resolution
    // ========================================================================

    /// Extract the primary ID from a JSON "id" field, which may be a string
    /// or an array of strings (e.g. `["field", "isherwood_barry_rescue_field"]`).
    /// Returns the first string element for arrays, or the string itself.
    fn first_id_from_value(v: &Value) -> Option<&str> {
        if let Some(s) = v.as_str() {
            Some(s)
        } else if let Some(arr) = v.as_array() {
            arr.first().and_then(|s| s.as_str())
        } else {
            None
        }
    }

    /// Build a map from def ID string to raw JSON Value for copy-from resolution.
    ///
    /// For types where the identifying key is "id", this extracts that field.
    /// For types like recipes where "result" may be the key, uses a fallback.
    fn build_raw_map(&self, type_name: &str) -> HashMap<String, Value> {
        self.keyed_sources(type_name)
            .into_iter()
            .map(|(key, raw)| (key, raw.value.clone()))
            .collect()
    }

    /// Last-writer provenance using exactly the same identity rules as resolution.
    pub fn keyed_sources(&self, type_name: &str) -> HashMap<String, &RawDef> {
        let mut map = HashMap::new();

        let Some(raws) = self.raw_by_type.get(type_name) else {
            return map;
        };

        for raw in raws {
            // Recipes are identified by their *composite* id: `result` plus an
            // optional `id_suffix` (`herbal_tea` + `from_tea_bag` →
            // `herbal_tea_from_tea_bag`). CDDA recipes `copy-from` by that
            // composite name. A recipe with a `result` is keyed by its
            // composite (or bare `result` when no suffix); a recipe with no
            // `result` (a named abstract like `"abstract": "seed_extraction_base"`)
            // falls through to abstract keying below.
            if type_name == "recipe" {
                let result = raw.value.get("result").and_then(serde_json::Value::as_str);
                // A recipe's composite identity is `result` plus an `id_suffix`
                // OR a `variant` (`deck_of_cards` + `variant:
                // deck_of_cards_makeshift` → `deck_of_cards_deck_of_cards_makeshift`).
                // `copy-from` references resolve against this composite name.
                let suffix = raw
                    .value
                    .get("id_suffix")
                    .or_else(|| raw.value.get("variant"))
                    .and_then(serde_json::Value::as_str)
                    .filter(|s| !s.is_empty());
                if let Some(result) = result {
                    let key = match suffix {
                        Some(sfx) => format!("{result}_{sfx}"),
                        None => result.to_string(),
                    };
                    // A pure `variant` re-declaration that `copy-from`s its own
                    // computed key is a duplicate, not a real base — skip it so
                    // it doesn't overwrite the primary recipe or self-cycle.
                    let self_copy = raw
                        .value
                        .get("copy-from")
                        .and_then(serde_json::Value::as_str)
                        == Some(key.as_str());
                    if !self_copy || raw.value.get("variant").is_none() {
                        map.insert(key, raw);
                    }
                    continue;
                }
                // No `result` → fall through to abstract keying below.
            }

            // Try "id" first. An array-valued id means the def is registered
            // under *every* element (CDDA multi-id defs like
            // `"id": ["corpse_bowels_neck_right", ..., "corpse_bowels_empty_edge"]`),
            // and any of them may be a `copy-from` target.
            if let Some(idv) = raw.value.get("id") {
                if let Some(ids) = all_ids_from_value(idv) {
                    for id in ids {
                        map.insert(id, raw);
                    }
                }
            } else if let Some(result) = raw.value.get("result").and_then(|v| v.as_str()) {
                // Fallback for recipes and similar types
                map.insert(result.to_string(), raw);
            } else if let Some(abstract_id) = raw.value.get("abstract").and_then(|v| v.as_str()) {
                // Some defs use "abstract" as the ID for abstract base defs
                map.insert(abstract_id.to_string(), raw);
            } else if let Some(raw_id) = &raw.id {
                map.insert(raw_id.clone(), raw);
            }
        }

        map
    }

    /// Extract the def ID from a resolved JSON value for insertion into the registry.
    ///
    /// Tries "id" first (supports string or array), then "result" (for recipes),
    /// then "abstract".
    fn extract_def_id(resolved: &Value) -> Option<String> {
        resolved
            .get("id")
            .and_then(Self::first_id_from_value)
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
            // Supports both string and array "id" fields.
            let has_id = resolved_value
                .get("id")
                .and_then(Self::first_id_from_value)
                .is_some();
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
                    // Extract the final ID (may differ from the map key for
                    // array-valued ids etc.). Recipes are the exception: their
                    // registry identity MUST be the composite raw-map key
                    // (`result` + `id_suffix`/`variant`) — falling back to the
                    // bare `result` here let different recipes producing the
                    // same item overwrite each other.
                    let final_id = if type_name == "recipe" {
                        def_key.to_string()
                    } else {
                        Self::extract_def_id(&normalized_value)
                            .unwrap_or_else(|| def_key.to_string())
                    };

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

    /// Resolve a single type's raw definitions to their **final resolved raw
    /// JSON** without deserializing into typed structs.
    ///
    /// This is the lossless Phase-A seam used by the round-trip test: it runs
    /// exactly the same copy-from / abstract resolution as
    /// [`resolve_type_with_pipeline`](Self::resolve_type_with_pipeline), but
    /// stops *before* `serde_json::from_value`, returning each def's final
    /// `Value`. A consumer can `from_value::<DefRaw>` then re-serialize and
    /// compare back to this value to prove parse→unparse is lossless.
    ///
    /// Returns `(id, resolved_json)` pairs, sorted by id. Defs that fail
    /// resolution (missing copy-from parent / circular) are skipped and
    /// returned as `Err(id)` in the second element, so callers can report them
    /// but still verify the resolvable ones.
    pub fn resolve_type_raw(&self, type_name: &str) -> (Vec<(String, Value)>, Vec<String>) {
        let raw_map = self.build_raw_map(type_name);
        if raw_map.is_empty() {
            return (Vec::new(), Vec::new());
        }

        // Topologically sort by copy-from dependency.
        let sorted_ids = match resolve::topological_sort(&raw_map) {
            Ok(ids) => ids,
            Err(cycles) => {
                for cycle in &cycles {
                    warn!(
                        "{:?}: circular copy-from dependency {:?} (round-trip will skip)",
                        type_name, cycle
                    );
                }
                raw_map.keys().map(|k| k.as_str()).collect::<Vec<_>>()
            }
        };

        let mut resolved: Vec<(String, Value)> = Vec::new();
        let mut failures: Vec<String> = Vec::new();

        for &def_key in &sorted_ids {
            let mut chain = Vec::new();
            let resolved_value = match resolve::resolve_copy_from(def_key, &raw_map, &mut chain) {
                Ok(v) => v,
                Err(_) => {
                    failures.push(def_key.to_string());
                    continue;
                }
            };

            // Skip abstract templates (matching resolve_type_with_pipeline).
            let abstract_bool = resolved_value
                .get("abstract_")
                .or_else(|| resolved_value.get("abstract"));
            if matches!(abstract_bool, Some(Value::Bool(true))) {
                continue;
            }

            // Promote `"abstract": "name"` to `id` (mirrors normalize step).
            let has_id = resolved_value
                .get("id")
                .and_then(Self::first_id_from_value)
                .is_some();
            let normalized = if !has_id {
                if let Some(abs_id) = resolved_value.get("abstract").and_then(|v| v.as_str()) {
                    let mut obj = resolved_value.as_object().cloned().unwrap_or_default();
                    obj.insert("id".to_string(), Value::String(abs_id.to_string()));
                    Value::Object(obj)
                } else {
                    resolved_value
                }
            } else {
                resolved_value
            };

            let id = def_key.to_string();
            resolved.push((id, normalized));
        }

        resolved.sort_by(|a, b| a.0.cmp(&b.0));
        (resolved, failures)
    }

    /// Resolve every def in a category and return `(id, resolved_value,
    /// copy_from_parent)` triples. Unlike [`resolve_type_raw`](Self::resolve_type_raw)
    /// the returned tuples keep the *raw* `copy-from` linkage so callers (like
    /// the Part-B export bridge) can reconstruct parent→child override chains
    /// even though resolution strips the `copy-from` key from the resolved value.
    ///
    /// The `copy_from_parent` is `Some(name)` only for defs whose *raw* def
    /// references an in-category parent; parentless defs (and abstract templates
    /// which never reach the registry) yield `None`.
    pub fn resolve_type_raw_with_parent(
        &self,
        type_name: &str,
    ) -> (Vec<(String, Value, Option<String>)>, Vec<String>) {
        let (items, failures) = self.resolve_type_raw(type_name);
        let raw_map = self.build_raw_map(type_name);
        let linked = items
            .into_iter()
            .map(|(id, value)| {
                let parent = raw_map
                    .get(&id)
                    .and_then(|raw| {
                        raw.get("copy-from")
                            .or_else(|| raw.get("copy_from"))
                            .and_then(serde_json::Value::as_str)
                    })
                    .map(str::to_string)
                    // Only surface the link when the parent actually resolves
                    // (i.e. exists in the raw map / build_raw_map handles it).
                    .filter(|p| raw_map.contains_key(p));
                (id, value, parent)
            })
            .collect();
        (linked, failures)
    }

    /// Pass 2: resolve all raw definitions into typed structs.
    fn resolve_all(&self, errors: &mut Vec<LoaderError>) -> DefRegistry {
        let mut registry = DefRegistry::empty();

        // Drive every standard category from the single `for_each_raw_def_kind!`
        // table. Each row expands to one `resolve_type_with_pipeline` call:
        //   resolve_type_with_pipeline::<DefType>(json_type, &mut registry.field, errors)
        macro_rules! resolve_one {
            ($name:ident, $def_ty:ty, $json:expr, $field:ident, $strategy:ident) => {
                self.resolve_type_with_pipeline::<$def_ty>($json, &mut registry.$field, errors);
            };
        }
        for_each_raw_def_kind!(call resolve_one);

        // ---- Mapgen (deferred: String-keyed, special-cased, not in the table) ----
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
    ///
    /// The handled-type set is derived from the `for_each_raw_def_kind!` table
    /// (one `json_type` string per category) rather than re-listed by hand, so
    /// adding a category can never silently desync the skip log.
    fn log_skipped_types(&self) {
        let mut handled_types: std::collections::HashSet<&str> = std::collections::HashSet::new();
        macro_rules! collect_type {
            ($name:ident, $def_ty:ty, $json:expr, $field:ident, $strategy:ident) => {
                handled_types.insert($json);
            };
        }
        for_each_raw_def_kind!(call collect_type);
        // `mapgen` is intentionally handled (its own resolver) though abstract.
        handled_types.insert("mapgen");

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
    fn resolve_mapgen(&self, registry: &mut DefRegistry, errors: &mut Vec<LoaderError>) {
        let type_name = "mapgen";
        let raw_map = self.build_raw_map(type_name);

        if raw_map.is_empty() {
            return;
        }

        // Topological sort by copy-from
        let sorted_ids = match resolve::topological_sort(&raw_map) {
            Ok(ids) => ids,
            Err(cycles) => {
                for cycle in &cycles {
                    errors.push(LoaderError::CircularCopyFrom {
                        chain: cycle.clone(),
                    });
                }
                raw_map.keys().map(|k| k.as_str()).collect::<Vec<_>>()
            }
        };

        let mut loaded_omt = 0;
        let mut loaded_nested = 0;

        for &def_key in &sorted_ids {
            let mut chain = Vec::new();

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

            match serde_json::from_value::<MapgenDef>(resolved_value) {
                Ok(def) => {
                    if let Some(ref omt) = def.om_terrain {
                        match omt {
                            MapgenTarget::Single(omt_id) => {
                                let vec: &mut Vec<Arc<MapgenDef>> =
                                    registry.mapgen.entry(omt_id.to_string()).or_default();
                                vec.push(Arc::new(def));
                                loaded_omt += 1;
                            }
                            MapgenTarget::Multi(omt_ids) => {
                                for omt_id in omt_ids {
                                    let vec: &mut Vec<Arc<MapgenDef>> =
                                        registry.mapgen.entry(omt_id.to_string()).or_default();
                                    vec.push(Arc::new(def.clone()));
                                }
                                loaded_omt += 1;
                            }
                        }
                    } else if let Some(ref nested_id) = def.nested_mapgen_id {
                        registry
                            .nested_mapgen
                            .insert(nested_id.clone(), Arc::new(def));
                        loaded_nested += 1;
                    }
                }
                Err(e) => {
                    errors.push(LoaderError::JsonParse {
                        path: PathBuf::from(type_name),
                        detail: format!("Failed to deserialize {} '{}': {}", type_name, def_key, e),
                    });
                }
            }
        }

        info!(
            "Loaded {} OMT mapgen + {} nested mapgen ({} total)",
            loaded_omt,
            loaded_nested,
            loaded_omt + loaded_nested
        );
    }
}

/// Return all string ids for a JSON "id" field (which may be a single string
/// or an array of strings). Returns `Some(vec)` when the value is a string or
/// array of strings, `None` otherwise.
fn all_ids_from_value(v: &Value) -> Option<Vec<String>> {
    match v {
        Value::String(s) => Some(vec![s.clone()]),
        Value::Array(arr) => {
            let ids: Vec<String> = arr
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(String::from)
                .collect();
            if ids.is_empty() {
                None
            } else {
                Some(ids)
            }
        }
        _ => None,
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
            id: Some("glock".into()),
            value: json!({"type": "GUN", "id": "glock"}),
            source: PathBuf::from("guns.json"),
        };
        let raw_item = RawDef {
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
                    id: Some("alpha".into()),
                    value: json!({"id": "alpha", "volume": "250 ml"}),
                    source: PathBuf::new(),
                },
                RawDef {
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
    /// Recipe registry identity is the COMPOSITE key (`result` + suffix /
    /// variant), never the bare `result`: two recipes producing the same item
    /// must not overwrite each other.
    #[test]
    fn recipe_variant_identity_does_not_collapse() {
        let dir = std::env::temp_dir().join(format!("cdda_recipe_id_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("recipes.json"),
            r#"[
                {"type": "recipe", "result": "meat_cooked", "time": 10, "category": "CC_COOKING"},
                {"type": "recipe", "result": "meat_cooked", "id_suffix": "boil", "time": 20, "category": "CC_COOKING"},
                {"type": "recipe", "result": "meat_cooked", "variant": "smoke", "time": 30, "category": "CC_COOKING"}
            ]"#,
        )
        .unwrap();

        let mut loader = Loader::new(vec![dir.clone()]);
        let registry = loader.load().expect("loads");
        assert_eq!(
            registry.recipes.len(),
            3,
            "three distinct recipe identities survive (base + suffix + variant)"
        );
        assert!(registry
            .recipes
            .contains_key(&"meat_cooked".to_string().into()));
        assert!(registry
            .recipes
            .contains_key(&"meat_cooked_boil".to_string().into()));
        assert!(registry
            .recipes
            .contains_key(&"meat_cooked_smoke".to_string().into()));

        let _ = std::fs::remove_dir_all(&dir);
    }

    fn build_raw_map_last_write_wins() {
        // Arrange
        let mut loader = Loader::new(vec![]);
        loader.raw_by_type.insert(
            "test_type".into(),
            vec![
                RawDef {
                    id: Some("dupe".into()),
                    value: json!({"id": "dupe", "volume": "100 ml"}),
                    source: PathBuf::from("first.json"),
                },
                RawDef {
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

    // -----------------------------------------------------------------------
    // build_raw_map — recipe composite ids and array multi-ids
    // -----------------------------------------------------------------------

    fn loader_with_items(type_name: &str, defs: Vec<Value>) -> Loader {
        let mut loader = Loader::new(vec![]);
        let raws = defs
            .into_iter()
            .map(|value| RawDef {
                id: value
                    .get("id")
                    .and_then(serde_json::Value::as_str)
                    .map(|s| s.to_string()),
                value,
                source: PathBuf::from("test.json"),
            })
            .collect();
        loader.raw_by_type.insert(type_name.to_string(), raws);
        loader
    }

    #[test]
    fn recipe_keyed_by_composite_id() {
        let defs = vec![json!({
            "type": "recipe",
            "result": "herbal_tea",
            "id_suffix": "from_tea_bag"
        })];
        let loader = loader_with_items("recipe", defs);
        let map = loader.build_raw_map("recipe");
        assert!(
            map.contains_key("herbal_tea_from_tea_bag"),
            "composite key missing"
        );
    }

    #[test]
    fn recipe_keyed_by_variant_composite_id() {
        let defs = vec![json!({
            "type": "recipe",
            "result": "deck_of_cards",
            "variant": "deck_of_cards_makeshift"
        })];
        let loader = loader_with_items("recipe", defs);
        let map = loader.build_raw_map("recipe");
        assert!(map.contains_key("deck_of_cards_deck_of_cards_makeshift"));
    }

    #[test]
    fn recipe_abstract_base_is_keyed() {
        let defs = vec![json!({
            "type": "recipe",
            "abstract": "seed_extraction_base"
        })];
        let loader = loader_with_items("recipe", defs);
        let map = loader.build_raw_map("recipe");
        assert!(map.contains_key("seed_extraction_base"));
    }

    #[test]
    fn recipe_variant_self_copy_does_not_clobber() {
        let base = json!({"type": "recipe", "result": "apron_cotton", "time": "3 h"});
        let variant = json!({
            "type": "recipe",
            "result": "apron_cotton",
            "variant": "maid_apron",
            "copy-from": "apron_cotton"
        });
        let loader = loader_with_items("recipe", vec![base.clone(), variant]);
        let map = loader.build_raw_map("recipe");
        assert_eq!(
            map["apron_cotton"]
                .get("time")
                .and_then(serde_json::Value::as_str),
            Some("3 h")
        );
    }

    #[test]
    fn array_id_registers_all_elements() {
        let defs = vec![json!({
            "type": "overmap_terrain",
            "id": ["corpse_bowels_neck_right", "corpse_bowels_empty_edge"]
        })];
        let loader = loader_with_items("overmap_terrain", defs);
        let map = loader.build_raw_map("overmap_terrain");
        assert!(map.contains_key("corpse_bowels_neck_right"));
        assert!(map.contains_key("corpse_bowels_empty_edge"));
    }
}
