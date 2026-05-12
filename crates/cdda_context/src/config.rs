//! Configuration resources for settings, character creation, and world gen.

use bevy_ecs::prelude::Resource;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Setting enums
// ---------------------------------------------------------------------------

/// Temperature display unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TemperatureUnit { Fahrenheit, #[default] Celsius, Kelvin }
impl TemperatureUnit {
    pub const ALL: &'static [Self] = &[Self::Fahrenheit, Self::Celsius, Self::Kelvin];
    pub fn label(self) -> &'static str { match self { Self::Fahrenheit => "Fahrenheit", Self::Celsius => "Celsius", Self::Kelvin => "Kelvin" } }
    pub fn next(self) -> Self { let i = Self::ALL.iter().position(|v| *v == self).unwrap_or(0); Self::ALL[(i + 1) % Self::ALL.len()] }
    pub fn prev(self) -> Self { let i = Self::ALL.iter().position(|v| *v == self).unwrap_or(0); Self::ALL[(i + Self::ALL.len() - 1) % Self::ALL.len()] }
}

/// Speed display unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SpeedUnit { #[default] Mph, Kmh, TilesPerTurn }
impl SpeedUnit {
    pub const ALL: &'static [Self] = &[Self::Mph, Self::Kmh, Self::TilesPerTurn];
    pub fn label(self) -> &'static str { match self { Self::Mph => "Mph", Self::Kmh => "Km/h", Self::TilesPerTurn => "Tiles/Turn" } }
    pub fn next(self) -> Self { let i = Self::ALL.iter().position(|v| *v == self).unwrap_or(0); Self::ALL[(i + 1) % Self::ALL.len()] }
    pub fn prev(self) -> Self { let i = Self::ALL.iter().position(|v| *v == self).unwrap_or(0); Self::ALL[(i + Self::ALL.len() - 1) % Self::ALL.len()] }
}

/// Weight display unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WeightUnit { #[default] Lbs, Kg }
impl WeightUnit {
    pub const ALL: &'static [Self] = &[Self::Lbs, Self::Kg];
    pub fn label(self) -> &'static str { match self { Self::Lbs => "Lbs", Self::Kg => "Kg" } }
    pub fn next(self) -> Self { let i = Self::ALL.iter().position(|v| *v == self).unwrap_or(0); Self::ALL[(i + 1) % Self::ALL.len()] }
    pub fn prev(self) -> Self { let i = Self::ALL.iter().position(|v| *v == self).unwrap_or(0); Self::ALL[(i + Self::ALL.len() - 1) % Self::ALL.len()] }
}

/// Volume display unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VolumeUnit { #[default] Cup, Liter, Quart }
impl VolumeUnit {
    pub const ALL: &'static [Self] = &[Self::Cup, Self::Liter, Self::Quart];
    pub fn label(self) -> &'static str { match self { Self::Cup => "Cup", Self::Liter => "Liter", Self::Quart => "Quart" } }
    pub fn next(self) -> Self { let i = Self::ALL.iter().position(|v| *v == self).unwrap_or(0); Self::ALL[(i + 1) % Self::ALL.len()] }
    pub fn prev(self) -> Self { let i = Self::ALL.iter().position(|v| *v == self).unwrap_or(0); Self::ALL[(i + Self::ALL.len() - 1) % Self::ALL.len()] }
}

/// Distance display unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DistanceUnit { #[default] Metric, Imperial }
impl DistanceUnit {
    pub const ALL: &'static [Self] = &[Self::Metric, Self::Imperial];
    pub fn label(self) -> &'static str { match self { Self::Metric => "Metric", Self::Imperial => "Imperial" } }
    pub fn next(self) -> Self { let i = Self::ALL.iter().position(|v| *v == self).unwrap_or(0); Self::ALL[(i + 1) % Self::ALL.len()] }
    pub fn prev(self) -> Self { let i = Self::ALL.iter().position(|v| *v == self).unwrap_or(0); Self::ALL[(i + Self::ALL.len() - 1) % Self::ALL.len()] }
}

