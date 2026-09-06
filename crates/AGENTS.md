# Crates DOX

## Purpose
Owns the Cargo workspace — 17 member crates, including native catalog and generic UI boundaries. Crate boundaries are aligned with dependency-layer separation and incremental-compile cost.

## Ownership
- The workspace manifest is at the repository root: `Cargo.toml` and `Cargo.lock`.
- Dependency direction is strictly bottom-up — a crate may only depend on crates at its own layer or below. See `CURRENT_ARCHITECTURE.md` for the layer diagram and `TARGET_ARCHITECTURE.md` § Dependency Direction for the rule.
- Bevy 0.18 is pinned at the workspace level. New Bevy features must be enabled uniformly across the workspace, not per crate.

## Local Contracts
- **Layer 1 — pure domain types** (no Bevy ECS): `cdda_core_types`.
- **Layer 1.5 — planner-core library** (bevy_ecs only, no `cdda_*` deps): `cdda_htn` (developed standalone as `bevy_bhtn`, moved into the workspace, and renamed — the `cdda_htn` name it now owns was vacated by the removed reflection-based planner). Any Layer ≥ 2 crate may depend on it; it must never depend on a `cdda_*` crate. The game integration seam is `cdda_sim::ai::htn`.
- **Layer 2 — ECS components and shared schedule**: `cdda_components` (the single home for all shared domain `Component`s and event/message types), `cdda_sim` (the runtime harness plus all consolidated game-logic submodules).
- **Layer 3 — game logic** (Bevy ECS only, no full Bevy): all of the consolidated `cdda_sim::{actor, ai, activity, combat, crafting, equipment, inventory, item, noise}` submodules.
- **Layer 4 — world and data crates**: `cdda_data`, `cdda_overmap`, `cdda_overmap_gen`.
- **Layer 5 — app shell** (full Bevy, binaries): `cdda_context`, `cdda_input`, `cdda_render`, `cdda_replay`, `cdda_app`, `cdda_cli`.
- **UI input adapters live in `cdda_render` (`render/input.rs`), never `cdda_sim`.** `cdda_sim` is the pure use-case layer and must not match the display-UI `GameAction` enum. This is the workspace's "presenter-above-sim" contract: new screen-keyboard handlers go in `cdda_render`, and `cdda_sim` exposes use-case functions for them to call.
- **Shared domain components live in `cdda_components`; authoritative operations live in their owning simulation subsystem.** Read shared components/read models across domains. Route invariant-sensitive mutations through shared validating operations rather than duplicating validation or directly calling another domain's scheduled system. Bevy relationships alone do not enforce gameplay ownership/capacity/cost invariants.
- **One headless simulation contract:** `cdda_sim::runtime::SimulationPlugin` owns `SimulationTurn`, command ingress, action/activity dispatch, post-commit refresh, gameplay plugin wiring, time and pause. The app supplies input/render/world adapters; tests use the same persistent schedule. `GameSet` orders outer Update adapters; `SimSet` orders logical simulation only. See cdda_sim/AGENTS.md for the explicit remaining AP-budget and command-routing work.
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
- `crates/cdda_defs_raw/AGENTS.md` — The 139 raw JSON def structs. Typed AST layer of the data pipeline; no Bevy, no logic.

Layer 2 — ECS components and shared schedule:

- `crates/cdda_components/AGENTS.md` — All Bevy ECS components (actor, item, activity, def, schedule, messages, events, stats, tokens), semantic event/message types, and the cross-domain coordination contract (marker components + shared data + `States`).
- `crates/cdda_catalog/AGENTS.md` — Load-free definition indexes, normalized inventory catalog, session interners and native HTN input.
- `crates/cdda_sim/AGENTS.md` — `AppState` + `TestBed` runtime harness **plus** every game-logic submodule (actor, ai, activity, combat, crafting, equipment, inventory, item, noise). The single source of truth for the simulation engine.

Layer 3 — game logic (Bevy ECS only):

- The nine game-logic submodules all live in `cdda_sim`. See the `src/<area>/` index in `crates/cdda_sim/AGENTS.md` (including `src/ai/htn/`, the HTN game integration over `cdda_htn`). The former `cdda_htn` crate has been **removed** — its replacement is the workspace-internal `crates/cdda_htn` planner core (the crate formerly named `bevy_bhtn`) plus the `cdda_sim::ai::htn` integration module.

Layer 4 — world and data:

- `crates/cdda_data/AGENTS.md` — JSON ingest → resolve → `DefRegistry` → `build_def_world`; `copy-from` resolver; schema generation.
- `crates/cdda_overmap/AGENTS.md` — Overmap chunk storage, terrain registry, spatial index, serialization. Current layering debt: depends on `cdda_sim`.
- `crates/cdda_overmap_gen/AGENTS.md` — Overmap generation pipeline (Bevy ECS systems).

Layer 5 — app shell (full Bevy, binaries):

- `crates/cdda_context/AGENTS.md` — Headless Ctx state machine, navigation, focus, overlays, menu, and Bevy `SubStates` nested-menu types (`SettingsTab`). Current layering debt: depends on `cdda_sim::runtime::state::AppState` for state transitions.
- `crates/cdda_input/AGENTS.md` — Input plugin, action bridging, keybinding maps.
- `crates/cdda_ui/AGENTS.md` — Generic Bevy ECS/UI scrolling, virtual-list geometry and retained keyed rows; no gameplay dependencies.
- `crates/cdda_render/AGENTS.md` — UI rendering, ASCII viewport, tile rendering, theming, and mouse-driven menu picking (`On<Pointer<Click>>`). May read `cdda_overmap_gen` resources for overmap preview/config UI.
- `crates/cdda_replay/AGENTS.md` — Deterministic session recording and replay.
- `crates/cdda_app/AGENTS.md` — Binary entry point (`cdda`); wires all subsystems and Bevy `DefaultPlugins`.
- `crates/cdda_cli/AGENTS.md` — `cdda-cli` binary: `schema`, `gen-schemas`, `validate`, `stats`, `check`, `ablation`, `city-view`. Mod developer tools.

Test-only:

- `crates/cdda_integration_tests/AGENTS.md` — Test-only crate that hosts cross-crate integration tests (crafting time, equipment system, screen integration). Has no library; `cargo build` does not compile it.
