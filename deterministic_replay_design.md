# Deterministic Replay System
### Design Document — Bevy CDDA Rewrite

---

## Overview

A deterministic replay system that records the minimal input necessary to reproduce
an entire session from a world seed, enabling exact bug reproduction, automated
regression testing, and session sharing. The system is built on existing Bevy crates
and idiomatic ECS patterns, with zero overhead in release builds.

The core invariant:

```
f(world_seed, action_log) → identical simulation state at every turn
```

---

## The Four Sources of Non-Determinism in Bevy

Research into Bevy's determinism model identifies four distinct sources that must
each be addressed independently.

### 1. HashMap iteration order

Historically this was a critical footgun. **As of Bevy 0.16+, this is solved.**
`bevy::platform::collections::HashMap` now defaults to `FixedHasher` instead of
`RandomState`, providing determinism by default. Any internal Bevy collections used
in simulation hot paths are already safe. For your own simulation code, always
use `bevy::platform::collections::HashMap` / `HashSet` rather than `std::collections`
or `hashbrown` directly.

### 2. System execution order

Bevy's parallel executor runs systems in a non-deterministic order by default —
the order can change every frame. For simulation systems this must be locked down.

Two strategies, used in combination:

**Ambiguity detection in debug builds:**
```rust
app.edit_schedule(SimulationUpdate, |schedule| {
    schedule.set_build_settings(ScheduleBuildSettings {
        ambiguity_detection: LogLevel::Error, // hard error in dev
        ..default()
    });
});
```

**Explicit ordering for all simulation-touching systems:**
Every system that reads or writes simulation state must declare its position in
the `SimulationSet` ordering chain. No simulation system is left with an implicit
order relative to any other.

For the nuclear option during debugging, switch the executor to single-threaded:
```rust
app.edit_schedule(SimulationUpdate, |schedule| {
    schedule.set_executor_kind(ExecutorKind::SingleThreaded);
});
```
This guarantees determinism at the cost of parallelism and is useful for
isolating whether a bug is ordering-related.

### 3. Query iteration order

Bevy's query iteration order over archetypes is not guaranteed stable. Any system
that iterates a query and makes RNG calls, or accumulates into shared mutable state,
must sort by a stable key first.

The right key is **not** `Entity` — Entity IDs are assigned by spawn order which
can vary. Use a dedicated `SimId` component:

```rust
#[derive(Component, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy,
         Serialize, Deserialize, Reflect)]
pub struct SimId(pub u64);
```

`SimId` is assigned deterministically at spawn time from `(world_seed, spawn_counter)`.
It is stable across replays because it encodes *why* an entity was spawned, not
*when* in Bevy's internal allocation order.

For simulation systems that must iterate in order:
```rust
fn npc_act(
    mut q: Query<(&SimId, &mut NpcState, &mut WyRand)>,
) {
    let mut sorted: Vec<_> = q.iter_mut().collect();
    sorted.sort_by_key(|(id, ..)| *id);
    for (_, mut state, mut rng) in sorted {
        // ...
    }
}
```

The sort cost is O(n log n) per simulation tick, not per frame, which is acceptable.
For large queries, a `SimId`-keyed `BTreeMap` cache can avoid repeated sorting.

### 4. RNG

A global RNG is incompatible with both determinism and parallelism. Any system
touching the global RNG enforces a serial dependency chain, and the output depends
on call order across all systems.

**Use `bevy_rand` with per-entity `WyRand` components.** This is the correct
solution:

```toml
[dependencies]
bevy_rand = { version = "0.12", features = ["wyrand"] }
bevy_prng = { version = "0.12", features = ["wyrand"] }
```

Each entity owns its RNG. Systems accessing unrelated entities' RNGs can run in
parallel with no ordering requirement. Only systems that access the *same* entity's
RNG need ordering relative to each other.

World-gen seeding flow:
```rust
fn seed_world(mut commands: Commands, mut global: Single<&mut WyRand, With<GlobalRng>>) {
    // Each spawned entity gets a deterministic fork of the global seed.
    // The fork is derived from the global seed state at this point, so
    // spawn order determines the fork — which is why SimId + ordered
    // spawn is required upstream of this.
    commands.spawn((Npc, global.fork_seed()));
    commands.spawn((Monster, global.fork_seed()));
}
```

