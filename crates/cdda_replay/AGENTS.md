# cdda_replay DOX

## Purpose
Deterministic session recording, replay, and state hashing for CDDA-BR. Captures
player `InputAction` messages into a `SessionLog`, replays them injectively, and
(gated by the `devtools` feature) hashes committed gameplay state per turn to
detect simulation drift between runs.

## Ownership
- The two `bevy_app::Plugin` entry points, the postcard `SessionLog` format, the
  replay state machine, and the state hash + divergence systems all live here.
- `SimId` (deterministic entity tag) is owned by `cdda_core_types::sim_id`.
- `InputAction` / `GameAction` / `ActionSource` are owned by
  `cdda_components::input`; `GameTime` is owned by `cdda_components::sim` (not
  `cdda_sim`).
- Does not depend on `cdda_input`, `cdda_render`, or any other Layer 5 crate.

## Local Contracts
- Runtime deps: `bevy_ecs`, `bevy_app`, `cdda_core_types`, `cdda_components`,
  `serde`, `postcard`, `tracing`. Dev-dep: `tempfile` (file round-trip tests).
- Single `devtools` feature flag (no deps). When enabled it adds the
  `StateHashLog` resource, the `hash_simulation_state` and `check_divergence`
  systems, and registers `SimulationDiverged` as a `Message`. The hash system
  is a no-op in non-`devtools` builds via a `cfg!` early-return.
- Two plugins (mutually exclusive modes, registered separately), both phase-ordered
  against the canonical simulation:
  - `CddaReplayPlugin { world_seed: u64 }` — recording mode. Inserts
    `SessionLog::new(world_seed)` and adds `recording::record_actions` in
    `GameSet::Input` (before the simulation driver consumes the frame).
  - `CddaReplayModePlugin` — replay mode. Inserts `ReplayState::default()` and
    adds `replay::inject_replay_actions` in `GameSet::Input`.
- Recording pipeline (`recording.rs`): the `record_actions` system reads
  `InputAction` messages and the `GameTime` resource, appending an
  `ActionRecord { turn, action, source }` to `SessionLog.actions` each frame
  the reader fires. Stamp is `game_time.turn` at message-drain time, not at
  `InputAction` creation.
- Replay state machine (`replay.rs`):
  - `ReplayState { cursor: usize, speed: ReplaySpeed, paused: bool }` resource.
    `Default` is cursor 0, `Fast`, not paused. Exposes `is_complete(&log)`.
  - `ReplaySpeed` enum: `RealTime | Fast | Step` (no `Copy` data, derive
    `Copy`+`Eq`).
  - `inject_replay_actions` is a no-op while `paused`. In `Fast` it drains
    every record whose `turn == game_time.turn` and skips past records with
    `turn < game_time.turn` (resync). In `RealTime | Step` it emits at most
    one record per call, rewriting `source` to `ActionSource::Script`.
- State hashing (`state_hash.rs`): `hash_simulation_state` (exclusive) digests
  MEANINGFUL GAMEPLAY STATE, not just entity membership: per non-`IsDef`
  entity the stable `SimId` (Entity bits ONLY as an intra-world test fallback),
  world position, AP, health, stack count, plus the stable id of its
  `InsideContainer`/`WieldedBy`/`WornOn` owner; rows are sorted by stable id so
  spawn order does not change the digest; the turn is hashed in. It runs in
  `GameSet::Render` — AFTER the simulation driver — so the digest always
  reflects COMMITTED state. The live history `StateHashLog.hashes` is always
  appended; `SessionLog.state_hashes` (the EXPECTED log) is appended ONLY in
  recording mode (no `ReplayState` resource) — a replay never mutates the log
  it is compared against. `compute_state_hash(&mut World)` is the pure entry
  point the tests use. `check_divergence` (replay-only) compares the live vs
  recorded hash for `game_time.turn` and writes a
  `SimulationDiverged { turn, detail }` message on mismatch. Pinned by
  `tests/state_hash_test.rs` (`#![cfg(feature = "devtools")]`): state changes
  change the digest, spawn order does not, ownership edges are covered, and
  replay/recording append behavior is enforced.
- Remaining replay debt: recording still captures `InputAction` messages with
  turn-at-drain stamps (not semantic commands with sequence numbers); the
  digest does not yet include RNG state or a definition-version tag; and the
  `Fast` vs `RealTime|Step` replay speeds still differ on missed turns.
- Session log format (`session_log.rs`): postcard binary via `to_bytes` /
  `from_bytes` and `save_to_file` / `load_from_file`. Schema is
  `{ world_seed: u64, actions: Vec<ActionRecord>, state_hashes: Vec<(u64, u64)> }`.
  There is **no explicit version field** — forward compat depends on
  `#[serde(default)]` on `state_hashes`; new top-level fields will break old
  blobs. The `save_compressed` / `load_compressed` helpers currently write
  raw postcard bytes (a `// TODO` left in the implementation; the
  `compressed_file_is_smaller_than_uncompressed_for_large_log` test is
  `#[ignore]`d because of this).

## Work Guidance
- `GameTime` lives in `cdda_components::sim`, not `cdda_sim`. If the import
  path `cdda_components::sim::GameTime` breaks, look for a move into a
  re-export, not a `cdda_sim` dependency.
- Adding a new `ActionRecord` field is a wire-format break. Add it behind a
  `#[serde(default)]` and consider introducing a real version tag at the same
  time.
- Both plugins can technically coexist; don't rely on that — callers pick one
  mode per app.
- `default()` for `SessionLog` produces `world_seed = 0`; production callers
  must use `SessionLog::new(seed)` so replays are reproducible.

## Verification
- `cargo check -p cdda_replay` for compile sanity, including with
  `--features devtools` and `--no-default-features`.
- `cargo nextest run -p cdda_replay` covers the `SessionLog` round-trip / corruption / file I/O tests in `tests/session_log_test.rs` and the `ReplayState` / `ReplaySpeed` defaults and `is_complete` tests in `tests/replay_state_test.rs` (fall back to `cargo test -p cdda_replay` if `nextest` is unavailable).
- `cargo nextest run -p cdda_replay --features devtools` additionally runs `tests/state_hash_test.rs` — the deterministic-digest regressions (state sensitivity, spawn-order invariance, ownership edges, immutable expected log). Without the feature that file compiles to nothing.

## Child DOX Index
- *(none — `src/` and `tests/` are flat, no durable sub-boundaries yet)*
