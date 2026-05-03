//! Pure logic functions — no ECS dependencies.
//!
//! Each module is pure computation that takes domain types and returns results.
//! Systems in `crate::systems` call into these functions.

pub mod ai;
pub mod combat;
pub mod fsm;