`WyRand` is the right algorithm choice: fast, portable, deterministic across
platforms (including wasm32 vs x86-64), and supports `Serialize`/`Deserialize`
out of the box via `bevy_rand`'s reflection support.

---

## Architecture

### Schedule layout

All simulation logic runs in a dedicated schedule, separate from rendering:

```
FixedMain
└── SimulationUpdate          ← all simulation systems, ordered set
    ├── SimulationSet::Input   ← consume player actions / replay feed
    ├── SimulationSet::Turn    ← turn resolution: NPC AI, item effects, etc.
    ├── SimulationSet::World   ← world events, chunk ticks, weather
    └── SimulationSet::Post    ← state hash capture (debug only)
```

`SimulationUpdate` runs at a fixed rate completely decoupled from frame rate.
For CDDA's turn-based model, one "tick" of `SimulationUpdate` corresponds to
one in-game turn — the rate can be driven arbitrarily fast during replay
(uncapped) or slowed for step-through debugging.

### Crate dependency summary

| Concern | Crate | Notes |
|---|---|---|
| Per-entity RNG | `bevy_rand` + `bevy_prng` (wyrand) | Reflection + serde built in |
| Input abstraction | `bevy_enhanced_input` | Action-based, not key-based |
| Serialization | `serde` + `postcard` | `postcard` for compact binary |
| State hashing | `rustc-hash` (via `bevy::platform`) | Already in Bevy's dep tree |
| Ambiguity detection | `bevy_mod_debugdump` | Dev/debug only |
| Schedule visualization | `bevy_mod_debugdump` | Render graph + schedule DOT |

---

## Input Recording with `bevy_enhanced_input`

`bevy_enhanced_input` is the correct foundation for the action log. It works
at the *action* level, not the raw input level — the log records `MoveNorth`,
not `KeyW`. This means:

- Rebinds don't break old logs
- Gamepad and keyboard inputs produce identical log entries
- The action layer is already the correct granularity for CDDA (one action = one turn)

### Action definition

```rust
#[derive(InputAction, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[input_action(output = bool)]
pub struct MoveNorth;

#[derive(InputAction, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[input_action(output = bool)]
pub struct PickUp;

// ... etc
```

### The log format

```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ActionRecord {
    pub turn: u64,
    pub action: SimAction,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum SimAction {
    Move(IVec3),        // includes z-level delta
    Attack(SimId),      // stable ID, not ECS Entity
    PickUp(SimId),
    Drop(SimId),
    Wait,
    // ... 
}

#[derive(Resource, Serialize, Deserialize)]
pub struct SessionLog {
    pub world_seed: u64,
    pub git_commit: &'static str,   // from env!("GIT_HASH") at compile time
    pub bevy_version: &'static str, // from env!("CARGO_PKG_VERSION") 
    pub recorded_at: u64,           // unix timestamp
    pub state_hashes: Vec<(u64, u64)>,  // (turn, hash) — debug builds only
    pub actions: Vec<ActionRecord>,
}
```

The `git_commit` field is set at compile time:
```rust
// build.rs
println!("cargo:rustc-env=GIT_HASH={}", git_hash());
```

This makes it impossible to confuse logs from different commits.
The `state_hashes` field is populated only in `debug` builds and stripped
in release, keeping the release log lean.

### Recording system

```rust
fn record_actions(
    mut log: ResMut<SessionLog>,
    turn: Res<TurnCounter>,
    // bevy_enhanced_input fires observers, so we listen via EventReader
    mut move_events: EventReader<Fire<MoveNorth>>,
    mut pickup_events: EventReader<Fire<PickUp>>,
    // ...
) {
    for _ in move_events.read() {
        log.actions.push(ActionRecord {
            turn: turn.current,
            action: SimAction::Move(/* resolved direction */),
        });
    }
    // ... other actions
}
```

### Replay injection system