/// Time display format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimeFormat { TwelveHour, #[default] Military, TwentyFourHour }
impl TimeFormat {
    pub const ALL: &'static [Self] = &[Self::TwelveHour, Self::Military, Self::TwentyFourHour];
    pub fn label(self) -> &'static str { match self { Self::TwelveHour => "12 hour", Self::Military => "24 hour (no am/pm)", Self::TwentyFourHour => "24 hour (am/pm)" } }
    pub fn next(self) -> Self { let i = Self::ALL.iter().position(|v| *v == self).unwrap_or(0); Self::ALL[(i + 1) % Self::ALL.len()] }
    pub fn prev(self) -> Self { let i = Self::ALL.iter().position(|v| *v == self).unwrap_or(0); Self::ALL[(i + Self::ALL.len() - 1) % Self::ALL.len()] }
}

/// Auto-pulp / auto-butcher mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AutoPulpMode { #[default] Off, Pulp, PulpAdjacent, Butcher }
impl AutoPulpMode {
    pub const ALL: &'static [Self] = &[Self::Off, Self::Pulp, Self::PulpAdjacent, Self::Butcher];
    pub fn label(self) -> &'static str { match self { Self::Off => "Off", Self::Pulp => "Pulp", Self::PulpAdjacent => "Pulp Adjacent", Self::Butcher => "Butcher" } }
    pub fn next(self) -> Self { let i = Self::ALL.iter().position(|v| *v == self).unwrap_or(0); Self::ALL[(i + 1) % Self::ALL.len()] }
    pub fn prev(self) -> Self { let i = Self::ALL.iter().position(|v| *v == self).unwrap_or(0); Self::ALL[(i + Self::ALL.len() - 1) % Self::ALL.len()] }
}

/// Auto-foraging mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AutoForagingMode { #[default] Off, Bushes, Trees, Crops, All }
impl AutoForagingMode {
    pub const ALL: &'static [Self] = &[Self::Off, Self::Bushes, Self::Trees, Self::Crops, Self::All];
    pub fn label(self) -> &'static str { match self { Self::Off => "Off", Self::Bushes => "Bushes", Self::Trees => "Trees", Self::Crops => "Crops", Self::All => "All" } }
    pub fn next(self) -> Self { let i = Self::ALL.iter().position(|v| *v == self).unwrap_or(0); Self::ALL[(i + 1) % Self::ALL.len()] }
    pub fn prev(self) -> Self { let i = Self::ALL.iter().position(|v| *v == self).unwrap_or(0); Self::ALL[(i + Self::ALL.len() - 1) % Self::ALL.len()] }
}

/// Dangerous terrain warning mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DangerousTerrainWarning { #[default] Always, Running, Crouching, Never }
impl DangerousTerrainWarning {
    pub const ALL: &'static [Self] = &[Self::Always, Self::Running, Self::Crouching, Self::Never];
    pub fn label(self) -> &'static str { match self { Self::Always => "Always", Self::Running => "Running only", Self::Crouching => "Crouch walking only", Self::Never => "Never" } }
    pub fn next(self) -> Self { let i = Self::ALL.iter().position(|v| *v == self).unwrap_or(0); Self::ALL[(i + 1) % Self::ALL.len()] }
    pub fn prev(self) -> Self { let i = Self::ALL.iter().position(|v| *v == self).unwrap_or(0); Self::ALL[(i + Self::ALL.len() - 1) % Self::ALL.len()] }
}

/// Drop empty containers mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DropEmptyMode { #[default] No, Watertight, All }
impl DropEmptyMode {
    pub const ALL: &'static [Self] = &[Self::No, Self::Watertight, Self::All];
    pub fn label(self) -> &'static str { match self { Self::No => "No", Self::Watertight => "Watertight only", Self::All => "All" } }
    pub fn next(self) -> Self { let i = Self::ALL.iter().position(|v| *v == self).unwrap_or(0); Self::ALL[(i + 1) % Self::ALL.len()] }
    pub fn prev(self) -> Self { let i = Self::ALL.iter().position(|v| *v == self).unwrap_or(0); Self::ALL[(i + Self::ALL.len() - 1) % Self::ALL.len()] }
}

/// Death cam mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DeathCamMode { #[default] Always, Ask, Never }
impl DeathCamMode {
    pub const ALL: &'static [Self] = &[Self::Always, Self::Ask, Self::Never];
    pub fn label(self) -> &'static str { match self { Self::Always => "Always", Self::Ask => "Ask", Self::Never => "Never" } }
    pub fn next(self) -> Self { let i = Self::ALL.iter().position(|v| *v == self).unwrap_or(0); Self::ALL[(i + 1) % Self::ALL.len()] }
    pub fn prev(self) -> Self { let i = Self::ALL.iter().position(|v| *v == self).unwrap_or(0); Self::ALL[(i + Self::ALL.len() - 1) % Self::ALL.len()] }
}

