//! Screen trait — eliminates duplicated registration boilerplate.
//!
//! Implement `CddaScreen` for a unit struct, then call
//! `app.add_plugins(Screen::<YourScreen>::default())` to register
//! the OnEnter (populate actions + spawn) and Update systems.

use bevy_app::{App, Plugin, Update};
use bevy_ecs::prelude::*;
use bevy_state::condition::in_state;
use bevy_state::state::OnEnter;
use std::marker::PhantomData;

use crate::actions::ContextActions;
use crate::ctx::Ctx;
use cdda_components::input::BindableAction;

// ---------------------------------------------------------------------------
// CddaScreen trait
// ---------------------------------------------------------------------------

/// Implement this trait on a unit struct to declare a screen.
///
/// # Cleanup responsibility
///
/// Implementors that spawn UI trees MUST ensure proper cleanup when the
/// screen exits.  Use either `DespawnOnExit` on the root entity (preferred)
/// or call `despawn_recursive()` manually.  Failing to do so leaks entities
/// and can cause visual overlay glitches on screen transitions.
///
/// # Example
/// ```ignore
/// pub struct InventoryScreen;
///
/// impl CddaScreen for InventoryScreen {
///     const CTX: Ctx = Ctx::Inventory;
///     const ACTIONS: &'static [(&'static str, BindableAction)] = &[
///         ("navigate", BindableAction::NavigateUp),
///         ("examine", BindableAction::Confirm),
///     ];
///     fn spawn(world: &mut World) {
///         // build UI ...
///     }
///     fn update(world: &mut World) {
///         // refresh per-frame ...
///     }
/// }
///
/// app.add_plugins(Screen::<InventoryScreen>::default());
/// ```
pub trait CddaScreen: Send + Sync + 'static {
    /// Which `Ctx` state this screen occupies.
    const CTX: Ctx;

    /// Static actions shown in the footer.
    const ACTIONS: &'static [(&'static str, BindableAction)];

    /// Spawn the UI.  Called on `OnEnter(Self::CTX)`.
    fn spawn(world: &mut World);

    /// Update the UI each frame.  Called in `Update` when `in_state(Self::CTX)`.
    fn update(_world: &mut World) {}
}

// ---------------------------------------------------------------------------
// Screen plugin — registers all systems for a CddaScreen
// ---------------------------------------------------------------------------

/// Generated plugin.  Use via `Screen::<YourScreen>::default()`.
pub struct Screen<S: CddaScreen> {
    _phantom: PhantomData<S>,
}

impl<S: CddaScreen> Default for Screen<S> {
    fn default() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<S: CddaScreen> Plugin for Screen<S> {
    fn build(&self, app: &mut App) {
        // 1. Populate ContextActions on enter.
        app.add_systems(OnEnter(S::CTX), populate_actions::<S>);

        // 2. Spawn UI on enter.
        app.add_systems(OnEnter(S::CTX), spawn_screen::<S>);

        // 3. Update each frame.
        app.add_systems(Update, update_screen::<S>.run_if(in_state(S::CTX)));
    }
}

fn populate_actions<S: CddaScreen>(mut ctx: ResMut<ContextActions>) {
    ctx.populate(S::ACTIONS);
}

fn spawn_screen<S: CddaScreen>(world: &mut World) {
    S::spawn(world);
}

fn update_screen<S: CddaScreen>(world: &mut World) {
    S::update(world);
}