```rust
#[derive(Resource)]
pub struct ReplayState {
    pub cursor: usize,
    pub speed: ReplaySpeed,
    pub paused: bool,
}

pub enum ReplaySpeed {
    RealTime,   // one sim tick per real fixed-update tick
    Fast,       // uncapped — run turns until cursor exhausted
    Step,       // advance one turn per user keypress
}

fn inject_replay_actions(
    log: Res<SessionLog>,
    mut replay: ResMut<ReplayState>,
    turn: Res<TurnCounter>,
    // Write into the same action resource the normal input system writes to
    mut pending: ResMut<PendingSimAction>,
) {
    if replay.paused { return; }
    
    if let Some(record) = log.actions.get(replay.cursor) {
        if record.turn == turn.current {
            pending.0 = Some(record.action.clone());
            replay.cursor += 1;
        }
    }
}
```

The `PendingSimAction` resource is the single handoff point. In `AppMode::Playing`
it is written by the input recording system. In `AppMode::Replaying` it is written
by the replay injection system. The simulation systems downstream are identical
in both modes.

---

## State Hashing and Divergence Detection

This is the feature that makes the system genuinely useful for bug hunting rather
than just an interesting novelty.

### What to hash

Hash only simulation-affecting components. Explicitly exclude all render, UI,
audio, and transform components. The hash captures "did the simulation state
change in an unexpected way", not "did pixels move".

```rust
#[derive(Component, Reflect)]
#[reflect(Hash)]
pub struct SimulationComponent; // marker — add to every sim-relevant component

fn hash_simulation_state(
    world: &World,
    mut hash_log: ResMut<StateHashLog>,
    turn: Res<TurnCounter>,
) {
    use std::hash::{Hash, Hasher};
    // rustc-hash: deterministic, fast, already in Bevy's dep tree
    let mut hasher = rustc_hash::FxHasher::default();
    
    // Collect all sim entities, sorted by SimId for stable order
    let mut entities: Vec<_> = world
        .query_filtered::<(Entity, &SimId), With<SimulationComponent>>()
        // ... iterate and collect
        .collect();
    entities.sort_by_key(|(_, id)| *id);
    
    for (entity, sim_id) in entities {
        sim_id.hash(&mut hasher);
        // Hash each registered simulation component via reflection
        // world.inspect_entity(entity) + component_id filtering
    }
    
    hash_log.hashes.push((turn.current, hasher.finish()));
}
```

This system only runs in builds with the `devtools` feature:
```rust
#[cfg(feature = "devtools")]
app.add_systems(SimulationUpdate, hash_simulation_state
    .in_set(SimulationSet::Post));
```

### Divergence detection during replay

When replaying a log that contains recorded hashes:

```rust
fn check_divergence(
    log: Res<SessionLog>,
    hash_log: Res<StateHashLog>,
    turn: Res<TurnCounter>,
    mut divergence: EventWriter<SimulationDiverged>,
) {
    let t = turn.current;
    let live = hash_log.hashes.iter().find(|(turn, _)| *turn == t);
    let recorded = log.state_hashes.iter().find(|(turn, _)| *turn == t);
    
    if let (Some((_, live_hash)), Some((_, rec_hash))) = (live, recorded) {
        if live_hash != rec_hash {
            divergence.write(SimulationDiverged { turn: t, live: *live_hash, recorded: *rec_hash });
        }
    }
}

#[derive(Event)]
pub struct SimulationDiverged {
    pub turn: u64,
    pub live: u64,
    pub recorded: u64,
}
```

When `SimulationDiverged` fires, the devtools panel highlights the exact turn,
pauses replay, and shows which entities were present in the hash but differ.
The first divergence turn is the upper bound on where the bug was introduced.

---

## Serialization

Use `postcard` for the binary log format — it is compact, fast, and already
used by `bevy_replicon` for network payloads, so it has strong Bevy ecosystem
validation.