/// World end mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorldEndMode { #[default] Reset, Delete, Query, Keep }
impl WorldEndMode {
    pub const ALL: &'static [Self] = &[Self::Reset, Self::Delete, Self::Query, Self::Keep];
    pub fn label(self) -> &'static str { match self { Self::Reset => "Reset", Self::Delete => "Delete", Self::Query => "Query", Self::Keep => "Keep" } }
    pub fn next(self) -> Self { let i = Self::ALL.iter().position(|v| *v == self).unwrap_or(0); Self::ALL[(i + 1) % Self::ALL.len()] }
    pub fn prev(self) -> Self { let i = Self::ALL.iter().position(|v| *v == self).unwrap_or(0); Self::ALL[(i + Self::ALL.len() - 1) % Self::ALL.len()] }
}

/// Sidebar position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SidebarPosition { Left, #[default] Right }
impl SidebarPosition {
    pub const ALL: &'static [Self] = &[Self::Left, Self::Right];
    pub fn label(self) -> &'static str { match self { Self::Left => "Left", Self::Right => "Right" } }
    pub fn next(self) -> Self { let i = Self::ALL.iter().position(|v| *v == self).unwrap_or(0); Self::ALL[(i + 1) % Self::ALL.len()] }
    pub fn prev(self) -> Self { let i = Self::ALL.iter().position(|v| *v == self).unwrap_or(0); Self::ALL[(i + Self::ALL.len() - 1) % Self::ALL.len()] }
}

/// Message log flow direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MessageLogFlow { #[default] NewTop, NewBottom }
impl MessageLogFlow {
    pub const ALL: &'static [Self] = &[Self::NewTop, Self::NewBottom];
    pub fn label(self) -> &'static str { match self { Self::NewTop => "New at Top", Self::NewBottom => "New at Bottom" } }
    pub fn next(self) -> Self { let i = Self::ALL.iter().position(|v| *v == self).unwrap_or(0); Self::ALL[(i + 1) % Self::ALL.len()] }
    pub fn prev(self) -> Self { let i = Self::ALL.iter().position(|v| *v == self).unwrap_or(0); Self::ALL[(i + Self::ALL.len() - 1) % Self::ALL.len()] }
}

/// Inventory highlight mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InventoryHighlight { #[default] Symbol, Highlight, Disabled }
impl InventoryHighlight {
    pub const ALL: &'static [Self] = &[Self::Symbol, Self::Highlight, Self::Disabled];
    pub fn label(self) -> &'static str { match self { Self::Symbol => "Symbol", Self::Highlight => "Highlight", Self::Disabled => "Disabled" } }
    pub fn next(self) -> Self { let i = Self::ALL.iter().position(|v| *v == self).unwrap_or(0); Self::ALL[(i + 1) % Self::ALL.len()] }
    pub fn prev(self) -> Self { let i = Self::ALL.iter().position(|v| *v == self).unwrap_or(0); Self::ALL[(i + Self::ALL.len() - 1) % Self::ALL.len()] }
}

/// Item health display mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ItemHealthDisplay { #[default] Bars, Descriptions, Both }
impl ItemHealthDisplay {
    pub const ALL: &'static [Self] = &[Self::Bars, Self::Descriptions, Self::Both];
    pub fn label(self) -> &'static str { match self { Self::Bars => "Bars", Self::Descriptions => "Descriptions", Self::Both => "Both" } }
    pub fn next(self) -> Self { let i = Self::ALL.iter().position(|v| *v == self).unwrap_or(0); Self::ALL[(i + 1) % Self::ALL.len()] }
    pub fn prev(self) -> Self { let i = Self::ALL.iter().position(|v| *v == self).unwrap_or(0); Self::ALL[(i + Self::ALL.len() - 1) % Self::ALL.len()] }
}

/// Auto inventory assignment mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AutoInvAssign { #[default] Disabled, Enabled, Favorites }
impl AutoInvAssign {
    pub const ALL: &'static [Self] = &[Self::Disabled, Self::Enabled, Self::Favorites];
    pub fn label(self) -> &'static str { match self { Self::Disabled => "Disabled", Self::Enabled => "Enabled", Self::Favorites => "Favorites" } }
    pub fn next(self) -> Self { let i = Self::ALL.iter().position(|v| *v == self).unwrap_or(0); Self::ALL[(i + 1) % Self::ALL.len()] }
    pub fn prev(self) -> Self { let i = Self::ALL.iter().position(|v| *v == self).unwrap_or(0); Self::ALL[(i + Self::ALL.len() - 1) % Self::ALL.len()] }
}

