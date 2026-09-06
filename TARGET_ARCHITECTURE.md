# Target Architecture — CDDA-BR

The rewrite preserves CDDA's semantic responsibilities, not its global objects or giant functions. CURRENT_ARCHITECTURE.md records implemented behavior; this roadmap orders the remaining foundation work.

## Current acceptance gate

Before further gameplay expansion, reconcile the confirmed player-visible
mismatches in [the master semantics audit](docs/master-semantics-audit.md).
Completed architecture checkboxes below do not assert master behavior parity.

## Foundation checklist

- [x] Canonical headless SimulationPlugin/SimulationTurn reused by the app.
- [x] One-second game-time units; turn-based/manual/optional realtime drivers; central pause gate; no per-render-frame effect decay.
- [x] Sequential live intent commits and correlated outcomes; contested-pickup/position validation regressions.
- [x] Stable-ID terrain save palettes and append-only, fallible terrain registry reload.
- [x] Shared AP-budget selection for AI/planner actions and activities; bounded dispatch, player input continuation within a world turn, craft-specific finishing costs and truncated partial TIME spending.
- [x] Command ingress separated from world ticks; inventory/spatial refresh after commits, including commands using spare player moves.
- [x] Item-action transactional boundary (`inventory::transfer`) used by UI, AI and the resolver for Pickup/Wield/Drop/Stow/Transfer.
- [x] Independent runtime melee/dodge/natural protection; explicit legacy construction conversion; native interruption/resume command routing.
- [ ] Combat verbs (MeleeAttack/UseItem/Reload), remaining activity-start commands and equipment/effect-derived combat modifiers.
- [x] Native StartCraft arbitration and legacy pending-menu translation; validated craft interruption/resume and completion.
- [x] Shared whole-stack transfer boundary for native intents/legacy messages; checked nested pocket capacity; explicit colocated merges and independent letter assignment.
- [ ] Specialized pocket/fluid/charge semantics, partial stacks and capacity-aware direct spawn/craft-output placement.
- [x] Category-qualified keys, stable recipe identity, load-free catalog and adapter-owned input/presentation vocabulary.
- [x] Native inventory import/publication with retained item/craft snapshots.
- [ ] Migrate broad legacy publication, actor/AI references and full native game persistence.
- [x] Generic cdda_ui extraction; fixed headers and bounded virtual lists.
- [x] Retained keyed rows in crafting, character and Settings; crafting membership and Settings labels cached independently of selection.
- [x] Registry/spawn retained rows; separate catalog/interaction resources, typed registry input/presentation, independent detail invalidation and fixed headings.
- [ ] Persistent local submaps, active-region membership, dynamic entity activation/deactivation, dirty saves and elapsed-time catch-up.
- [x] Canonical full-state digest (positions/AP/health/stacks/ownership by stable SimId, spawn-order invariant) hashed after commit, with an immutable expected log during replay.
- [ ] Semantic replay command ingress with sequence numbers, RNG/definition-version in the digest, and uniform replay-speed handling of missed turns.
- [x] Restore Cargo discovery of migrated actor/combat/inventory tests and add production-schedule scenarios.

## Simulation boundary

The renderer never defines game time. GameSet orders outer adapters; SimulationTurn and explicit action/turn phases own simulation. Real-time pacing is optional and must not change action semantics. Activities consume the same actor budget as other actions; fast actors must execute more work, not merely accumulate AP.

An action operation validates actual state, commits state plus costs, and publishes its result. Deferred commands inside a loop do not provide intermediate visibility. A bounded exclusive commit is acceptable; reservations/commit buffers are alternatives only with equivalent invariant tests.

Use events for notifications and bounded reactions. Do not replace the scheduler with an unconstrained observer cascade. Choose components, tags and child entities for actual query/lifecycle needs rather than blanket decomposition rules.

## Identity and data boundary

Keep three concepts distinct:
1. typed stable definition key (category + ID);
2. persistent simulation-object identity;
3. generation-local handles / Bevy Entity values.

Runtime tables can use compact indices; saves use stable IDs or versioned palettes. Definition reload stages a validated replacement, then publishes atomically with explicit migrate/retain/cancel policies. Preserve or reject references in activities, items, terrain and plans rather than letting dangling Entity IDs silently change behavior.

Keep a validated catalog interface below simulation. Parsing/filesystem/asset loading and ECS publication are adapters, not mandatory runtime dependencies. Per-category builders already exist; further file splitting and registry ergonomics are lower priority than correctness of identity/publication.

## World lifecycle boundary

Overmap describes strategic terrain. Local submaps own playable terrain/furniture/fields/traps and persistent local state. Active-region membership determines detailed simulation, not render visibility. Leaving a region saves authoritative state; reentry accounts for elapsed game time. Spatial, navigation and visibility caches are derived and invalidated by committed mutations.

Chunked SoA arrays are appropriate for dense tiles (Master also uses this); dynamic entities remain ECS objects. Establish save/activation semantics before proliferating pathfinding, vehicles, fire and rot systems.

## Verification boundary

Use the production persistent simulation schedule in headless App tests. Test idle/pause and frame partition, multi-action budgets, contested mutations, reload during craft/plan, save/load across reordered definitions, region leave/reenter, and replay equality under different presentation pacing.

Keep isolated unit tests for pure logic, but do not mistake recreated-system TestBed calls for schedule coverage. CI must verify test target discovery so moved files do not silently disappear.

## Dependency direction

Domain values/raw schemas → shared contracts and validated catalog → simulation/world → input/render/app adapters. The planner core cdda_htn depends on no cdda crate. No crate depends on cdda_app/cdda_cli. Simulation excludes loader/raw/input/asset dependencies through cdda_catalog. cdda_components excludes input vocabulary; cdda_ui has no gameplay dependencies. scripts/check_runtime_dependencies.py guards these boundaries.