```rust
pub fn save_log(log: &SessionLog, path: &Path) -> anyhow::Result<()> {
    let bytes = postcard::to_allocvec(log)?;
    // Optionally compress with zstd for long sessions
    let compressed = zstd::encode_all(bytes.as_slice(), 3)?;
    std::fs::write(path, compressed)?;
    Ok(())
}

pub fn load_log(path: &Path) -> anyhow::Result<SessionLog> {
    let compressed = std::fs::read(path)?;
    let bytes = zstd::decode_all(compressed.as_slice())?;
    Ok(postcard::from_bytes(&bytes)?)
}
```

A 10-hour CDDA session at ~1 action/turn and ~1 turn/second is ~36,000 action
records. Each record is approximately 16 bytes uncompressed. Uncompressed: ~576 KB.
With zstd at level 3: typically under 50 KB. The state hashes add ~16 bytes/turn,
~576 KB more, but are debug-only and stripped from release logs.

For bug reports, compress + base64 and paste into a GitHub issue body:
```rust
pub fn log_to_clipboard_string(log: &SessionLog) -> anyhow::Result<String> {
    let bytes = postcard::to_allocvec(log)?;
    let compressed = zstd::encode_all(bytes.as_slice(), 9)?; // max compression
    Ok(base64::encode(compressed))
}
```

---

## The Plugin

Everything gates behind a feature flag:

```toml
# Cargo.toml
[features]
default = []
devtools = ["bevy_mod_debugdump", "iyes_perf_ui", "bevy_inspector_egui"]
replay = []  # enables recording/replay without full devtools

[dependencies]
bevy_rand = { version = "0.12", features = ["wyrand"] }
bevy_prng = { version = "0.12", features = ["wyrand"] }
bevy_enhanced_input = "0.9"
postcard = { version = "1", features = ["alloc"] }
zstd = "0.13"
```

```rust
pub struct ReplayPlugin;

impl Plugin for ReplayPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(SessionLog::default())
            .insert_resource(TurnCounter::default())
            .add_systems(SimulationUpdate, (
                record_actions.in_set(SimulationSet::Input),
            ));
        
        #[cfg(feature = "devtools")]
        app.add_systems(SimulationUpdate,
            hash_simulation_state.in_set(SimulationSet::Post));
    }
}

pub struct ReplayModePlugin;  // added instead of ReplayPlugin during replay

impl Plugin for ReplayModePlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(ReplayState::default())
            .add_systems(SimulationUpdate, (
                inject_replay_actions.in_set(SimulationSet::Input),
                #[cfg(feature = "devtools")]
                check_divergence.in_set(SimulationSet::Post),
            ));
    }
}
```

---

## Regression Testing

The most valuable long-term use of this system is automated regression tests.
Once you have a confirmed-reproducible bug log, add it to a `tests/replays/`
directory and write a test that replays it and asserts the simulation reaches
the expected state (or no longer crashes):

```rust
#[test]
fn regression_issue_1234_npc_pathfinding_panic() {
    let log = load_log("tests/replays/issue_1234.replay").unwrap();
    let mut app = build_headless_sim_app(log.world_seed);
    app.add_plugins(ReplayModePlugin);
    
    // Run until log is exhausted
    while !app.world.resource::<ReplayState>().is_complete() {
        app.update();
    }
    
    // Assert the specific condition that was broken
    assert!(app.world.resource::<CrashLog>().is_empty());
}
```

This turns every bug report into a permanent regression test for free.
The `.replay` file is the test fixture. No manual test case construction needed.

---

## Checklist for Determinism

Use this during development to audit each new simulation system:

- [ ] Uses `bevy::platform::collections::HashMap`, not `std::collections::HashMap`
- [ ] Any query that makes RNG calls sorts by `SimId` before iterating
- [ ] Any system that writes simulation state has explicit `before`/`after` ordering
- [ ] RNG is per-entity `WyRand` component, never global
- [ ] New entity spawns use `global.fork_seed()` and assign a `SimId` deterministically
- [ ] No `Time::delta()` or frame-rate-dependent values in simulation logic
- [ ] New components that affect simulation are registered with `SimulationComponent` marker
- [ ] Ambiguity detection (`LogLevel::Error`) passes cleanly in debug builds
