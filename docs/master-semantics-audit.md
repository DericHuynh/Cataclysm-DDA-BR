# Player-visible semantics audit against master

Recorded 2026-09-05 against the local `../Cataclysm-DDA-master` snapshot and the
current BR working tree. **Continuation gate: not met.** The user requested the
same player-visible outcomes, not matching class/component organization. Feature
expansion must not be described as semantics-preserving on the current evidence.

This is a source comparison plus focused BR headless checks. Master was not built
or executed in this audit; there is no cross-engine differential test result.
The current 1,309 passing BR tests establish regression coverage, not master
parity. The first timing reconciliation replaces the divergent expectations
described below.

## Resolved neutral timing scenarios

The first reconciliation addresses three source-derived cases:

| Scenario | Master expectation | BR implementation and evidence |
|---|---|---|
| Craft finishing budget | `craft_activity_actor::do_turn` spends all available moves before clamping progress: 250 available / 200 remaining leaves 0. [activity_actor.cpp](../../Cataclysm-DDA-master/src/activity_actor.cpp), lines 6308–6325. | `tick_crafting` consumes the whole budget, including finishing overshoot. `master_craft_ticks_consume_all_moves_including_the_finishing_tick` covers speeds 50, 100, 250 and 1000 through imported recipe intents. |
| Partial TIME rounding | `player_activity` truncates `moves * remaining / 100`: 25 available / 50 remaining costs 12, leaving 13. [player_activity.cpp](../../Cataclysm-DDA-master/src/player_activity.cpp), lines 244–250. | `spend_time` truncates; scheduler recognizes completion even at zero AP cost. `master_partial_time_completion_truncates_cost_and_allows_the_next_action` covers neutral budgets/remaining work (25,50), (1,1), (101,99), with and without a queued action. |
| Repeated player input | The avatar input loop continues while moves remain, before creature processing and the next world turn. [do_turn.cpp](../../Cataclysm-DDA-master/src/do_turn.cpp), lines 555–615. | TurnBased PlayerData/DevPlayer actors spend remaining moves across frames without another AP grant, GameTime advance or effect tick. Other actors wait until player moves run out. `master_player_commands_share_a_turn_and_hold_ai_until_budget_is_spent` covers two 100-move actions from a 200-move grant, rejection/retry, both player identities, idle frames and AI deferral. |

`SimulationIngress` separates pending menu/item requests from one-second world
phases. `SimulationRefresh` updates inventory and spatial state after commits,
including commands using a turn's remainder. Additional headless cases cover
pause/debt, same-turn menu crafting/result letters, and spatial refresh without
idle NPC activity causing repeated refreshes.

These are limited source-derived scenarios, not broad timing equivalence. Craft
speed/exertion modifiers are absent; detailed physiology, creature and world-phase
ordering remains unverified. Manual/RealTime and explicit step requests deliberately
force world turns and do not wait for avatar input. The tests do not execute master.

## Confirmed player-visible differences

| Area | Master behavior and source | Current BR behavior and consequence |
|---|---|---|
| Nearby crafting ingredients | `crafting_inventory()` uses `PICKUP_RANGE` and map inventory/path rules. See [crafting.cpp](../../Cataclysm-DDA-master/src/crafting.cpp), lines 648–677, and [game_constants.h](../../Cataclysm-DDA-master/src/game_constants.h), line 44 (`PICKUP_RANGE = 6`). | [collect_available_items](../crates/cdda_sim/src/crafting/systems.rs) includes unowned ground items in the same 24×24 OMT, without map path checks. In an unobstructed map, an ingredient at (20,0) can be used from (0,0) in BR; one at (24,0) is excluded from (23,0). Master's six-tile search has the opposite reach result for those examples. |
| Craft eligibility, speed and results | Master checks required proficiencies, tool/component requirements and continuation/recipe knowledge; speed incorporates lighting, morale, pain, manipulation, assistants and workbench/tool context. Completion creates recipe results and byproducts. See [crafting.cpp](../../Cataclysm-DDA-master/src/crafting.cpp), `can_start_craft`, `crafting_speed_multiplier` and result creation; [activity_actor.cpp](../../Cataclysm-DDA-master/src/activity_actor.cpp), lines 6274–6395. | [check_can_craft/prepare_craft/complete_craft](../crates/cdda_sim/src/crafting/systems.rs) checks counted components and tool quality, uses fixed recipe seconds ×100 work, and spawns one prepared result definition/count. Those other conditions and effects are absent. The strict inventory importer rejects unsupported fields, but broad legacy app loading is a separate path; loading a recipe is not proof that its behavior is supported. |
| Resuming a craft | The item-use route checks mounted restrictions, wielding, continuation requirements and recipe knowledge. See [iuse.cpp](../../Cataclysm-DDA-master/src/iuse.cpp), lines 8802–8835. | [resume_craft_with_outcome](../crates/cdda_sim/src/crafting/systems.rs) checks a living actor, AP, ownership, sealed access and current activity. It does not perform those additional checks. Retaining progress is aligned; the conditions for resuming are not equivalent. |
| Item handling cost and pocket access | Pickup uses location acquisition/handling costs, quantity, distance and bulk handling; it can unseal a containing pocket. See [pickup.cpp](../../Cataclysm-DDA-master/src/pickup.cpp), lines 349–367. | [apply_inventory_action](../crates/cdda_sim/src/inventory/transfer.rs) charges fixed 100-AP costs for supported item actions and rejects sealed ancestors. Moving the same item can take different time or be refused. |
| Pocket/content capabilities | Master's [_can_contain](../../Cataclysm-DDA-master/src/item_pocket.cpp) supports liquids, charges, specialized types, restrictions and partial fitting. | [capacity.rs](../crates/cdda_sim/src/inventory/capacity.rs) supports whole counted-solid transfers into ordinary unrestricted pockets and rejects other semantics. Valid master inventory operations are unavailable in BR. Direct spawning and craft-result placement also bypass the transfer capacity policy. |
| Combat, reading, reload and aiming outcomes | Master performs attack/damage/death resolution and activity-specific effects. For example, [reload_activity_actor::finish](../../Cataclysm-DDA-master/src/activity_actor.cpp), line 7951, actually reloads; reading completion processes the book at line 2492. | [combat/systems.rs](../crates/cdda_sim/src/combat/systems.rs) still contains execution placeholders. The [intent resolver](../crates/cdda_sim/src/intent/systems.rs) rejects unsupported verbs as Failed. [Activity systems](../crates/cdda_sim/src/activity/systems.rs) advance timers but do not implement equivalent ammunition/book effects; aiming uses a native 20-AP-per-percent approximation. Component decomposition is not combat parity. |

