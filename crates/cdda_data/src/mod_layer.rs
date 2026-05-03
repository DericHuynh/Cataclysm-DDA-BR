//! Mod loading and layering.
//!
//! CDDA mods use the identical JSON format as core data. Mods define their
//! own definitions which are merged on top of the core registry, with
//! last-write-wins semantics for conflicting IDs.
//!
//! Mod layering works as follows:
//! 1. The core `DefRegistry` is loaded first.
//! 2. Each mod is loaded in dependency order.
//! 3. Mod definitions override core definitions by ID.
//! 4. Overrides at the field level use the same `copy-from` resolution
//!    mechanism as regular definitions.
//!
//! For Stage 1, this module provides a simplified merge-on-ID approach.
//! Full field-level merge with conflict detection is Stage 2.

use crate::registry::DefRegistry;
use crate::Loader;
use std::path::PathBuf;

/// A loaded mod.
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

/// Result of loading and merging a mod.
#[derive(Debug)]
pub struct ModLoadResult {
    /// The mod that was loaded.
    pub mod_info: ModInfo,
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

    #[error("Load error: {0}")]
    LoadError(String),
}

/// Manage mod loading, dependency resolution, and merging.
pub struct ModManager {
    /// Available mods discovered in data/mods/.
    #[allow(dead_code)]
    available: Vec<ModInfo>,
    /// The base core registry.
    #[allow(dead_code)]
    core_registry: DefRegistry,
}

impl ModManager {
    /// Create a new mod manager with the given core registry.
    pub fn new(core_registry: DefRegistry) -> Self {
        ModManager {
            available: Vec::new(),
            core_registry,
        }
    }

    /// Scan a directory for available mods.
    pub fn scan_mods(&mut self, _mods_dir: &PathBuf) -> Result<(), ModError> {
        // TODO: Scan mods directory for modinfo.json / mod.json files
        // and populate self.available.
        //
        // For Stage 1, this is a no-op.
        Ok(())
    }

    /// Resolve and load mods in dependency order.
    ///
    /// Returns a list of load results in load order, with the final registry
    /// being the last result's registry.
    pub fn load_mods(&self, _mod_ids: &[String]) -> Result<Vec<ModLoadResult>, ModError> {
        // TODO: Topological sort by dependencies, then load each mod's
        // JSON directory and merge definitions into the registry.
        //
        // For Stage 1, this is a no-op that returns the core registry unchanged.
        Ok(Vec::new())
    }

    /// Check if two mods conflict.
    pub fn check_conflicts(&self, _enabled: &[String]) -> Result<(), ModError> {
        // TODO: Check conflict lists for pairs of enabled mods.
        Ok(())
    }
}

/// Quick helper: load core data + a list of mods.
pub fn load_with_mods(
    core_dirs: Vec<PathBuf>,
    mod_dirs: Vec<PathBuf>,
    mod_ids: &[String],
) -> Result<(DefRegistry, Vec<ModLoadResult>), Vec<super::loader::LoaderError>> {
    let mut loader = Loader::new(core_dirs);
    let mut errors = Vec::new();

    // Load core definitions
    let core_registry = match loader.load() {
        Ok(r) => r,
        Err(e) => return Err(e),
    };

    if mod_ids.is_empty() {
        return Ok((core_registry, Vec::new()));
    }

    // Build mod manager and load mods
    let mut mgr = ModManager::new(core_registry.clone());
    for dir in mod_dirs {
        if let Err(e) = mgr.scan_mods(&dir) {
            errors.push(super::loader::LoaderError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            )));
            return Err(errors);
        }
    }

    match mgr.load_mods(mod_ids) {
        Ok(results) => {
            let final_registry = results
                .last()
                .map(|r| r.registry.clone())
                .unwrap_or(core_registry);
            Ok((final_registry, results))
        }
        Err(e) => {
            errors.push(super::loader::LoaderError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            )));
            Err(errors)
        }
    }
}
