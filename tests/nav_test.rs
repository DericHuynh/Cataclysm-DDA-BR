use bevy_state::prelude::NextState;
use cdda_context::ctx::{ContextStack as ScreenStack, Ctx as Screen};
use cdda_context::nav::{
    pop_ctx as pop_screen, push_ctx as push_screen, FocusedCommandIndex,
};

// ---------------------------------------------------------------------------
// ScreenStack primitives
// ---------------------------------------------------------------------------

#[test]
fn stack_starts_empty() {
    let stack = ScreenStack::default();
    assert!(stack.0.is_empty());
}

#[test]
fn push_adds_to_stack() {
    let mut stack = ScreenStack::default();
    let mut next = NextState::<Screen>::default();
    let mut focused = FocusedCommandIndex::default();

    push_screen(
        Screen::MainMenu,
        Screen::SettingsMenu,
        &mut stack,
        &mut next,
        &mut focused,
    );

    assert_eq!(stack.0.len(), 1);
    assert_eq!(stack.0[0], Screen::MainMenu);
}

#[test]
fn push_stores_current_screen_not_next() {
    let mut stack = ScreenStack::default();
    let mut next = NextState::<Screen>::default();
    let mut focused = FocusedCommandIndex::default();

    push_screen(
        Screen::MainMenu,
        Screen::NewGameHub,
        &mut stack,
        &mut next,
        &mut focused,
    );

    // The stack records where we came from (MainMenu), not where we're going
    assert_eq!(stack.0[0], Screen::MainMenu);
}

#[test]
fn multiple_pushes_grow_stack() {
    let mut stack = ScreenStack::default();
    let mut next = NextState::<Screen>::default();
    let mut focused = FocusedCommandIndex::default();

    push_screen(
        Screen::MainMenu,
        Screen::NewGameHub,
        &mut stack,
        &mut next,
        &mut focused,
    );
    push_screen(
        Screen::NewGameHub,
        Screen::ScenarioSelect,
        &mut stack,
        &mut next,
        &mut focused,
    );

    assert_eq!(stack.0.len(), 2);
    assert_eq!(stack.0[0], Screen::MainMenu);
    assert_eq!(stack.0[1], Screen::NewGameHub);
}

// ---------------------------------------------------------------------------
// pop_screen
// ---------------------------------------------------------------------------

#[test]
fn pop_on_empty_stack_is_noop() {
    let mut stack = ScreenStack::default();
    let mut next = NextState::<Screen>::default();
    let mut focused = FocusedCommandIndex::default();

    // Should not panic
    pop_screen(&mut stack, &mut next, &mut focused);
    assert!(stack.0.is_empty());
}

#[test]
fn pop_removes_top_of_stack() {
    let mut stack = ScreenStack::default();
    let mut next = NextState::<Screen>::default();
    let mut focused = FocusedCommandIndex::default();

    push_screen(
        Screen::MainMenu,
        Screen::SettingsMenu,
        &mut stack,
        &mut next,
        &mut focused,
    );
    assert_eq!(stack.0.len(), 1);

    pop_screen(&mut stack, &mut next, &mut focused);
    assert!(stack.0.is_empty());
}

#[test]
fn push_then_pop_returns_to_empty_stack() {
    let mut stack = ScreenStack::default();
    let mut next = NextState::<Screen>::default();
    let mut focused = FocusedCommandIndex::default();

    push_screen(
        Screen::MainMenu,
        Screen::SettingsMenu,
        &mut stack,
        &mut next,
        &mut focused,
    );
    push_screen(
        Screen::SettingsMenu,
        Screen::HelpScreen,
        &mut stack,
        &mut next,
        &mut focused,
    );
    assert_eq!(stack.0.len(), 2);

    pop_screen(&mut stack, &mut next, &mut focused);
    assert_eq!(stack.0.len(), 1);

    pop_screen(&mut stack, &mut next, &mut focused);
    assert!(stack.0.is_empty());
}

// ---------------------------------------------------------------------------
// push_screen restores focus correctly
// ---------------------------------------------------------------------------

#[test]
fn push_resets_focus_to_zero_for_first_visit() {
    let mut stack = ScreenStack::default();
    let mut next = NextState::<Screen>::default();
    let mut focused = FocusedCommandIndex::default();

    focused.set(3);
    push_screen(
        Screen::MainMenu,
        Screen::NewGameHub,
        &mut stack,
        &mut next,
        &mut focused,
    );

    assert_eq!(focused.current(), 0);
}

#[test]
fn pop_restores_focus_to_parent_screen() {
    let mut stack = ScreenStack::default();
    let mut next = NextState::<Screen>::default();
    let mut focused = FocusedCommandIndex::default();

    // Focus on item 4 in MainMenu
    focused.set(4);
    push_screen(
        Screen::MainMenu,
        Screen::SettingsMenu,
        &mut stack,
        &mut next,
        &mut focused,
    );

    // Move focus in settings
    focused.set(2);
    pop_screen(&mut stack, &mut next, &mut focused);

    // Should restore MainMenu's saved focus
    assert_eq!(focused.current(), 4);
}
