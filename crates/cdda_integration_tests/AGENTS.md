# cdda_integration_tests

## Purpose
Test-only workspace member that hosts integration tests spanning multiple crates. Compiled only by `cargo test --workspace`; not part of the runtime graph.

## Ownership
- Has no library code. The only Rust source is the empty stub `src/lib.rs` (required by Cargo to recognize the crate as a library).
- The actual test code lives under `tests/`, one file per scenario.
- Workspace members that any test depends on are declared in this crate's `[dev-dependencies]`.

## Local Contracts
- **No runtime code.** Do not add modules to `src/`. New test scenarios are added as new files in `tests/`.
- **Test target only.** `publish = false` in `Cargo.toml`. The crate is not published, not depended on by any other workspace member, and not part of `cargo build --workspace`.
- **Cross-cutting suites include:**
  - `crafting_time_test.rs` — exercises `cdda_sim::actor::turn` AP costs, `cdda_sim::crafting::systems::{start_craft, complete_craft}`, and the `cdda_sim::inventory` interaction. Owner in spirit: `cdda_sim::crafting`.
  - `equipment_system_test.rs` — exercises `cdda_sim::equipment::systems::*` against `cdda_components::{actor, def, item, schedule, sim}`. Owner in spirit: `cdda_sim::equipment`.
  - `screen_integration_test.rs` — exercises `cdda_context` + `cdda_input` end-to-end. Owner in spirit: `cdda_context`.

## Work Guidance
- A new integration test belongs here when it cannot live in a single crate's `tests/` because it imports 2+ workspace members as runtime API (not just as test helpers).
- If a test "grows up" and only needs one crate, move it into that crate's `tests/`. Update the index below and prune the dep from `[dev-dependencies]`.
- The crate's `Cargo.toml` is the canonical list of workspace deps any test can `use`. Add a dep there before referencing it from a test file.

## Verification
- `cargo nextest run -p cdda_integration_tests` runs these suites (fall back to `cargo test -p cdda_integration_tests` if `nextest` is unavailable). The `TestBed`-based simulation tests in here require the same setup as the per-crate suites.
- The integration suites compile against the dev-deps declared in `Cargo.toml`; no transitive adds expected.
- The pre-existing warning about `OverlayBlockScreen` being unused is intentional scaffolding and not a defect.

## Child DOX Index
- `src/lib.rs` — Empty stub. No further durable sub-boundaries.
- `tests/native_inventory_test.rs` — Strict import, stable variants, failed publication rollback, craft output across reload, ingredient reservation, pocket lifetimes, imported-capacity transfers, nested crafting access, snapshot-safe explicit merging, master-derived whole-budget craft ticks, same-turn menu ingress and immediate result bookkeeping, contested starts, validated interruption/resume and native lifecycle commands with pause/priority/rejection; fixture under tests/fixtures.
- `tests/crafting_time_test.rs` — AP-driven crafting flow over multiple turns.
- `tests/equipment_system_test.rs` — Wielding, wearing, and slot management end-to-end.
- `tests/screen_integration_test.rs` — `cdda_context` + `cdda_input` plugin registration and navigation.
