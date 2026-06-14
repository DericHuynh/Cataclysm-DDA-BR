# cdda_intent

## Purpose
The workspace's canonical string-typed intent vocabulary — the input/output language between systems, UI, AI, and replay. An `Intent` is a stable string name plus a `serde_json::Value` payload.

## Ownership
- A new foundational crate, added to Layer 2 alongside `cdda_components` and `cdda_events`.
- No Bevy deps — pure Rust with `serde` + `serde_json` + `thiserror`. Usable from any layer.
- Consumers reach into the public surface at `cdda_intent::{Intent, IntentError, parse_intent}` and `cdda_intent::registry::{IntentHandler, IntentRegistry}`.

## Local Contracts
- **Intent is `(name, payload)`.** `name: &'static str` is a stable, lower_snake_case identifier. `payload: Value` is untyped JSON. Producers build intents; consumers parse them.
- **`parse_intent!` macro** extracts a typed local tuple from a payload. Returns `Result<(tuple), IntentError>` — the macro is the only sanctioned way to read a payload. No inline `payload.get("foo").unwrap()`.
- **IntentRegistry** (`cdda_intent::registry`) is the only sanctioned place that matches on `intent.name` against more than one value. Systems register one `IntentHandler` per intent they consume, and the registry dispatches.
- **Intents are untyped at the wire.** `Intent: Serialize + Deserialize`. Producers and consumers are coupled only by the intent name + payload schema, not by a Rust type. New intents can be added without breaking existing consumers.

## Work Guidance
- Adding a new intent: pick a `lower_snake_case` name, document the payload shape at the producer's call site (no need to register the name with `cdda_intent` itself), and add a handler in each consumer crate.
- Adding a new intent handler: implement `IntentHandler` (returns the name + handles the payload), then `IntentRegistry::new().register(MyHandler)`. The registry is per-system, not global; multiple systems can each have their own registry.
- New payload shapes use `serde_json::json!` at the producer. The `parse_intent!` macro at the consumer side gives a typed read.
- **Do not** add a typed enum like `pub enum IntentKind { Move, Cancel, ... }` — that's the path we left. The whole point of this crate is the absence of that enum.
- **Do not** put Bevy types in this crate. It is intentionally a leaf that the rest of the workspace depends on.

## Verification
- `cargo test -p cdda_intent` runs the 10 integration tests in `tests/basic.rs` covering construction, `parse_intent!` success/failure modes, and `IntentRegistry` dispatch.
- Cross-crate impact: any consumer of the `GameAction` enum (`cdda_components::input`) is a candidate for migration. New code that emits or consumes intents should use `cdda_intent` directly. The 167-variant `GameAction` stays for now; conversion happens opportunistically at each call site.

## Child DOX Index
- `src/lib.rs` — `Intent` struct, `IntentError` enum, `parse_intent!` macro.
- `src/registry.rs` — `IntentHandler` trait and `IntentRegistry` (the router).
- `tests/basic.rs` — 10 tests covering construction, parsing, error reporting, and registry dispatch.
