# cdda_ai DOX

## Purpose
Owns AI behavior and pathfinding.

## Ownership
- AI systems and behavior decisions live in this crate.
- Pathfinding helpers may coordinate with `cdda_overmap` when operating on overmap data.

## Local Contracts
- AI should consume actor/world state without depending on render or input crates.
- Behavior decisions should be testable independently where practical.

## Work Guidance
- Prefer small behavior units over monolithic AI systems.
- Keep pathfinding assumptions explicit in code or tests.

## Verification
- Run `cargo check -p cdda_ai`.
- Run `cargo test -p cdda_ai` when AI behavior changes.

## Child DOX Index
