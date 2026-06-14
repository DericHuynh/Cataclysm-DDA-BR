//! Intent registry — typed handlers keyed by intent name.
//!
//! A consumer registers one handler per intent it cares about, then routes
//! incoming intents through the registry. The registry is the only place
//! where a system ever matches on `intent.name` against more than one value.

use crate::{Intent, IntentError};
use serde_json::Value;
use std::collections::HashMap;

/// Trait for typed intent handlers. Each handler is responsible for parsing
/// its own payload.
pub trait IntentHandler: Send + Sync {
    /// The intent name this handler consumes.
    fn name(&self) -> &'static str;
    /// Apply the intent. Return `Ok(())` on success, `Err(IntentError)` on
    /// schema violation (the handler is responsible for surfacing more
    /// domain-specific errors via Bevy messages or other channels).
    fn handle(&self, payload: &Value) -> Result<(), IntentError>;
}

/// In-process registry of intent handlers, keyed by name.
#[derive(Default)]
pub struct IntentRegistry {
    handlers: HashMap<&'static str, Box<dyn IntentHandler>>,
}

impl IntentRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a handler. If a handler for the same name already exists it
    /// is replaced. Returns `self` for chaining.
    pub fn register<H: IntentHandler + 'static>(mut self, handler: H) -> Self {
        self.handlers.insert(handler.name(), Box::new(handler));
        self
    }

    /// Number of registered handlers.
    pub fn len(&self) -> usize {
        self.handlers.len()
    }

    /// True if the registry has no handlers.
    pub fn is_empty(&self) -> bool {
        self.handlers.is_empty()
    }

    /// True if a handler is registered for `name`.
    pub fn has(&self, name: &str) -> bool {
        self.handlers.contains_key(name)
    }

    /// Names of all registered handlers, in arbitrary order.
    pub fn names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.handlers.keys().copied()
    }

    /// Dispatch an intent to its registered handler. Returns
    /// `IntentError::UnknownIntent` if no handler matches.
    pub fn dispatch(&self, intent: &Intent) -> Result<(), IntentError> {
        match self.handlers.get(intent.name) {
            Some(h) => h.handle(&intent.payload),
            None => Err(IntentError::UnknownIntent(intent.name.to_string())),
        }
    }
}
