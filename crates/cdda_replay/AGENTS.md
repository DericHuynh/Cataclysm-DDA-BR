# cdda_replay DOX

## Purpose
Deterministic session recording, replay, and state hashing for CDDA-BR. Captures
player `InputAction` messages into a `SessionLog`, replays them injectively, and
(gated by the `devtools` feature) hashes the `SimId` set per turn to detect
simulation drift between runs.

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
- Two plugins (mutually exclusive modes, registered separately):
  - `CddaReplayPlugin { world_seed: u64 }` — recording mode. Inserts
    `SessionLog::new(world_seed)` and adds `recording::record_actions` to
    `Update`.
  - `CddaReplayModePlugin` — replay mode. Inserts `ReplayState::default()` and
    adds `replay::inject_replay_actions` to `Update`.
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
- State hashing (`state_hash.rs`): `hash_simulation_state` queries
  `Query<&SimId>`, sorts the collected `u64`s, and feeds entity count + each
  id into a `DefaultHasher`. Both `StateHashLog.hashes` and
  `SessionLog.state_hashes` are appended with `(turn, hash)`.
  `check_divergence` (replay-only) compares the live vs recorded hash for
  `game_time.turn` and writes a `SimulationDiverged { turn, detail }` message
  on mismatch.
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
- `cargo nextest run -p cdda_replay` (or `cargo test -p cdda_replay`) covers
  the `SessionLog` round-trip / corruption / file I/O tests in
  `tests/session_log_test.rs` and the `ReplayState` / `ReplaySpeed` defaults
  and `is_complete` tests in `tests/replay_state_test.rs`.

## Child DOX Index
- *(none — `src/` and `tests/` are flat, no durable sub-boundaries yet)*
