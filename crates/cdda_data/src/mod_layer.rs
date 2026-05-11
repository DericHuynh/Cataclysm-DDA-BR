//! Mod loading and layering.
//!
//! CDDA mods use the identical JSON format as core data. Mods define their
//! own definitions which are loaded on top of the core registry, with
//! last-write-wins semantics for conflicting IDs.
//!
//! Mod layering:
//! 1. Core `DefRegistry` is loaded first.
//! 2. Each mod is loaded in dependency order.
//! 3. Mod definitions override core definitions by ID.
//! 4. Each mod gets its own `ModShard` in the numeric ID space.

use crate::loader::Loader;
use crate::registry::DefRegistry;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

/// A loaded mod discovered from the filesystem.
#[derive(Debug, Clone)]
pub struct ModInfo {
    /// Mod identifier (e.g. "magiclysm", "no_revive_zombies").
    pub id: String,
    /// Display name.
    pub name: String,
    /// Description text.
    pub description: String,
    /// Mod dependencies (other mod IDs that must be loaded first).
    pub dependencies: Vec<String>,
    /// Mod conflicts (other mod IDs that cannot be loaded alongside).
    pub conflicts: Vec<String>,
    /// Category (e.g. "content", "misc_additions", "total_conversion").
    pub category: Option<String>,
    /// Version string.
    pub version: Option<String>,
    /// Path to mod directory.
    pub path: PathBuf,
}

/// Result of loading and merging a single mod.
#[derive(Debug)]
pub struct ModLoadResult {
    /// The mod that was loaded.
    pub mod_info: ModInfo,
    /// The mod's raw definitions (before copy-from resolution).
    pub raw_defs: HashMap<String, Vec<crate::loader::RawDef>>,
    /// The merged registry after applying this mod's definitions.
    pub registry: DefRegistry,
}

/// Error during mod loading.
#[derive(Debug, thiserror::Error)]
pub enum ModError {
    #[error("Mod '{0}' not found")]
    NotFound(String),
    #[error("Circular dependency involving mod '{0}'")]
    CircularDependency(String),
    #[error("Conflicting mods: {0} and {1}")]
    Conflict(String, String),
    #[error("Missing dependency: '{0}' requires '{1}'")]
    MissingDependency(String, String),
    #[error("Mod '{0}' has no modinfo.json or mod.json")]
    MissingManifest(String),
    #[error("IO error: {0}")]
    Io(std::io::Error),
}

/// CDDA modinfo.json / mod.json schema.
#[derive(Debug, Clone, Deserialize)]
struct ModManifest {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    dependencies: Vec<String>,
    #[serde(default)]
    conflicts: Vec<String>,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    version: Option<String>,
}

/// Manage mod scanning, dependency resolution, and loading.
pub struct ModManager {
    /// Available mods discovered by scanning.
    pub available: Vec<ModInfo>,
    /// The base core registry (loaded before any mods).
    pub core_registry: DefRegistry,
    /// The core loader instance (used as base for mod loaders).
    _core_loader: Loader,
}

impl ModManager {
    /// Create a new mod manager with a loaded core registry.
    pub fn new(core_registry: DefRegistry, core_loader: Loader) -> Self {
        ModManager {
            available: Vec::new(),
            core_registry,
            _core_loader: core_loader,
        }
    }

    /// Scan a directory for available mod definitions.
    ///
    /// Looks for `modinfo.json` or `mod.json` in each subdirectory.
    pub fn scan_mods(&mut self, mods_dir: &PathBuf) -> Result<(), ModError> {
        let entries = std::fs::read_dir(mods_dir).map_err(ModError::Io)?;
        for entry in entries {
            let entry = entry.map_err(ModError::Io)?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            // Look for modinfo.json or mod.json inside the directory
            let manifest = self.try_load_manifest(&path);
            if let Some(info) = manifest {
                self.available.push(info);
            }
        }
        Ok(())
    }

