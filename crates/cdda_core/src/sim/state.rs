//! # Game state machine
//!
//! `AppState` drives the lifecycle. The app starts in `MainMenu` (no data
//! loaded, no systems wired). After character creation and world setup the
//! player confirms, which transitions to `DataLoading` → `WorldGen` → `InGame`.
//!
//! This mirrors CDDA's flow: you create a world, customize it with mods,
//! create a character, and only THEN does the engine load JSON and generate
//! the world.

use bevy_ecs::prelude::*;
use bevy_state::prelude::*;

// ---------------------------------------------------------------------------
// AppState — lifecycle
// ---------------------------------------------------------------------------

/// Top-level lifecycle state for the game.
#[derive(States, Default, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum AppState {
    /// Main menu and all pre-game UI (world creation, character creation).
    /// No JSON loaded, no simulation systems active.
    #[default]
    MainMenu,
    /// Loading JSON files and building the definition registry.
    /// Triggered when the player confirms "Start Game" after character creation.
    DataLoading,
    /// Generating the overmap, placing the player's starting position.
    WorldGen,
    /// Main game loop — player moves, monsters act, simulation ticks.
    InGame,
    /// Game is paused (e.g. in-game menu open).
    Paused,
    /// Game over — player character has died.
    GameOver,
}

// ---------------------------------------------------------------------------
// TurnState — phase of the game tick loop
// ---------------------------------------------------------------------------

/// The phase of the game tick loop.
/// Checked by the main tick system to decide which sub-systems run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Resource)]
pub enum TurnState {
    WaitingForInput,
    PlayerActed,
    Simulating,
    Animating,
}

// ---------------------------------------------------------------------------
// GameTime — in-game clock
// ---------------------------------------------------------------------------

/// Re-exported from `cdda_components::sim`.
pub use cdda_components::sim::GameTime;

// ---------------------------------------------------------------------------
// LoadingStatus
// ---------------------------------------------------------------------------

#[derive(Resource, Debug, Clone)]
pub struct LoadingStatus {
    pub total_files: usize,
    pub loaded_files: usize,
    pub total_defs: usize,
    pub current_phase: String,
}

impl Default for LoadingStatus {
    fn default() -> Self {
        Self {
            total_files: 0,
            loaded_files: 0,
            total_defs: 0,
            current_phase: "Initialising".into(),
        }
    }
}

impl LoadingStatus {
    pub fn progress_pct(&self) -> f32 {
        if self.total_files == 0 {
            0.0
        } else {
            self.loaded_files as f32 / self.total_files as f32
        }
    }
}

// ---------------------------------------------------------------------------
// StartupConfig — built during pre-game UI, consumed by data loading
// ---------------------------------------------------------------------------

#[derive(Resource, Debug, Clone)]
pub struct StartupConfig {
    pub data_dirs: Vec<std::path::PathBuf>,
    pub mod_ids: Vec<String>,
    pub scenario_id: String,
    pub profession_id: String,
    pub world_name: String,
    pub world_seed: u64,
}

impl Default for StartupConfig {
    fn default() -> Self {
        Self {
            data_dirs: vec![std::path::PathBuf::from("data/core")],
            mod_ids: Vec::new(),
            scenario_id: "evacuee".into(),
            profession_id: "unemployed".into(),
            world_name: "New World".into(),
            world_seed: 0,
        }
    }
}
