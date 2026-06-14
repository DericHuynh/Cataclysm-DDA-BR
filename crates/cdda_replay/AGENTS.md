# cdda_replay DOX

## Purpose
Owns deterministic replay, session logging, and state hashing.

## Ownership
- Replay plugins, session recording, replay playback, and deterministic state hashing live in this crate.
- Deterministic entity IDs and RNG primitives remain in `cdda_core_types`.

## Local Contracts
- Replay output should be deterministic for the same seed and input stream.
- Replay systems should not depend on render or input crates.

## Work Guidance
- Keep replay serialization stable and versioned where practical.
- Use deterministic IDs and RNG from core types for reproducible sessions.

## Verification
- Run `cargo check -p cdda_replay`.
- Run `cargo test -p cdda_replay` when replay behavior changes.

## Child DOX Index
