# Crates DOX

## Purpose
Owns the Cargo workspace — 22 member crates organized into five dependency layers plus the binary entry points. Each member crate has its own `AGENTS.md` indexed below.

## Ownership
- The workspace manifest is at the repository root: `Cargo.toml` and `Cargo.lock`.
- Dependency direction is strictly bottom-up — a crate may only depend on crates at its own layer or below. See `CURRENT_ARCHITECTURE.md` for the layer diagram and `TARGET_ARCHITECTURE.md` § Dependency Direction for the rule.
- Bevy 0.18 is pinned at the workspace level. New Bevy features must be enabled uniformly across the workspace, not per crate.

## Local Contracts
- **Layer 1 — pure domain types** (no Bevy ECS): `cdda_core_types`.
- **Layer 2 — ECS components and shared schedule**: `cdda_components`, `cdda_events`, `cdda_sim`.
- **Layer 3 — game logic crates** (Bevy ECS only, no full Bevy): `cdda_actor`, `cdda_item`, `cdda_activity`, `cdda_combat`, `cdda_crafting`, `cdda_equipment`, `cdda_inventory`, `cdda_ai`, `cdda_noise`.
- **Layer 4 — world and data crates**: `cdda_data`, `cdda_overmap`, `cdda_overmap_gen`.
- **Layer 5 — app shell** (full Bevy, binaries): `cdda_context`, `cdda_input`, `cdda_render`, `cdda_replay`, `cdda_app`, `cdda_cli`.
- No crate may depend on `cdda_app` or `cdda_cli`. Those are leaf entry points.
- A crate that would need a reverse-layer dep must extract the shared types into a new crate (see `TARGET_ARCHITECTURE.md` § No Circular Dependencies).
- `Cargo.toml` workspace dependency table is the single source of truth for crate-to-crate versions; crate manifests must use `path = "..."` plus `workspace = true` style entries, not ad-hoc versions.

## Work Guidance
- When adding a new crate, decide its layer first, then add the entry to the root `Cargo.toml` `[workspace.members]` and the appropriate workspace deps block, then add a child `AGENTS.md` in this folder and link it from the index below.
- When splitting an existing crate, move the relevant `AGENTS.md` content with the code; do not duplicate contracts in both the old and new docs.
- Crate-local tests live under `crates/<crate>/tests/`. Workspace-wide tests live under `tests/` (see `tests/AGENTS.md`).
- Profile and feature defaults live in the root `Cargo.toml`; per-crate `[features]` blocks must be additive and named in `snake_case`.

## Verification
- `cargo check --workspace` for compile sanity.
- `cargo nextest run --workspace` (or `cargo test --workspace` if `nextest` is unavailable) for the full suite.
- `cargo metadata --format-version 1` to confirm a clean dep graph; circular deps will fail the build.

## Child DOX Index

Layer 1 — pure domain types (no Bevy ECS):

- `crates/cdda_core_types/AGENTS.md` — Value types, coordinates, `DefId<T>`, raw JSON def structs, damage model, RNG.

Layer 2 — ECS components and shared schedule:

- `crates/cdda_components/AGENTS.md` — All Bevy ECS components: actor, item, def, schedule, input, context, messages, events, stats, tokens.
- `crates/cdda_events/AGENTS.md` — Observer-based event types.
- `crates/cdda_sim/AGENTS.md` — State machine (`AppState`, `TurnState`), game time, `TestBed` test harness.

Layer 3 — game logic (Bevy ECS only):

- `crates/cdda_actor/AGENTS.md` — Creature turn scheduling, movement, bionics, effects, healing, temperature, morale, vision.
- `crates/cdda_item/AGENTS.md` — Item component type registration plugin.
- `crates/cdda_activity/AGENTS.md` — Multi-turn player activity (crafting, moving, waiting).
- `crates/cdda_combat/AGENTS.md` — Damage, hit/miss, melee and ranged combat.
- `crates/cdda_crafting/AGENTS.md` — Recipe lookup, component consumption, progress tracking.
- `crates/cdda_equipment/AGENTS.md` — Wield/wear API over Bevy `WieldedBy`/`WornOn` relationships; encumbrance not yet implemented.
- `crates/cdda_inventory/AGENTS.md` — Stacks, invlets, binned lookups, item movement.
- `crates/cdda_ai/AGENTS.md` — Monster/NPC decision making.
- `crates/cdda_noise/AGENTS.md` — 3D simplex noise matching CDDA master.

Layer 4 — world and data:

- `crates/cdda_data/AGENTS.md` — JSON ingest → resolve → `DefRegistry` → `build_def_world`; `copy-from` resolver; schema generation.
- `crates/cdda_overmap/AGENTS.md` — Overmap chunk storage, terrain registry, spatial index, serialization.
- `crates/cdda_overmap_gen/AGENTS.md` — Overmap generation pipeline (Bevy ECS systems).

Layer 5 — app shell (full Bevy, binaries):

- `crates/cdda_context/AGENTS.md` — Headless Ctx state machine, navigation, focus, overlays, menu.
- `crates/cdda_input/AGENTS.md` — Input plugin, action bridging, keybinding maps.
- `crates/cdda_render/AGENTS.md` — UI rendering, ASCII viewport, tile rendering, theming.
- `crates/cdda_replay/AGENTS.md` — Deterministic session recording and replay.
- `crates/cdda_app/AGENTS.md` — Binary entry point (`cdda`); wires all subsystems and Bevy `DefaultPlugins`.
- `crates/cdda_cli/AGENTS.md` — `cdda-cli` binary: `run`, `schedule-graph`, `render-graph`, `dump`, and mod developer tools.