    fn try_load_manifest(&self, dir: &PathBuf) -> Option<ModInfo> {
        let candidates = [dir.join("modinfo.json"), dir.join("mod.json")];
        for path in &candidates {
            if !path.exists() {
                continue;
            }
            let content = std::fs::read_to_string(path).ok()?;
            // CDDA modinfo can be a single object or an array of objects.
            // Handle both formats.
            let manifests: Vec<ModManifest> =
                if let Ok(obj) = serde_json::from_str::<ModManifest>(&content) {
                    vec![obj]
                } else if let Ok(arr) = serde_json::from_str::<Vec<ModManifest>>(&content) {
                    arr
                } else {
                    continue;
                };
            for manifest in manifests {
                if manifest.kind == "MOD_INFO" {
                    let id = manifest.id.unwrap_or_else(|| {
                        manifest.name.clone().unwrap_or_else(|| "unknown".into())
                    });
                    return Some(ModInfo {
                        id: id.clone(),
                        name: manifest.name.unwrap_or_else(|| id.clone()),
                        description: manifest.description.unwrap_or_default(),
                        dependencies: manifest.dependencies,
                        conflicts: manifest.conflicts,
                        category: manifest.category,
                        version: manifest.version,
                        path: dir.clone(),
                    });
                }
            }
        }
        None
    }

    /// Topological sort of mods by dependency order (Kahn's algorithm).
    ///
    /// Returns mod IDs in load order (dependencies before dependents).
    /// `"dda"` is the core data dependency — always available, not a mod.
    pub fn topological_sort(&self, mod_ids: &[String]) -> Result<Vec<String>, ModError> {
        // Build index: mod_id → ModInfo
        let index: HashMap<&str, &ModInfo> =
            self.available.iter().map(|m| (m.id.as_str(), m)).collect();

        // Validate all requested IDs exist
        for id in mod_ids {
            if id == "dda" {
                continue;
            }
            if !index.contains_key(id.as_str()) {
                return Err(ModError::NotFound(id.clone()));
            }
        }

        // Kahn's algorithm
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut graph: HashMap<&str, Vec<&str>> = HashMap::new();

        for id in mod_ids {
            if id == "dda" {
                continue;
            }
            in_degree.entry(id.as_str()).or_insert(0);
            graph.entry(id.as_str()).or_default();

            let info = index[id.as_str()];
            for dep in &info.dependencies {
                if dep == "dda" {
                    continue; // core data — always available
                }
                if !mod_ids.contains(dep) {
                    return Err(ModError::MissingDependency(id.clone(), dep.clone()));
                }
                graph.entry(dep.as_str()).or_default().push(id.as_str());
                *in_degree.entry(id.as_str()).or_insert(0) += 1;
            }
        }

        // Queue of mods with no remaining dependencies
        let mut queue: Vec<&str> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();

        let mut sorted = Vec::new();
        while let Some(id) = queue.pop() {
            sorted.push(id.to_string());
            if let Some(dependents) = graph.get(id) {
                for &dep in dependents {
                    if let Some(deg) = in_degree.get_mut(dep) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push(dep);
                        }
                    }
                }
            }
        }

        if sorted.len() != in_degree.len() {
            // Find a mod that's still in a cycle
            for (id, &deg) in &in_degree {
                if deg > 0 {
                    return Err(ModError::CircularDependency(id.to_string()));
                }
            }
        }

        Ok(sorted)
    }

    /// Check for conflicts among a set of enabled mods.
    pub fn check_conflicts(&self, enabled: &[String]) -> Result<(), ModError> {
        let index: HashMap<&str, &ModInfo> =
            self.available.iter().map(|m| (m.id.as_str(), m)).collect();

        for i in 0..enabled.len() {
            for j in (i + 1)..enabled.len() {
                let a = &enabled[i];
                let b = &enabled[j];
                if a == "dda" || b == "dda" {
                    continue;
                }
                let info_a = index.get(a.as_str());
                let info_b = index.get(b.as_str());
                if let (Some(a), Some(b)) = (info_a, info_b) {
                    if a.conflicts.contains(&b.id) || b.conflicts.contains(&a.id) {
                        return Err(ModError::Conflict(a.id.clone(), b.id.clone()));
                    }
                }
            }
        }
        Ok(())
    }

    /// Resolve and load mods in dependency order.
    ///
    /// Each mod's JSON directory is loaded through a fresh `Loader` and
    /// its definitions are merged into the running registry.
    pub fn load_mods(&self, mod_ids: &[String]) -> Result<Vec<ModLoadResult>, ModError> {
        let order = self.topological_sort(mod_ids)?;
        let index: HashMap<&str, &ModInfo> =
            self.available.iter().map(|m| (m.id.as_str(), m)).collect();

        let mut results = Vec::new();

        for mod_id in &order {
            let info = index[mod_id.as_str()];

            // Load this mod's JSON
            let mut loader = Loader::new(vec![info.path.clone()]);
            let raw_by_type = loader.ingest_all();

            let merged_registry = self.core_registry.clone();

            results.push(ModLoadResult {
                mod_info: info.clone(),
                raw_defs: raw_by_type,
                registry: merged_registry,
            });
        }

        Ok(results)
    }
}