/// Accuracy display mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AccuracyDisplay { #[default] Numbers, Bars }
impl AccuracyDisplay {
    pub const ALL: &'static [Self] = &[Self::Numbers, Self::Bars];
    pub fn label(self) -> &'static str { match self { Self::Numbers => "Numbers", Self::Bars => "Bars" } }
    pub fn next(self) -> Self { let i = Self::ALL.iter().position(|v| *v == self).unwrap_or(0); Self::ALL[(i + 1) % Self::ALL.len()] }
    pub fn prev(self) -> Self { let i = Self::ALL.iter().position(|v| *v == self).unwrap_or(0); Self::ALL[(i + Self::ALL.len() - 1) % Self::ALL.len()] }
}

/// Achievement popup mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AchievementPopup { #[default] Never, Always, FirstTime }
impl AchievementPopup {
    pub const ALL: &'static [Self] = &[Self::Never, Self::Always, Self::FirstTime];
    pub fn label(self) -> &'static str { match self { Self::Never => "Never", Self::Always => "Always", Self::FirstTime => "First Time" } }
    pub fn next(self) -> Self { let i = Self::ALL.iter().position(|v| *v == self).unwrap_or(0); Self::ALL[(i + 1) % Self::ALL.len()] }
    pub fn prev(self) -> Self { let i = Self::ALL.iter().position(|v| *v == self).unwrap_or(0); Self::ALL[(i + Self::ALL.len() - 1) % Self::ALL.len()] }
}

/// Lookaround position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LookaroundPosition { Left, #[default] Right }
impl LookaroundPosition {
    pub const ALL: &'static [Self] = &[Self::Left, Self::Right];
    pub fn label(self) -> &'static str { match self { Self::Left => "Left", Self::Right => "Right" } }
    pub fn next(self) -> Self { let i = Self::ALL.iter().position(|v| *v == self).unwrap_or(0); Self::ALL[(i + 1) % Self::ALL.len()] }
    pub fn prev(self) -> Self { let i = Self::ALL.iter().position(|v| *v == self).unwrap_or(0); Self::ALL[(i + Self::ALL.len() - 1) % Self::ALL.len()] }
}

/// Morale style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MoraleStyle { #[default] Vertical, Horizontal }
impl MoraleStyle {
    pub const ALL: &'static [Self] = &[Self::Vertical, Self::Horizontal];
    pub fn label(self) -> &'static str { match self { Self::Vertical => "Vertical", Self::Horizontal => "Horizontal" } }
    pub fn next(self) -> Self { let i = Self::ALL.iter().position(|v| *v == self).unwrap_or(0); Self::ALL[(i + 1) % Self::ALL.len()] }
    pub fn prev(self) -> Self { let i = Self::ALL.iter().position(|v| *v == self).unwrap_or(0); Self::ALL[(i + Self::ALL.len() - 1) % Self::ALL.len()] }
}

/// Pixel minimap display mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PixelMinimapMode { #[default] Solid, Squares, Dots }
impl PixelMinimapMode {
    pub const ALL: &'static [Self] = &[Self::Solid, Self::Squares, Self::Dots];
    pub fn label(self) -> &'static str { match self { Self::Solid => "Solid", Self::Squares => "Squares", Self::Dots => "Dots" } }
    pub fn next(self) -> Self { let i = Self::ALL.iter().position(|v| *v == self).unwrap_or(0); Self::ALL[(i + 1) % Self::ALL.len()] }
    pub fn prev(self) -> Self { let i = Self::ALL.iter().position(|v| *v == self).unwrap_or(0); Self::ALL[(i + Self::ALL.len() - 1) % Self::ALL.len()] }
}

/// Memory map display mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MemoryMapMode { SepiaLight, SepiaDark, #[default] BlueDark, DarkenColor }
impl MemoryMapMode {
    pub const ALL: &'static [Self] = &[Self::SepiaLight, Self::SepiaDark, Self::BlueDark, Self::DarkenColor];
    pub fn label(self) -> &'static str { match self { Self::SepiaLight => "Sepia (Light)", Self::SepiaDark => "Sepia (Dark)", Self::BlueDark => "Blue (Dark)", Self::DarkenColor => "Darken colors" } }
    pub fn next(self) -> Self { let i = Self::ALL.iter().position(|v| *v == self).unwrap_or(0); Self::ALL[(i + 1) % Self::ALL.len()] }
    pub fn prev(self) -> Self { let i = Self::ALL.iter().position(|v| *v == self).unwrap_or(0); Self::ALL[(i + Self::ALL.len() - 1) % Self::ALL.len()] }
}

