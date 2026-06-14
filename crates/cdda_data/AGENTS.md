# cdda_data DOX

## Purpose
Owns JSON loading, definition registry, copy-from inheritance, and data artifacts.

## Ownership
- Loader, schema generation, definition registry, and definition-world construction live in this crate.
- Raw data files under `data/` remain owned by `data/AGENTS.md`.

## Local Contracts
- The loader is the authoritative path from JSON definitions to `DefRegistry` and Bevy definition entities.
- `copy-from` inheritance must remain deterministic and schema-aware.

## Work Guidance
- Keep parsing, resolution, and entity spawning concerns as separated as practical.
- Add schema or loader tests when changing definition contracts.

## Verification
- Run `cargo check -p cdda_data`.
- Run `cargo test -p cdda_data` for loader/schema changes.
- Run `cargo run -p cdda-cli -- check data/core` for end-to-end data validation.

## Child DOX Index
