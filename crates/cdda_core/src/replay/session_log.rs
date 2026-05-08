//! Session log format — records a complete deterministic replay.
//!
//! Uses `GameAction` from `cdda_input`. Binary format is `postcard`,

use crate::input::{ActionSource, GameAction};
use bevy_ecs::prelude::Resource;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ActionRecord {
    pub turn: u64,
    pub action: GameAction,
    pub source: ActionSource,
}

#[derive(Resource, Serialize, Deserialize, Clone, Debug)]
pub struct SessionLog {
    pub world_seed: u64,
    pub actions: Vec<ActionRecord>,
    /// (turn, hash) — populated only in devtools builds.
    #[serde(default)]
    pub state_hashes: Vec<(u64, u64)>,
}

impl Default for SessionLog {
    fn default() -> Self {
        Self {
            world_seed: 0,
            actions: Vec::new(),
            state_hashes: Vec::new(),
        }
    }
}

impl SessionLog {
    pub fn new(world_seed: u64) -> Self {
        Self {
            world_seed,
            ..Default::default()
        }
    }
    pub fn len(&self) -> usize {
        self.actions.len()
    }
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, String> {
        postcard::to_allocvec(self).map_err(|e| format!("Serialize: {e}"))
    }
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        postcard::from_bytes(bytes).map_err(|e| format!("Deserialize: {e}"))
    }
    pub fn save_to_file(&self, path: &std::path::Path) -> Result<(), String> {
        std::fs::write(path, self.to_bytes()?).map_err(|e| format!("IO: {e}"))
    }
    pub fn load_from_file(path: &std::path::Path) -> Result<Self, String> {
        Self::from_bytes(&std::fs::read(path).map_err(|e| format!("IO: {e}"))?)
    }
    pub fn save_compressed(&self, path: &std::path::Path) -> Result<(), String> {
        let raw = self.to_bytes()?;
        let compressed = raw.clone();
        std::fs::write(path, compressed).map_err(|e| format!("IO: {e}"))
    }
    pub fn load_compressed(path: &std::path::Path) -> Result<Self, String> {
        let compressed = std::fs::read(path).map_err(|e| format!("IO: {e}"))?;
        Self::from_bytes(
            &compressed,
        )
    }
}