/// Fullscreen mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FullscreenMode { #[default] Windowed, Maximized, Fullscreen }
impl FullscreenMode {
    pub const ALL: &'static [Self] = &[Self::Windowed, Self::Maximized, Self::Fullscreen];
    pub fn label(self) -> &'static str { match self { Self::Windowed => "Windowed", Self::Maximized => "Maximized", Self::Fullscreen => "Fullscreen" } }
    pub fn next(self) -> Self { let i = Self::ALL.iter().position(|v| *v == self).unwrap_or(0); Self::ALL[(i + 1) % Self::ALL.len()] }
    pub fn prev(self) -> Self { let i = Self::ALL.iter().position(|v| *v == self).unwrap_or(0); Self::ALL[(i + Self::ALL.len() - 1) % Self::ALL.len()] }
}

// ---------------------------------------------------------------------------
// GameSettings
// ---------------------------------------------------------------------------

/// All user-configurable game settings, mirroring CDDA's options categories.
#[derive(Resource, Debug, Clone)]
pub struct GameSettings {
    // ── General ──────────────────────────────────────────────────────────────
    pub default_char_name: String,
    pub auto_pickup: bool,
    pub auto_pickup_adjacent: bool,
    pub auto_pickup_safemode: bool,
    pub auto_pickup_weight_limit: u32,
    pub auto_pickup_volume_limit: u32,
    pub auto_features: bool,
    pub auto_pulp_butcher: AutoPulpMode,
    pub auto_foraging: AutoForagingMode,
    pub dangerous_terrain_warning: DangerousTerrainWarning,
    pub safe_mode_proximity: u32,
    pub safe_mode_driving: bool,
    pub auto_safe_mode: bool,
    pub auto_safe_mode_turns: u32,
    pub safe_mode_ignore_turns: u32,
    pub turn_duration: u32,
    pub auto_save: bool,
    pub auto_save_turns: u32,
    pub auto_save_minutes: u32,
    pub auto_notes: bool,
    pub auto_notes_stairs: bool,
    pub auto_notes_map_extras: bool,
    pub auto_notes_dropped_favorites: bool,
    pub circular_distance: bool,
    pub drop_empty: DropEmptyMode,
    pub death_cam: DeathCamMode,
    pub world_end: WorldEndMode,
    pub meta_progress: bool,

    // ── Graphics / UI ────────────────────────────────────────────────────────
    pub temperature_unit: TemperatureUnit,
    pub speed_unit: SpeedUnit,
    pub weight_unit: WeightUnit,
    pub volume_unit: VolumeUnit,
    pub distance_unit: DistanceUnit,
    pub time_format: TimeFormat,
    pub show_day_month: bool,
    pub show_vitamin_mass: bool,
    pub force_capital_yn: bool,
    pub snap_to_target: bool,
    pub aim_after_firing: bool,
    pub query_disassemble: bool,
    pub query_deconstruct: bool,
    pub query_keybind_removal: bool,
    pub inventory_highlight: InventoryHighlight,
    pub highlight_unread_recipes: bool,
    pub highlight_unread_items: bool,
    pub sidebar_position: SidebarPosition,
    pub sidebar_spacers: bool,
    pub message_log_flow: MessageLogFlow,
    pub message_ttl: u32,
    pub message_cooldown: u32,
    pub message_limit: u32,
    pub no_unknown_cmd_msg: bool,
    pub achievement_popup: AchievementPopup,
    pub lookaround_position: LookaroundPosition,
    pub accuracy_display: AccuracyDisplay,
    pub morale_style: MoraleStyle,
    pub move_view_offset: u32,
    pub fast_scroll_offset: u32,
    pub auto_inv_assign: AutoInvAssign,
    pub item_health_display: ItemHealthDisplay,
    pub item_symbols: bool,
    pub item_bodygraph: bool,
    pub vehicle_armor_color: bool,
    pub driving_view_offset: u32,
    pub menu_scroll: bool,
    pub enable_mouse: bool,
    pub log_items_on_ground: bool,
    pub log_monster_movement: bool,