The resolved craft-completion difference corrected the earlier source-backed
reasoning: generic `player_activity` SPEED handling preserves a finishing
remainder, but the **craft-specific actor overrides that behavior**. Using the
generic path as the specification for crafting was insufficient.

## Aligned principles with limited evidence

- Attack skill/dice, dodge and natural protection can be independent components
  without changing their values. `CombatStats::into_bundle` preserves its input
  values. The monster dodge projection now reads the resolved `dodge` field
  independently of melee dice, consistent with [monstergenerator.cpp](../../Cataclysm-DDA-master/src/monstergenerator.cpp), lines 907–915. This says nothing about unimplemented combat resolution.
- Occupied volume/weight must count toward container limits. Master's
  [item_size_modifier/item_weight_modifier](../../Cataclysm-DDA-master/src/item_pocket.cpp), lines 630–655,
  and BR's aggregate load checks share the basic rigid-volume/mass principle.
  Per-pocket multipliers, mixed pocket types and JSON capability support still
  prevent a broad containment-parity claim.
- Craft progress belongs to a retained in-progress object and can survive
  interruption. BR tests verify that lifecycle; master-specific resume checks
  and tool/resource use remain missing.
- Neutral full-second TIME work can advance one second while consuming the
  actor's available budget. This narrow case and neutral partial-time rounding match; exertion and
  activity-specific effects remain separate compatibility work.
- Fixed headers, retained list rows and independent selection reveal preserve
  the intended BR UI interaction. Master-equivalent item grouping, recipe
  membership, filtering and detail values have not been established by layout
  tests. Cosmetic differences were requested; gameplay/data differences were not.

## Verification and next gate

The initial audit ran eight existing BR checks, which confirmed the divergent
BR rules rather than master parity. For reconciliation, four master-derived timing
checks were observed failing before the runtime changes and passing afterwards.
Additional menu and spatial-refresh cases exercise the new command/refresh phases.
The full headless workspace run passed 1,309 tests with 100 pre-existing skips;
workspace/all-target compilation and runtime dependency checks passed. See the
[compatibility baseline](ecs-compatibility-baseline.md) for commands and scope.

Before resuming feature expansion:

1. Completed for the neutral scenarios above: master-derived player input/world-time
   boundaries, craft-specific finishing costs and partial TIME rounding. Broader
   modifiers and world-phase ordering remain outside that evidence.
2. Reconcile crafting reach/access, then eligibility, continuation and output
   semantics for the explicitly supported recipe family.
3. Reconcile handling costs and supported pocket behavior. Keep unavailable
   capabilities explicit at the playable-content boundary.
4. Compare inventory/recipe rows and detail values from the same fixtures.
5. Gate each subsequent combat/activity implementation on master-specific
   effects and outcomes, including rejection and interruption. Architecture
   and internal regression tests alone cannot satisfy this gate.

Feature expansion remains gated. The timing reconciliation fixes existing behavior;
next work is crafting reach/access and the supported recipe family’s checks/results.

## Reference fingerprint

Bare SHA-256 hashes of the local files inspected (distinct from the manifest
hashes in [the compatibility baseline](ecs-compatibility-baseline.md)):

| File under master `src/` | SHA-256 |
|---|---|
| activity_actor.cpp | d22be91cd3c9fc986321d26e1519c194bc76138a5b2e34e7320cbc18ce19cb0d |
| player_activity.cpp | e91b5641d2dd5b239e5335cc8e86ad7b00acf32687ebcf7b2c14af64bcfe0394 |
| do_turn.cpp | a1a370a730c6d5d13b10c24bda17b8e0645732084b386fc6f226d5338bcec160 |
| crafting.cpp | 445fa66e727ccc23205ce543e838e18a848926ddec17dcdb2d07117550da1eeb |
| pickup.cpp | 639ec5aaaf5ab888ce61a1a7b2e78aa4c04df6aa06c18ea9ae8d90bb39788b06 |
| item_pocket.cpp | f0c4c1019fa8b4312976e470796e5a488adacb095ac4e2940290d1f5488ca2b2 |
