# cdda_render DOX

## Purpose
Owns rendering, UI screens, ASCII viewport, tile rendering, and theming.

## Ownership
- Bevy UI, Text2d, tile rendering, screen rendering, overmap/dev-world visualization, and theming live in this crate.
- Context state and input action contracts remain in `cdda_context` and `cdda_input`.

## Local Contracts
- Rendering systems should read game/context/input state and produce visuals only.
- Render systems should not mutate navigation or simulation state except through established events/messages.
- UI screen lifecycle should follow the `CddaScreen` plugin pattern from `cdda_context`.

## Work Guidance
- Keep render code separated by screen or visual subsystem.
- Prefer data-driven UI where existing screen definitions support it.
- Coordinate with `cdda_input` when visual screens consume semantic input.

## Verification
- Run `cargo check -p cdda_render`.
- Run `cargo run -p cdda_app` for visual/runtime smoke validation when render behavior changes.
- Run relevant screen integration tests when UI navigation contracts change.

## Child DOX Index
