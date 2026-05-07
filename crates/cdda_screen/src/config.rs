//! Configuration resources for settings, character creation, and world gen.

use bevy_ecs::prelude::Resource;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// GameSettings
// ---------------------------------------------------------------------------

/// Settings categories and values for the options menu.
#[derive(Resource, Debug, Clone)]
pub struct GameSettings {
    /// General settings
    pub auto_save_enabled: bool,
    pub auto_save_interval_minutes: u32,
    pub auto_note_enabled: bool,
    pub circular_distance: bool,
    /// Interface
    pub sidebar_style: String,
    pub show_compass: bool,
    pub pixel_minimap_height: u32,
    pub force_capital_yn: bool,
    /// Graphics
    pub terminal_width: u32,
    pub terminal_height: u32,
    pub font_size: u32,
    pub fullscreen: bool,
    /// Sound
    pub music_volume: u32,
    pub sfx_volume: u32,
    /// Debug
    pub debug_mode: bool,
    pub show_fps: bool,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            auto_save_enabled: true,
            auto_save_interval_minutes: 5,
            auto_note_enabled: true,
            circular_distance: false,
            sidebar_style: "classic".into(),
            show_compass: true,
            pixel_minimap_height: 100,
            force_capital_yn: true,
            terminal_width: 80,
            terminal_height: 25,
            font_size: 16,
            fullscreen: false,
            music_volume: 80,
            sfx_volume: 100,
            debug_mode: false,
            show_fps: false,
        }
    }
}

// ---------------------------------------------------------------------------
// CharacterCreationState
// ---------------------------------------------------------------------------

/// Character creation state — accumulates choices across screens.
#[derive(Resource, Debug, Clone)]
pub struct CharacterCreationState {
    pub scenario_id: String,
    pub profession_id: String,
    pub gender: String,
    pub name: String,
    pub strength: u32,
    pub dexterity: u32,
    pub intelligence: u32,
    pub perception: u32,
    pub unspent_points: u32,
    pub selected_traits: Vec<String>,
    pub selected_skills: HashMap<String, u32>,
    /// Which step: 0=scenario, 1=profession, 2=stats, 3=traits, 4=confirm
    pub step: u32,
}

impl Default for CharacterCreationState {
    fn default() -> Self {
        Self {
            scenario_id: "evacuee".into(),
            profession_id: "unemployed".into(),
            gender: "male".into(),
            name: String::new(),
            strength: 8,
            dexterity: 8,
            intelligence: 8,
            perception: 8,
            unspent_points: 6,
            selected_traits: Vec::new(),
            selected_skills: HashMap::new(),
            step: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// WorldCreationSettings
// ---------------------------------------------------------------------------

/// World creation configuration.
#[derive(Resource, Debug, Clone)]
pub struct WorldCreationSettings {
    pub world_name: String,
    pub world_seed: u64,
    pub city_size: u32,
    pub city_spacing: u32,
    pub spawn_rate: f32,
    pub item_spawn_rate: f32,
    pub monster_evolution_rate: f32,
    pub initial_time: String,
    pub season_length: u32,
    pub static_npc: bool,
    pub random_npc: bool,
}

impl Default for WorldCreationSettings {
    fn default() -> Self {
        Self {
            world_name: "New World".into(),
            world_seed: 0,
            city_size: 8,
            city_spacing: 4,
            spawn_rate: 1.0,
            item_spawn_rate: 1.0,
            monster_evolution_rate: 1.0,
            initial_time: "dawn".into(),
            season_length: 91,
            static_npc: false,
            random_npc: true,
        }
    }
}
