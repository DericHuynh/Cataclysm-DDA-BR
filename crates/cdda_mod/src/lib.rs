//! # cdda_mod — Mod management
//!
//! Mod metadata, dependency resolution, and conflict detection.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

// ---------------------------------------------------------------------------
// ModInfo
// ---------------------------------------------------------------------------

/// Unique string identifier for a mod (matches `"id"` in `modinfo.json`).
pub type ModId = String;

/// Metadata about a single installed mod.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModInfo {
    pub id: ModId,
    pub name: String,
    /// Mods that must be loaded before this one.
    pub dependencies: Vec<ModId>,
    /// Mods that cannot be active at the same time as this one.
    pub conflicts: Vec<ModId>,
}

impl ModInfo {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            dependencies: Vec::new(),
            conflicts: Vec::new(),
        }
    }

    pub fn with_dependency(mut self, dep: impl Into<String>) -> Self {
        self.dependencies.push(dep.into());
        self
    }

    pub fn with_conflict(mut self, other: impl Into<String>) -> Self {
        self.conflicts.push(other.into());
        self
    }
}

// ---------------------------------------------------------------------------
// ModError
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ModError {
    #[error("Circular dependency involving: {0}")]
    CircularDependency(String),

    #[error("Unknown dependency '{dep}' required by '{requirer}'")]
    UnknownDependency { dep: String, requirer: String },

    #[error("Conflict: '{a}' and '{b}' cannot be active together")]
    Conflict { a: String, b: String },
}

// ---------------------------------------------------------------------------
// check_dependencies
// ---------------------------------------------------------------------------

/// Verify that every mod's dependencies are present in the active set.
pub fn check_dependencies(mods: &[ModInfo]) -> Result<(), ModError> {
    let available: HashSet<&str> = mods.iter().map(|m| m.id.as_str()).collect();
    for m in mods {
        for dep in &m.dependencies {
            if !available.contains(dep.as_str()) {
                return Err(ModError::UnknownDependency {
                    dep: dep.clone(),
                    requirer: m.id.clone(),
                });
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// check_conflicts
// ---------------------------------------------------------------------------

/// Verify that no active mod conflicts with another active mod.
pub fn check_conflicts(mods: &[ModInfo]) -> Result<(), ModError> {
    let active: HashSet<&str> = mods.iter().map(|m| m.id.as_str()).collect();
    for m in mods {
        for conflict in &m.conflicts {
            if active.contains(conflict.as_str()) {
                return Err(ModError::Conflict {
                    a: m.id.clone(),
                    b: conflict.clone(),
                });
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// resolve_load_order
// ---------------------------------------------------------------------------

/// Topological sort of mods by dependency — dependencies come before dependents.
///
/// Returns `ModError::UnknownDependency` if a dep isn't in the slice.
/// Returns `ModError::CircularDependency` if the dependency graph has a cycle.
pub fn resolve_load_order(mods: &[ModInfo]) -> Result<Vec<ModId>, ModError> {
    check_dependencies(mods)?;

    // Kahn's algorithm.
    let mut in_degree: HashMap<&str, usize> = mods.iter().map(|m| (m.id.as_str(), 0)).collect();
    let mut dependents: HashMap<&str, Vec<&str>> =
        mods.iter().map(|m| (m.id.as_str(), Vec::new())).collect();

    for m in mods {
        for dep in &m.dependencies {
            if let Some(list) = dependents.get_mut(dep.as_str()) {
                list.push(m.id.as_str());
            }
            *in_degree.entry(m.id.as_str()).or_insert(0) += 1;
        }
    }

    // Deterministic start: sort zero-in-degree nodes.
    let mut queue: VecDeque<&str> = {
        let mut roots: Vec<&str> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();
        roots.sort_unstable();
        roots.into()
    };

    let mut result: Vec<ModId> = Vec::new();
    while let Some(id) = queue.pop_front() {
        result.push(id.to_string());
        if let Some(deps) = dependents.get(id) {
            let mut ready: Vec<&str> = deps
                .iter()
                .filter_map(|&dep| {
                    let deg = in_degree.get_mut(dep)?;
                    *deg -= 1;
                    (*deg == 0).then_some(dep)
                })
                .collect();
            ready.sort_unstable();
            queue.extend(ready);
        }
    }

    if result.len() != mods.len() {
        return Err(ModError::CircularDependency("cycle detected".into()));
    }

    Ok(result)
}
