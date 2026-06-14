# cdda_components DOX

## Purpose
Owns shared ECS components, schedule sets, and input/action contracts.

## Ownership
- Shared components, messages, events, context state, schedule sets, and input/action enums live in this crate.
- Type-specific systems belong in their owning subsystem crates.

## Local Contracts
- `GameSet` and `SimSet` define the shared schedule contract for simulation and rendering.
- `GameAction`, `BindableAction`, and `InputAction` define the semantic input contract.
- Systems should consume semantic input messages instead of reading raw `ButtonInput<KeyCode>` directly.

## Work Guidance
- Add components here only when they are shared across multiple crates or define durable ECS contracts.
- Keep input/action contracts aligned with `cdda_input`.

## Verification
- Run `cargo check -p cdda_components`.
- Run `cargo test -p cdda_components` when shared contracts change.

## Child DOX Index
