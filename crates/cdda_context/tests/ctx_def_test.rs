use cdda_context::ctx::Ctx as Screen;
use cdda_context::nav::{ctx_def as screen_def, GameEvent, TransitionTarget};

// ---------------------------------------------------------------------------
// MainMenu
// ---------------------------------------------------------------------------

#[test]
fn main_menu_has_title() {
    let def = screen_def(Screen::MainMenu);
    assert!(!def.title.is_empty());
    assert!(def.title.contains("CATACLYSM"));
}

#[test]
fn main_menu_command_count() {
    let def = screen_def(Screen::MainMenu);
    assert_eq!(def.commands.len(), 9, "expected 9 main menu commands");
}

#[test]
fn main_menu_has_new_game_command() {
    let def = screen_def(Screen::MainMenu);
    assert!(
        def.commands.iter().any(|c| c.label == "New Game"),
        "expected a 'New Game' command"
    );
}

#[test]
fn main_menu_quit_command_has_hotkey_q() {
    let def = screen_def(Screen::MainMenu);
    let quit = def
        .commands
        .iter()
        .find(|c| c.label == "Quit")
        .expect("Quit command");
    assert_eq!(quit.hotkey, Some('q'));
}

#[test]
fn main_menu_quit_target_is_quit() {
    let def = screen_def(Screen::MainMenu);
    let quit = def
        .commands
        .iter()
        .find(|c| c.label == "Quit")
        .expect("Quit command");
    assert!(matches!(quit.target, TransitionTarget::Quit));
}

#[test]
fn main_menu_settings_pushes_settings_screen() {
    let def = screen_def(Screen::MainMenu);
    let settings = def
        .commands
        .iter()
        .find(|c| c.label == "Settings")
        .expect("Settings");
    assert!(matches!(
        settings.target,
        TransitionTarget::Push(Screen::SettingsMenu)
    ));
}

#[test]
fn main_menu_new_game_pushes_new_game_hub() {
    let def = screen_def(Screen::MainMenu);
    let ng = def
        .commands
        .iter()
        .find(|c| c.label == "New Game")
        .expect("New Game");
    assert!(matches!(
        ng.target,
        TransitionTarget::Push(Screen::NewGameHub)
    ));
}

#[test]
fn main_menu_all_commands_have_hotkeys() {
    let def = screen_def(Screen::MainMenu);
    for cmd in &def.commands {
        assert!(
            cmd.hotkey.is_some() || cmd.label == "Special",
            "command '{}' missing hotkey",
            cmd.label
        );
    }
}

// ---------------------------------------------------------------------------
// NewGameHub
// ---------------------------------------------------------------------------

#[test]
fn new_game_hub_has_start_game_event() {
    let def = screen_def(Screen::NewGameHub);
    let start = def
        .commands
        .iter()
        .find(|c| c.label == "Start Game")
        .expect("Start Game");
    assert!(matches!(
        start.target,
        TransitionTarget::Event(GameEvent::StartNewGame)
    ));
}

#[test]
fn new_game_hub_character_pushes_scenario_select() {
    let def = screen_def(Screen::NewGameHub);
    let chr = def
        .commands
        .iter()
        .find(|c| c.label == "Character")
        .expect("Character");
    assert!(matches!(
        chr.target,
        TransitionTarget::Push(Screen::ScenarioSelect)
    ));
}

// ---------------------------------------------------------------------------
// CharacterConfirm
// ---------------------------------------------------------------------------

#[test]
fn character_confirm_start_game_emits_event() {
    let def = screen_def(Screen::CharacterConfirm);
    let start = def
        .commands
        .iter()
        .find(|c| c.label == "Start Game")
        .expect("Start Game");
    assert!(matches!(
        start.target,
        TransitionTarget::Event(GameEvent::StartNewGame)
    ));
}

#[test]
fn character_confirm_go_back_pops() {
    let def = screen_def(Screen::CharacterConfirm);
    let back = def
        .commands
        .iter()
        .find(|c| c.label == "Go Back")
        .expect("Go Back");
    assert!(matches!(back.target, TransitionTarget::Pop));
}

// ---------------------------------------------------------------------------
// WorldMenu
// ---------------------------------------------------------------------------

#[test]
fn world_menu_create_pushes_world_settings() {
    let def = screen_def(Screen::WorldMenu);
    let create = def
        .commands
        .iter()
        .find(|c| c.label == "Create World")
        .expect("Create World");
    assert!(matches!(
        create.target,
        TransitionTarget::Push(Screen::WorldSettings)
    ));
}

#[test]
fn world_settings_save_pops() {
    let def = screen_def(Screen::WorldSettings);
    let save = def
        .commands
        .iter()
        .find(|c| c.label == "Save & Return")
        .expect("Save & Return");
    assert!(matches!(save.target, TransitionTarget::Pop));
}

// ---------------------------------------------------------------------------
// DevWorldgen
// ---------------------------------------------------------------------------

#[test]
fn dev_worldgen_start_emits_start_new_game_event() {
    let def = screen_def(Screen::DevWorldgen);
    let start = def
        .commands
        .iter()
        .find(|c| c.label == "Start Showcase")
        .expect("Start Showcase");
    assert!(matches!(
        start.target,
        TransitionTarget::Event(GameEvent::StartNewGame)
    ));
}

#[test]
fn dev_worldgen_go_back_pops() {
    let def = screen_def(Screen::DevWorldgen);
    let back = def
        .commands
        .iter()
        .find(|c| c.label == "Go Back")
        .expect("Go Back");
    assert!(matches!(back.target, TransitionTarget::Pop));
}

// ---------------------------------------------------------------------------
// Gameplay
// ---------------------------------------------------------------------------

#[test]
fn gameplay_has_inventory_command() {
    let def = screen_def(Screen::Gameplay);
    assert!(def.commands.iter().any(|c| c.label == "Inventory"));
}

#[test]
fn gameplay_inventory_hotkey_is_i() {
    let def = screen_def(Screen::Gameplay);
    let inv = def
        .commands
        .iter()
        .find(|c| c.label == "Inventory")
        .expect("Inventory");
    assert_eq!(inv.hotkey, Some('i'));
}

#[test]
fn gameplay_inventory_pushes_inventory_screen() {
    let def = screen_def(Screen::Gameplay);
    let inv = def
        .commands
        .iter()
        .find(|c| c.label == "Inventory")
        .expect("Inventory");
    assert!(matches!(
        inv.target,
        TransitionTarget::Push(Screen::Inventory)
    ));
}

// ---------------------------------------------------------------------------
// Screens with no static commands
// ---------------------------------------------------------------------------

#[test]
fn scenario_select_has_no_static_commands() {
    let def = screen_def(Screen::ScenarioSelect);
    assert!(def.commands.is_empty());
}

#[test]
fn custom_screen_has_no_static_commands() {
    let def = screen_def(Screen::Custom(42));
    assert!(def.commands.is_empty());
}