    // ── Graphics ─────────────────────────────────────────────────────────────
    pub fullscreen: FullscreenMode,
    pub animations: bool,
    pub animation_rain: bool,
    pub animation_projectiles: bool,
    pub animation_sct: bool,
    pub animation_delay_ms: u32,
    pub blink_speed_ms: u32,
    pub force_redraw: bool,
    pub enable_ascii_art: bool,
    pub pixel_minimap: bool,
    pub pixel_minimap_mode: PixelMinimapMode,
    pub pixel_minimap_brightness: f32,
    pub pixel_minimap_height: u32,
    pub pixel_minimap_beacon_size: u32,
    pub pixel_minimap_blink: bool,
    pub nv_green_toggle: bool,
    pub memory_map_mode: MemoryMapMode,

    // ── Audio ────────────────────────────────────────────────────────────────
    pub sound_enabled: bool,
    pub music_volume: f32,
    pub sfx_volume: f32,
    pub ambient_sound_volume: f32,
    pub soundpack: String,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            default_char_name: String::new(),
            auto_pickup: true,
            auto_pickup_adjacent: false,
            auto_pickup_safemode: false,
            auto_pickup_weight_limit: 0,
            auto_pickup_volume_limit: 0,
            auto_features: true,
            auto_pulp_butcher: AutoPulpMode::default(),
            auto_foraging: AutoForagingMode::default(),
            dangerous_terrain_warning: DangerousTerrainWarning::default(),
            safe_mode_proximity: 50,
            safe_mode_driving: true,
            auto_safe_mode: true,
            auto_safe_mode_turns: 50,
            safe_mode_ignore_turns: 1,
            turn_duration: 6,
            auto_save: true,
            auto_save_turns: 1,
            auto_save_minutes: 10,
            auto_notes: true,
            auto_notes_stairs: true,
            auto_notes_map_extras: true,
            auto_notes_dropped_favorites: false,
            circular_distance: true,
            drop_empty: DropEmptyMode::default(),
            death_cam: DeathCamMode::default(),
            world_end: WorldEndMode::default(),
            meta_progress: true,
            temperature_unit: TemperatureUnit::default(),
            speed_unit: SpeedUnit::default(),
            weight_unit: WeightUnit::default(),
            volume_unit: VolumeUnit::default(),
            distance_unit: DistanceUnit::default(),
            time_format: TimeFormat::default(),
            show_day_month: true,
            show_vitamin_mass: true,
            force_capital_yn: true,
            snap_to_target: false,
            aim_after_firing: true,
            query_disassemble: true,
            query_deconstruct: true,
            query_keybind_removal: false,
            inventory_highlight: InventoryHighlight::default(),
            highlight_unread_recipes: true,
            highlight_unread_items: true,
            sidebar_position: SidebarPosition::default(),
            sidebar_spacers: true,
            message_log_flow: MessageLogFlow::default(),
            message_ttl: 10,
            message_cooldown: 2,
            message_limit: 256,
            no_unknown_cmd_msg: false,
            achievement_popup: AchievementPopup::default(),
            lookaround_position: LookaroundPosition::default(),
            accuracy_display: AccuracyDisplay::default(),
            morale_style: MoraleStyle::default(),
            move_view_offset: 3,
            fast_scroll_offset: 3,
            auto_inv_assign: AutoInvAssign::default(),
            item_health_display: ItemHealthDisplay::default(),
            item_symbols: true,
            item_bodygraph: true,
            vehicle_armor_color: true,
            driving_view_offset: 3,
            menu_scroll: true,
            enable_mouse: true,
            log_items_on_ground: true,
            log_monster_movement: true,
            fullscreen: FullscreenMode::default(),
            animations: true,
            animation_rain: true,
            animation_projectiles: true,
            animation_sct: true,
            animation_delay_ms: 10,
            blink_speed_ms: 300,
            force_redraw: true,
            enable_ascii_art: true,
            pixel_minimap: true,
            pixel_minimap_mode: PixelMinimapMode::default(),
            pixel_minimap_brightness: 0.5,
            pixel_minimap_height: 200,
            pixel_minimap_beacon_size: 3,
            pixel_minimap_blink: true,
            nv_green_toggle: true,
            memory_map_mode: MemoryMapMode::default(),
            sound_enabled: true,
            music_volume: 100.0,
            sfx_volume: 80.0,
            ambient_sound_volume: 100.0,
            soundpack: "basic".into(),
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