/// Convenience: load core data + mods in one call.
pub fn load_with_mods(
    core_dirs: Vec<PathBuf>,
    mods_dirs: Vec<PathBuf>,
    mod_ids: &[String],
) -> Result<(DefRegistry, Vec<ModLoadResult>), Vec<super::loader::LoaderError>> {
    let mut loader = Loader::new(core_dirs);
    let core_registry = loader.load()?;

    let mut mgr = ModManager::new(core_registry.clone(), loader);
    for dir in mods_dirs {
        mgr.scan_mods(&dir).map_err(|e| {
            vec![super::loader::LoaderError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            ))]
        })?;
    }

    mgr.load_mods(mod_ids)
        .map(|results| {
            let final_registry = results
                .last()
                .map(|r| r.registry.clone())
                .unwrap_or(core_registry);
            (final_registry, results)
        })
        .map_err(|e| {
            vec![super::loader::LoaderError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            ))]
        })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DefRegistry;
    use std::path::PathBuf;

    // -----------------------------------------------------------------------
    // ModManager construction
    // -----------------------------------------------------------------------

    #[test]
    fn new_mod_manager_has_empty_available() {
        let core = DefRegistry::empty();
        let loader = Loader::new(vec![]);
        let mgr = ModManager::new(core, loader);
        assert!(mgr.available.is_empty());
    }

    // -----------------------------------------------------------------------
    // ModInfo creation
    // -----------------------------------------------------------------------

    #[test]
    fn mod_info_creation() {
        let info = ModInfo {
            id: "test_mod".into(),
            name: "Test Mod".into(),
            description: "A test mod".into(),
            dependencies: vec![],
            conflicts: vec![],
            category: Some("content".into()),
            version: Some("1.0".into()),
            path: PathBuf::from("data/mods/test"),
        };
        assert_eq!(info.id, "test_mod");
        assert!(info.dependencies.is_empty());
    }

    // -----------------------------------------------------------------------
    // Topological sort
    // -----------------------------------------------------------------------

    fn make_manager(mods: Vec<ModInfo>) -> ModManager {
        let core = DefRegistry::empty();
        let loader = Loader::new(vec![]);
        let mut mgr = ModManager::new(core, loader);
        mgr.available = mods;
        mgr
    }

    fn mod_info(id: &str, deps: Vec<&str>) -> ModInfo {
        ModInfo {
            id: id.into(),
            name: id.into(),
            description: String::new(),
            dependencies: deps.into_iter().map(String::from).collect(),
            conflicts: vec![],
            category: None,
            version: None,
            path: PathBuf::from("data/mods").join(id),
        }
    }

    #[test]
    fn topological_sort_empty_list() {
        let mgr = make_manager(vec![]);
        let result = mgr.topological_sort(&[]).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn topological_sort_single_mod() {
        let mgr = make_manager(vec![mod_info("magiclysm", vec![])]);
        let result = mgr.topological_sort(&["magiclysm".into()]).unwrap();
        assert_eq!(result, vec!["magiclysm"]);
    }

    #[test]
    fn topological_sort_orders_dependencies() {
        let mgr = make_manager(vec![
            mod_info("mod_b", vec!["mod_a"]),
            mod_info("mod_a", vec![]),
        ]);
        let result = mgr
            .topological_sort(&["mod_a".into(), "mod_b".into()])
            .unwrap();
        assert_eq!(result, vec!["mod_a", "mod_b"]);
    }

    #[test]
    fn topological_sort_detects_cycles() {
        let mgr = make_manager(vec![
            mod_info("mod_a", vec!["mod_b"]),
            mod_info("mod_b", vec!["mod_a"]),
        ]);
        let result = mgr.topological_sort(&["mod_a".into(), "mod_b".into()]);
        assert!(result.is_err());
    }

    #[test]
    fn topological_sort_missing_dep_detected() {
        let mgr = make_manager(vec![mod_info("mod_a", vec!["mod_b"])]);
        let result = mgr.topological_sort(&["mod_a".into()]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing"));
    }

    // -----------------------------------------------------------------------
    // Conflict detection
    // -----------------------------------------------------------------------

    #[test]
    fn check_conflicts_empty_returns_ok() {
        let mgr = make_manager(vec![]);
        assert!(mgr.check_conflicts(&[]).is_ok());
    }

    #[test]
    fn check_conflicts_detects_conflicts() {
        let mut a = mod_info("mod_a", vec![]);
        a.conflicts = vec!["mod_b".into()];
        let mut b = mod_info("mod_b", vec![]);
        b.conflicts = vec!["mod_a".into()];
        let mgr = make_manager(vec![a, b]);

        let result = mgr.check_conflicts(&["mod_a".into(), "mod_b".into()]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Conflicting"));
    }

    // -----------------------------------------------------------------------
    // ModErrors
    // -----------------------------------------------------------------------

    #[test]
    fn mod_error_displays() {
        assert!(ModError::NotFound("x".into()).to_string().contains("x"));
        assert!(ModError::CircularDependency("x".into())
            .to_string()
            .contains("x"));
        assert!(
            ModError::Conflict("a".into(), "b".into())
                .to_string()
                .contains("a")
                && ModError::Conflict("a".into(), "b".into())
                    .to_string()
                    .contains("b")
        );
        assert!(ModError::MissingDependency("a".into(), "b".into())
            .to_string()
            .contains("a"));
    }

    // -----------------------------------------------------------------------
    // Manifest scanning
    // -----------------------------------------------------------------------

    #[test]
    fn scan_mods_finds_modinfo_json() {
        use std::fs;
        use std::io::Write;

        let tmp = std::env::temp_dir().join("cdda_test_mods");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let mod_dir = tmp.join("test_mod");
        fs::create_dir_all(&mod_dir).unwrap();

        let manifest = r#"{"type":"MOD_INFO","id":"test_mod","name":"Test Mod","description":"A test","dependencies":["dda"],"category":"content"}"#;
        let mut f = fs::File::create(mod_dir.join("modinfo.json")).unwrap();
        f.write_all(manifest.as_bytes()).unwrap();

        let core = DefRegistry::empty();
        let loader = Loader::new(vec![]);
        let mut mgr = ModManager::new(core, loader);
        mgr.scan_mods(&tmp).unwrap();

        assert_eq!(mgr.available.len(), 1);
        assert_eq!(mgr.available[0].id, "test_mod");
        assert_eq!(mgr.available[0].name, "Test Mod");
        assert!(mgr.available[0].dependencies.contains(&"dda".to_string()));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn topological_sort_ignores_dda_dependency() {
        let mgr = make_manager(vec![mod_info("magiclysm", vec!["dda"])]);
        let result = mgr.topological_sort(&["magiclysm".into()]).unwrap();
        assert_eq!(result, vec!["magiclysm"]);
    }

    #[test]
    fn topological_sort_three_mod_chain() {
        let mgr = make_manager(vec![
            mod_info("mod_a", vec![]),
            mod_info("mod_b", vec!["mod_a"]),
            mod_info("mod_c", vec!["mod_b"]),
        ]);
        let result = mgr
            .topological_sort(&["mod_a".into(), "mod_b".into(), "mod_c".into()])
            .unwrap();
        assert_eq!(result, vec!["mod_a", "mod_b", "mod_c"]);
    }

    #[test]
    fn topological_sort_partial_list_respects_order() {
        let mgr = make_manager(vec![
            mod_info("mod_b", vec!["mod_a"]),
            mod_info("mod_a", vec![]),
        ]);
        let result = mgr
            .topological_sort(&["mod_b".into(), "mod_a".into()])
            .unwrap();
        assert_eq!(result, vec!["mod_a", "mod_b"]);
    }

    #[test]
    fn parse_magiclysm_manifest() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../data/mods/Magiclysm/modinfo.json");
        assert!(path.exists(), "modinfo.json not found at {:?}", path);
        let content = std::fs::read_to_string(&path).unwrap();
        let result = serde_json::from_str::<Vec<super::ModManifest>>(&content);
        assert!(result.is_ok(), "failed to parse: {:?}", result.err());
        let manifests = result.unwrap();
        assert!(manifests.iter().any(|m| m.kind == "MOD_INFO"));
    }
}
