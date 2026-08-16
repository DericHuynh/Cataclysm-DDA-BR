# Crates DOX

## Purpose
Owns the Cargo workspace — 15 member crates (13 source + 1 test-only + 1 raw-def AST leaf). Crate boundaries are aligned with dependency-layer separation and incremental-compile cost.

## Ownership
- The workspace manifest is at the repository root: `Cargo.toml` and `Cargo.lock`.
- Dependency direction is strictly bottom-up — a crate may only depend on crates at its own layer or below. See `CURRENT_ARCHITECTURE.md` for the layer diagram and `TARGET_ARCHITECTURE.md` § Dependency Direction for the rule.
- Bevy 0.18 is pinned at the workspace level. New Bevy features must be enabled uniformly across the workspace, not per crate.

## Local Contracts
- **Layer 1 — pure domain types** (no Bevy ECS): `cdda_core_types`.
- **Layer 2 — ECS components and shared schedule**: `cdda_components` (the single home for all shared domain `Component`s and event/message types), `cdda_sim` (the runtime harness plus all consolidated game-logic submodules).
- **Layer 3 — game logic** (Bevy ECS only, no full Bevy): all of the consolidated `cdda_sim::{actor, ai, activity, combat, crafting, equipment, inventory, item, noise}` submodules.
- **Layer 4 — world and data crates**: `cdda_data`, `cdda_overmap`, `cdda_overmap_gen`.
- **Layer 5 — app shell** (full Bevy, binaries): `cdda_context`, `cdda_input`, `cdda_render`, `cdda_replay`, `cdda_app`, `cdda_cli`.
- **UI input adapters live in `cdda_render` (`render/input.rs`), never `cdda_sim`.** `cdda_sim` is the pure use-case layer and must not match the display-UI `GameAction` enum. This is the workspace's "presenter-above-sim" contract: new screen-keyboard handlers go in `cdda_render`, and `cdda_sim` exposes use-case functions for them to call.
- **All shared domain `Component`s live in `cdda_components`.** A domain's *data* (its components, marker components, relationships) is the cross-domain communication medium; a domain's *systems* live in the crate whose main task they serve (e.g. crafting systems in `cdda_sim::crafting`). When domain A needs data owned by domain B, it queries the shared entity's marker + components + `States` — it does **not** import B's system/function. This is how inventory ↔ crafting ↔ body-parts ↔ map tiles coordinate: via one entity carrying the relevant components/markers, not via cross-crate function calls.
- **Test-only**: `cdda_integration_tests` (no library, no `cargo build`; only `cargo test --workspace` compiles it).
- No crate may depend on `cdda_app` or `cdda_cli`. Those are leaf entry points.
- A crate that would need a reverse-layer dep must extract the shared types into a new crate (see `TARGET_ARCHITECTURE.md` § No Circular Dependencies).
- `Cargo.toml` workspace dependency table is the single source of truth for crate-to-crate versions; crate manifests must use `path = "..."` plus `workspace = true` style entries, not ad-hoc versions.

## Work Guidance
- When adding a new crate, decide its layer first, then add the entry to the root `Cargo.toml` `[workspace.members]` and the appropriate workspace deps block, then add a child `AGENTS.md` in this folder and link it from the index below.
- When splitting an existing crate, move the relevant `AGENTS.md` content with the code; do not duplicate contracts in both the old and new docs.
- Crate-local tests live under `crates/<crate>/tests/`. Workspace-wide cross-crate tests live under `crates/cdda_integration_tests/tests/`.
- Profile and feature defaults live in the root `Cargo.toml`; per-crate `[features]` blocks must be additive and named in `snake_case`.
- New game-logic code goes into the matching submodule under `crates/cdda_sim/src/<area>/`. New `cdda_sim` submodules are added by declaring them in `src/lib.rs` and indexing them in `crates/cdda_sim/AGENTS.md`.

## Verification
- `cargo check --workspace` for compile sanity.
- `cargo nextest run --workspace` (or `cargo test --workspace` if `nextest` is unavailable) for the full suite.
- `cargo metadata --format-version 1` to confirm a clean dep graph; circular deps will fail the build.

## Child DOX Index

Layer 1 — pure domain types (no Bevy ECS):

- `crates/cdda_core_types/AGENTS.md` — Value types, coordinates, `DefId<T>`, damage model, RNG. No raw def structs anymore (they moved to `cdda_defs_raw` in Phase 3a).
- `crates/cdda_defs_raw/AGENTS.md` — The 138 raw JSON def structs. Typed AST layer of the data pipeline; no Bevy, no logic.

Layer 2 — ECS components and shared schedule:

- `crates/cdda_components/AGENTS.md` — All Bevy ECS components (actor, item, activity, def, schedule, input, context, messages, events, stats, tokens), event/message types, `Ctx` states, and the cross-domain coordination contract (marker components + shared data + `States`).
- `crates/cdda_sim/AGENTS.md` — `AppState` + `TestBed` runtime harness **plus** every game-logic submodule (actor, ai, activity, combat, crafting, equipment, inventory, item, noise). The single source of truth for the simulation engine.

Layer 3 — game logic (Bevy ECS only):

- The nine game-logic submodules all live in `cdda_sim`. See the `src/<area>/` index in `crates/cdda_sim/AGENTS.md`.
- `crates/cdda_htn/AGENTS.md` — **Headless HTN planner** (forward MTR + backward goal-state), `.htn` DSL parser, reflection-driven operators. Library leaf: no ECS/`Component` and no `cdda_sim`/`cdda_components` dependency (its ECS-driven Criterion benchmark lives in dev-deps only). Adopted by `cdda_sim::ai` to drive `PlannerHtn` mobs (wiring is a follow-up).

Layer 4 — world and data:

- `crates/cdda_data/AGENTS.md` — JSON ingest → resolve → `DefRegistry` → `build_def_world`; `copy-from` resolver; schema generation.
- `crates/cdda_overmap/AGENTS.md` — Overmap chunk storage, terrain registry, spatial index, serialization. Current layering debt: depends on `cdda_sim`.
- `crates/cdda_overmap_gen/AGENTS.md` — Overmap generation pipeline (Bevy ECS systems).

Layer 5 — app shell (full Bevy, binaries):

- `crates/cdda_context/AGENTS.md` — Headless Ctx state machine, navigation, focus, overlays, menu, and Bevy `SubStates` nested-menu types (`SettingsTab`). Current layering debt: depends on `cdda_sim::runtime::state::AppState` for state transitions.
- `crates/cdda_input/AGENTS.md` — Input plugin, action bridging, keybinding maps.
- `crates/cdda_render/AGENTS.md` — UI rendering, ASCII viewport, tile rendering, theming, and mouse-driven menu picking (`On<Pointer<Click>>`). May read `cdda_overmap_gen` resources for overmap preview/config UI.
- `crates/cdda_replay/AGENTS.md` — Deterministic session recording and replay.
- `crates/cdda_app/AGENTS.md` — Binary entry point (`cdda`); wires all subsystems and Bevy `DefaultPlugins`.
- `crates/cdda_cli/AGENTS.md` — `cdda-cli` binary: `schema`, `gen-schemas`, `validate`, `stats`, `check`, `ablation`, `city-view`. Mod developer tools.

Test-only:

- `crates/cdda_integration_tests/AGENTS.md` — Test-only crate that hosts cross-crate integration tests (crafting time, equipment system, screen integration). Has no library; `cargo build` does not compile it.
