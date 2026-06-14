//! # cdda_intent — String-typed intent vocabulary
//!
//! The workspace's canonical input/output language between systems, UI, AI,
//! and replay. An `Intent` is a `&'static str` name plus a `serde_json::Value`
//! payload. Names are stable string identifiers; payloads are untyped JSON
//! (the receiving system parses them with `parse_intent!`).
//!
//! ## Why strings, not enums
//!
//! A 167-variant `GameAction` enum compiles into every workspace crate that
//! touches input, and every variant is a parse failure point at the call
//! site. String names decouple producers from consumers: a new intent can be
//! added in one place without forcing every consumer to recompile, and dead
//! variants don't break the build.
//!
//! ## Workflow
//!
//! 1. A producer builds an `Intent::new("move", json!({"dx": -1, "dy": 0}))`
//!    and either stores it in a `Message<Intent>` buffer or calls a handler
//!    directly.
//! 2. A consumer subscribes to the `Message<Intent>` stream, filters by
//!    `intent.name`, and parses the payload:
//!
//! ```ignore
//! for intent in reader.read() {
//!     if intent.name != "move" { continue; }
//!     let (dx, dy): (i32, i32) = parse_intent!(intent.payload, dx: i32, dy: i32);
//!     // ... move the actor by (dx, dy)
//! }
//! ```
//!
//! Or, with the typed-handler pattern, register an `IntentHandler` and let the
//! router dispatch.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

pub mod registry;

/// The workspace's canonical intent type. Cheap to clone, hash, and compare.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Intent {
    /// Stable string identifier. Convention: lower_snake_case, no version
    /// suffix (mod compatibility is a registry concern, not a name concern).
    pub name: &'static str,
    /// Untyped JSON payload. Parsed at the call site via `parse_intent!`
    /// or the typed-handler pattern.
    pub payload: Value,
}

impl Intent {
    /// Build a new intent. Most callers should use the `intent!` macro for
    /// less boilerplate at the call site.
    pub const fn new(name: &'static str, payload: Value) -> Self {
        Self { name, payload }
    }

    /// Build an intent with an empty payload (event-style, no data).
    pub const fn unit(name: &'static str) -> Self {
        Self {
            name,
            payload: Value::Null,
        }
    }

    /// True if the payload is `Value::Null`. Useful for `match`-on-shape.
    pub fn has_payload(&self) -> bool {
        !self.payload.is_null()
    }
}

/// Errors raised when parsing a payload that does not match the expected
/// shape. `KeyNotFound` and `TypeMismatch` are produced by `parse_intent!`;
/// `UnknownIntent` is produced by the registry router.
#[derive(Debug, Error, PartialEq)]
pub enum IntentError {
    #[error("intent payload key not found: {0}")]
    KeyNotFound(&'static str),
    #[error("intent payload key {key} had wrong type: expected {expected}")]
    TypeMismatch {
        key: &'static str,
        expected: &'static str,
    },
    #[error("intent payload is not a JSON object: {0}")]
    NotAnObject(String),
    #[error("unknown intent: {0}")]
    UnknownIntent(String),
}

/// Extract a typed tuple of fields from a `serde_json::Value` payload.
///
/// `payload` must be a JSON object. Each `key: type` pair is extracted in
/// order and bound to a local variable of the same name. Fails with
/// `IntentError::KeyNotFound` or `IntentError::TypeMismatch`.
///
/// ```ignore
/// let (dx, dy): (i32, i32) = parse_intent!(payload, dx: i32, dy: i32);
/// ```
#[macro_export]
macro_rules! parse_intent {
    ($payload:expr, $($key:ident : $ty:ty),+ $(,)?) => {{
        let obj = $payload
            .as_object()
            .ok_or_else(|| $crate::IntentError::NotAnObject($payload.to_string()));
        obj.and_then(|obj| -> Result<($($ty,)+), $crate::IntentError> {
            Ok(($(
                obj.get(stringify!($key))
                    .ok_or($crate::IntentError::KeyNotFound(stringify!($key)))
                    .and_then(|v| serde_json::from_value::<$ty>(v.clone())
                        .map_err(|_| $crate::IntentError::TypeMismatch {
                            key: stringify!($key),
                            expected: stringify!($ty),
                        }))?,
            )+))
        })
    }};
}
