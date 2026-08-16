# cdda_htn AGENTS.md

## Purpose
Headless **Hierarchical Task Network** planner for CDDA AI. Imports `.htn` files, executes strongly-typed operators via `bevy_reflect`, and runs both **forward** (MTR backtracking over method decomposition) and **backward / goal-state** planning (given a `goal_task`). Combines the classic bevy_htn (forward, DSL, MTR) and bevy_bae (data-driven, composable) shapes into one idiomatic, reflection-driven crate.

## Ownership
- Lives at `crates/cdda_htn/`. The **library (`src/`)** is a leaf: it must **not** depend on ECS (`Component`), `cdda_sim`, `cdda_components`, `cdda_data`, or any world/data crate. Its only Bevy library ties are `bevy_reflect` (reflection) and `bevy_asset` (the optional `.htn` asset loader). Dev-dependencies (tests/benchmarks) **may** pull in `bevy_ecs` to exercise the planner the way a game would — but never a reverse layer dependency.
- Domain data (`HtnDomain`, `Task`, `Method`, `PrimitiveTask`, `HtnCondition`, `Effect`, `Operator`) is plain, reflection-free data — only the **plan state** type needs `Reflect`.
- The library stays headless-testable: planners take `<S: HtnState>` (a `Reflect + Default + Clone + Debug` struct) and never require an `Entity`/`World`.

## Local Contracts
- **API layout**: `parse_htn` (DSL) → `HtnDomain` → `HtnPlanner` (forward) / `BackPlanner` (backward). `Operators` resolve registered `Reflect` types in a `TypeRegistry`.
- **Backward planning** is deliberately **greedy reverse chaining** (a cheap stand-in for full goal search): it picks the operator that covers the most currently-needed goal fields, applies its effects to a working copy, and repeats. It returns `HtnError::NoPlan` if it plateaus.
- **MTR** is forward-only; backward plans carry an empty MTR.
- Error handling goes through `HtnError` / `HtnResult`.
- Public API + `prelude` are the seam; keep internals private unless a planner genuinely needs them.

## Work Guidance
- Add/modify grammar in `src/htn.pest` + the pest-parser in `src/dsl.rs` together; keep them 1:1.
- Conditions/effects mutate/read via `bevy_reflect` 0.18 idioms (e.g. `reflect_ref().as_struct()`, `try_downcast_ref`, `reflect_partial_eq`, `reflect_clone`). Do not reintroduce pre-0.18 APIs (`Ref::`, `Reflect::Struct`, `apply_boxed`).
- New planner algorithm → new module under `src/`, expose through `prelude`, and add tests via the shared `tests/common/mod.rs` `HtnTestBed` mirroring `tests/htn_planner.rs`. New/edited `.htn` domains should also be added to `tests/htn/` and pinned in `tests/htn_parse.rs` so the file API stays conformant with the reference `bevy_htn` examples.
- When wiring into `cdda_sim::ai` later: `cdda_sim` may depend on `cdda_htn`, never the reverse.
- **Tests** use the `HtnTestBed` pattern: `tests/common/mod.rs` wraps a parsed `HtnDomain` + `TypeRegistry` and exposes `plan_forward` / `plan_backward`. `tests/htn_planner.rs` pins forward planning (incl. backtracking + idempotent goals), backward planning (reachable + unreachable goals), plan *execution* (applying planned effects to reach the terminal state), and DSL/condition/effect details. `tests/htn_parse.rs` pins the parser against the reference `bevy_htn` example `.htn` files (in `tests/htn/`) so file-API conformance stays stable. `tests/htn_features.rs` pins the **full condition/effect variant matrix** (int/float/`Option`/enum/identifier comparisons incl. negation, all `Set*`/`Increment*` effects), `verify`/`verify_operator` error paths, `expected_effects` chaining, MTR/`Plan::is_preferred_over` ordering, domain helpers (`root_task`/`goal`/`primitive_names`), BackPlanner greedy tie-breaking + multi-leaf composition, `SetIdentifier` verify, and `Parser`/`UnknownTask` error shapes. `tests/htn_nested.rs` pins **deep nesting** (multi-level compound decomposition order) and **mid-execution replanning** (re-planning against a mutated world state each turn — the stateless-planner equivalent of world state changing while a plan runs).
- **Benchmarks** live in `benches/ai_throughput.rs` (Criterion). They run the planner **through real Bevy ECS**: 200k miner entities are spawned (each carrying a `MinerState` component) and a registered AI system iterates them each frame via `World::run_system`, writing a `Plan` component per entity — plus a single-actor latency case. This measures the same planner-through-query path the game uses, not just raw function calls.

## Verification
- `cargo check -p cdda_htn`
- `cargo test -p cdda_htn` — `tests/htn_parse.rs` (3: reference fixtures) + `tests/htn_planner.rs` (9: forward, backward, execution) + `tests/htn_features.rs` (20: condition/effect matrix, verify errors, operators, MTR ordering, expected-effects, Back/execution, domain helpers, error variants) + `tests/htn_nested.rs` (3: deep nesting, mid-execution replanning) via `tests/common/mod.rs`.
- `cargo bench -p cdda_htn --bench ai_throughput` — planner-through-ECS throughput over 10k / 50k / 200k entities and per-actor latency.
- `cargo check --workspace` and `cargo test --workspace` (full suite) must stay green after adopting the crate.

## Child DOX Index
None. This crate owns no child `AGENTS.md` files.