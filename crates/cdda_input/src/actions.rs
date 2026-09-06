//! Semantic game actions and input events.
//!
//! Two layers:
//! - `BindableAction` — flat enum that implements `Actionlike`; used with
//!   leafwing `InputMap<BindableAction>` and `ActionState<BindableAction>`.
//! - `GameAction` — rich enum with data variants (`Move(Direction)`,
//!   `TextChar(String)`, etc.); the downstream message type that all systems read.
//!
//! `BindableAction::to_game_action()` converts between layers.
//!
//! `Direction`, `GameAction`, `ActionSource`, `InputAction`, and `BindableAction` are
//! defined in `crate::vocabulary` so that downstream crates can use them
//! without depending on `cdda_core`.

pub use crate::vocabulary::{ActionSource, BindableAction, Direction, GameAction, InputAction};
