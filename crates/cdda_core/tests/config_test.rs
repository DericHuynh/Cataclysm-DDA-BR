use cdda_core::context::config::{
    CharacterCreationState, FullscreenMode, GameSettings, WorldCreationSettings,
};

#[test]
fn game_settings_default_auto_save() {
    let s = GameSettings::default();
    assert!(s.auto_save);
}

#[test]
fn game_settings_default_not_fullscreen() {
    let s = GameSettings::default();
    assert_eq!(s.fullscreen, FullscreenMode::Windowed);
}

#[test]
fn game_settings_default_music_volume() {
    let s = GameSettings::default();
    assert!(s.music_volume > 0);
}

#[test]
fn game_settings_default_sfx_volume() {
    let s = GameSettings::default();
    assert!(s.sfx_volume > 0);
}

#[test]
fn game_settings_mutation_works() {
    let mut s = GameSettings::default();
    s.auto_save = false;
    assert!(!s.auto_save);
    s.music_volume = 50;
    assert_eq!(s.music_volume, 50);
}

// ---------------------------------------------------------------------------
// CharacterCreationState defaults
// ---------------------------------------------------------------------------

#[test]
fn char_creation_default_scenario() {
    let c = CharacterCreationState::default();
    assert!(!c.scenario_id.is_empty());
}

#[test]
fn char_creation_default_stats_all_eight() {
    let c = CharacterCreationState::default();
    assert_eq!(c.strength, 8);
    assert_eq!(c.dexterity, 8);
    assert_eq!(c.intelligence, 8);
    assert_eq!(c.perception, 8);
}

#[test]
fn char_creation_default_has_unspent_points() {
    let c = CharacterCreationState::default();
    assert!(c.unspent_points > 0);
}

#[test]
fn char_creation_default_step_is_zero() {
    let c = CharacterCreationState::default();
    assert_eq!(c.step, 0);
}

#[test]
fn char_creation_default_no_traits() {
    let c = CharacterCreationState::default();
    assert!(c.selected_traits.is_empty());
}

#[test]
fn char_creation_default_no_skills() {
    let c = CharacterCreationState::default();
    assert!(c.selected_skills.is_empty());
}

#[test]
fn char_creation_name_starts_empty() {
    let c = CharacterCreationState::default();
    assert!(c.name.is_empty());
}

#[test]
fn char_creation_step_advances() {
    let mut c = CharacterCreationState::default();
    c.step = 1;
    assert_eq!(c.step, 1);
}

#[test]
fn char_creation_add_trait() {
    let mut c = CharacterCreationState::default();
    c.selected_traits.push("fast_learner".into());
    assert_eq!(c.selected_traits.len(), 1);
}

// ---------------------------------------------------------------------------
// WorldCreationSettings defaults
// ---------------------------------------------------------------------------

#[test]
fn world_creation_default_name() {
    let w = WorldCreationSettings::default();
    assert!(!w.world_name.is_empty());
}

#[test]
fn world_creation_default_seed_zero() {
    let w = WorldCreationSettings::default();
    assert_eq!(w.world_seed, 0);
}

#[test]
fn world_creation_default_spawn_rates_one() {
    let w = WorldCreationSettings::default();
    assert!((w.spawn_rate - 1.0).abs() < f32::EPSILON);
    assert!((w.item_spawn_rate - 1.0).abs() < f32::EPSILON);
}

#[test]
fn world_creation_default_season_length() {
    let w = WorldCreationSettings::default();
    assert!(w.season_length > 0);
}

#[test]
fn world_creation_city_size_positive() {
    let w = WorldCreationSettings::default();
    assert!(w.city_size > 0);
}

#[test]
fn world_creation_mutation_works() {
    let mut w = WorldCreationSettings::default();
    w.world_seed = 12345;
    w.world_name = "Test World".into();
    assert_eq!(w.world_seed, 12345);
    assert_eq!(w.world_name, "Test World");
}
