//! # Game state machine
//!
//! `AppState` drives the lifecycle: DataLoading → WorldGen → InGame.
//! `GameTime` tracks in-game elapsed time.
//! `LoadingStatus` reports progress during the loading state.
//!
//! ## Design note
//! We use a custom `Resource`-based state instead of `bevy::state::States`
//! because `cdda_sim` depends only on `bevy_ecs`/`bevy_reflect`, not the
//! full `bevy` crate (which is required for `States`).

use bevy_ecs::prelude::*;

// ---------------------------------------------------------------------------
// AppState — lifecycle
// ---------------------------------------------------------------------------

/// Top-level lifecycle state for the game.
#[derive(Resource, Default, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum AppState {
    /// Loading JSON files and building the definition registry.
    #[default]
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
// GameTime — in-game clock
// ---------------------------------------------------------------------------

/// How many game turns have elapsed since the start of the session.
///
/// Each turn = ~6 real-time seconds in CDDA convention.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameTime {
    pub turn: u64,
}

impl Default for GameTime {
    fn default() -> Self {
        Self { turn: 0 }
    }
}

impl GameTime {
    /// Advance by one turn.
    pub fn advance(&mut self) {
        self.turn += 1;
    }

    /// Approximate hours elapsed (1 turn = 6 seconds).
    pub fn hours_elapsed(&self) -> u64 {
        (self.turn * 6) / 3600
    }

    /// Approximate turns per day (24h / 6s).
    pub const TURNS_PER_DAY: u64 = 14400;
}

/// Reports the progress of the JSON loading phase.
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

/// Configuration passed to the loading and world-gen systems.
#[derive(Resource, Debug, Clone)]
pub struct StartupConfig {
    /// Paths to directories containing CDDA JSON data.
    pub data_dirs: Vec<std::path::PathBuf>,
    /// Which scenario to start with.
    pub scenario_id: String,
    /// Which profession to start with.
    pub profession_id: String,
}

impl Default for StartupConfig {
    fn default() -> Self {
        Self {
            data_dirs: vec![std::path::PathBuf::from("data/core")],
            scenario_id: "evacuee".into(),
            profession_id: "unemployed".into(),
        }
    }
}
