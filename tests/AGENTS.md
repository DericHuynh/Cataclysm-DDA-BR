# Tests DOX

## Purpose
Owns the workspace-level integration and regression test suite (`cargo test --workspace`).

## Ownership
- These are cross-crate tests that need more than one workspace member to be in scope. Per-crate unit and integration tests live next to the crate they test (e.g. `crates/cdda_actor/tests/`).
- Each `*.rs` file at this level targets a specific surface (data loading, hot reload, screens, systems, etc.) and typically pulls in `cdda_sim` or `cdda_data` resources to drive an end-to-end scenario.

## Local Contracts
- Workspace integration tests share the same `cdda_sim::test_utils` harness as the crate-level tests — prefer the harness over bespoke `App` setup.
- Tests in this folder must not depend on the Bevy renderer (`cdda_render`) or any platform-specific plugin; keep them headless.
- Test file naming reflects the area under test: `def_world_load.rs` exercises `cdda_data`, `inventory_system_test.rs` exercises `cdda_inventory`, etc.

## Work Guidance
- When adding a new cross-cutting concern, add a new test file here rather than nesting into a crate's `tests/`.
- When fixing a regression, add a test here (or in the owning crate) that fails before the fix and passes after.
- Long-running tests should be tagged appropriately or moved under a feature flag — keep `cargo test --workspace` fast.

## Verification
- `cargo test --workspace` is the canonical full-suite run. `cargo nextest run` is preferred per the repo's standard (see prior Bevy 0.18 reference text in the root doc history).
- Crate-level `cargo test -p <crate>` for targeted work.

## Child DOX Index
No durable sub-boundaries; tests are flat, peer-level files grouped by area under test.
