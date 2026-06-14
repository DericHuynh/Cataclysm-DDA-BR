# cdda_actor DOX

## Purpose
Owns creature actor state, scheduling, effects, bionics, morale, temperature, movement, and vision.

## Ownership
- Actor components and systems live in this crate.
- Shared actor-related components used by many crates remain in `cdda_components`.

## Local Contracts
- Actor systems should update actor state through ECS queries and established events.
- Avoid coupling actor logic to rendering or input crates.

## Work Guidance
- Keep movement and perception logic reusable by gameplay and AI systems.
- Preserve deterministic actor simulation where possible.

## Verification
- Run `cargo check -p cdda_actor`.
- Run `cargo test -p cdda_actor` when actor behavior changes.

## Child DOX Index
