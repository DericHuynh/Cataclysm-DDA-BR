use cdda_context::ctx::Ctx as Screen;
use cdda_context::nav::FocusedCommandIndex;

// ---------------------------------------------------------------------------
// Initial state
// ---------------------------------------------------------------------------

#[test]
fn default_current_is_zero() {
    let idx = FocusedCommandIndex::default();
    assert_eq!(idx.current(), 0);
}

// ---------------------------------------------------------------------------
// set
// ---------------------------------------------------------------------------

#[test]
fn set_changes_current() {
    let mut idx = FocusedCommandIndex::default();
    idx.set(3);
    assert_eq!(idx.current(), 3);
}

#[test]
fn set_zero_resets_current() {
    let mut idx = FocusedCommandIndex::default();
    idx.set(5);
    idx.set(0);
    assert_eq!(idx.current(), 0);
}

#[test]
fn set_large_value() {
    let mut idx = FocusedCommandIndex::default();
    idx.set(usize::MAX);
    assert_eq!(idx.current(), usize::MAX);
}

// ---------------------------------------------------------------------------
// on_push — saves current, loads saved for target (or 0)
// ---------------------------------------------------------------------------

#[test]
fn on_push_saves_current_screen_index() {
    let mut idx = FocusedCommandIndex::default();
    idx.set(2);
    idx.on_push(Screen::MainMenu, Screen::SettingsMenu);
    // After push, current should be 0 (first visit to SettingsMenu)
    assert_eq!(idx.current(), 0);
}

#[test]
fn on_push_saves_from_screen_index_in_history() {
    let mut idx = FocusedCommandIndex::default();

    // Set focus in MainMenu, then push to SettingsMenu
    idx.set(4);
    idx.on_push(Screen::MainMenu, Screen::SettingsMenu);
    // on_push saves MainMenu=4 in history; SettingsMenu not yet saved so current=0
    assert_eq!(idx.current(), 0);

    // Simulate returning via pop — restores MainMenu's saved index
    idx.on_pop(Screen::MainMenu);
    assert_eq!(idx.current(), 4);

    // Push to SettingsMenu again — on_push(MainMenu→SettingsMenu) saves current
    // index for MainMenu; SettingsMenu was never explicitly saved so still 0
    idx.set(2);
    idx.on_push(Screen::MainMenu, Screen::SettingsMenu);
    assert_eq!(idx.current(), 0);
}

#[test]
fn on_push_first_visit_is_zero() {
    let mut idx = FocusedCommandIndex::default();
    idx.set(7);
    idx.on_push(Screen::MainMenu, Screen::NewGameHub);
    assert_eq!(idx.current(), 0);
}

// ---------------------------------------------------------------------------
// on_pop — restores saved index for target
// ---------------------------------------------------------------------------

#[test]
fn on_pop_restores_parent_index() {
    let mut idx = FocusedCommandIndex::default();
    // Start at MainMenu with focus on item 2
    idx.set(2);
    // Push to Settings
    idx.on_push(Screen::MainMenu, Screen::SettingsMenu);
    idx.set(4);
    // Pop back
    idx.on_pop(Screen::MainMenu);
    assert_eq!(idx.current(), 2);
}

#[test]
fn on_pop_unknown_screen_gives_zero() {
    let mut idx = FocusedCommandIndex::default();
    idx.on_pop(Screen::Inventory); // never visited
    assert_eq!(idx.current(), 0);
}

// ---------------------------------------------------------------------------
// Push/pop round-trip
// ---------------------------------------------------------------------------

#[test]
fn push_then_pop_restores_original_index() {
    let mut idx = FocusedCommandIndex::default();
    idx.set(5);
    idx.on_push(Screen::MainMenu, Screen::SettingsMenu);
    idx.set(1);
    idx.on_pop(Screen::MainMenu);
    assert_eq!(idx.current(), 5);
}

#[test]
fn nested_push_pop_restores_correct_index() {
    let mut idx = FocusedCommandIndex::default();
    // MainMenu focus = 3
    idx.set(3);
    idx.on_push(Screen::MainMenu, Screen::NewGameHub);
    // NewGameHub focus = 1
    idx.set(1);
    idx.on_push(Screen::NewGameHub, Screen::ScenarioSelect);
    // ScenarioSelect focus = 7
    idx.set(7);

    // Pop back to NewGameHub
    idx.on_pop(Screen::NewGameHub);
    assert_eq!(idx.current(), 1, "NewGameHub focus should be restored");

    // Pop back to MainMenu
    idx.on_pop(Screen::MainMenu);
    assert_eq!(idx.current(), 3, "MainMenu focus should be restored");
}
