# CDDA-BR — Cataclysm: Dark Days Ahead in Rust + Bevy ECS

A reimplementation of CDDA in Rust, targeting maintainability,
performance, and first-class modding support.

## Remember to route in OpenTelemetry for Events
Example: https://github.com/madesroches/optimism

## Architecture

- **[CURRENT_ARCHITECTURE.md](CURRENT_ARCHITECTURE.md)** — What's implemented now.
  The Stage 1 JSON loading pipeline, crate layout, and design principles currently
  in effect.

- **[TARGET_ARCHITECTURE.md](TARGET_ARCHITECTURE.md)** — Where we're going.
  The ACL pattern, numeric IDs, component-based templates, SoA tile storage,
  event-driven systems, three-tier hot reload via subsecond, and the full migration
  path.

## Quick Start

```bash
# Run all tests (excluding Bevy rendering/audio crates)
cargo test --workspace

# Build and run
cargo run -p cdda_app
```

## Crate Overview

| Crate | Purpose | Bevy Deps |
|-------|---------|-----------|
| `cdda_core` | Pure domain types (units, coords, IDs) | None |
| `cdda_data` | JSON loading, copy-from resolver, ACL | None |
| `cdda_sim` | Simulation: components, systems, tick loop | `bevy_ecs`, `bevy_reflect` |
| `cdda_map` | Map storage, mapgen, FOV, pathfinding | None |
| `cdda_render` | Bevy rendering plugin (tiles, UI, ASCII) | Full `bevy` |
| `cdda_input` | Bevy input plugin (keybinds, contexts) | Full `bevy` |
| `cdda_audio` | Bevy audio plugin | Full `bevy` |
| `cdda_app` | Binary entry point + hot-reload boundaries | Full `bevy` |
