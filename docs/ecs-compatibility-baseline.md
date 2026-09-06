# ECS and JSON compatibility baseline

Recorded 2026-09-05. Rust base commit: `cd8dd24d686733ef0a893e742765c9a7bfaa3c9d` plus the working-tree UI changes. The sibling reference is a source snapshot without Git metadata; content hashes pin it. The catalog and retained UI batches follow [the source-backed plan](ecs-modularity-and-json-compatibility-plan.md).

## Content manifest

No mods are selected for these corpus reports. Ordered input is core only. The native acceptance fixture uses `core`; its override test applies `core` then `mod` explicitly.

| Input | JSON files | Bytes | SHA-256 manifest |
|---|---:|---:|---|
| `../Cataclysm-DDA-master/data/json` | 3,204 | 44,347,600 | `05c072e25e26be0bd93fdf2320f113063391968cefca3817f4e9f18d3b1ed3c7` |
| `data/core` | 3,203 | 44,349,273 | `bab03d3e4cfd5002dbe5f58e0f32b75c81d0351a6971d757d7abe918b2fafdb4` |
| `crates/cdda_integration_tests/tests/fixtures/inventory_native.json` | 1 | 761 | `a9e36dc481dc7243d1e5c585c3a1fbb1cd42d07a06bdb2f7524977d260878c11` |

Key source-file manifests:

| Reference source | SHA-256 manifest |
|---|---|
| `src/generic_factory.h` | `69b94160c95050ab46e19128c4cc10906d128853ad17f035e2d7cd50e28e27c8` |
| `src/itype.h` | `f3b4e6ed3d4546c5b616d4f817d7467803d38298083776a801d1bb2fccccaf46` |
| `src/activity_actor.h` | `d1b07283110c582cf91dbe7676f428d02527589f0d9894af9e602203f8695f13` |

Reproduce with `python3 scripts/content_baseline.py <paths ...>`. These are manifests, not bare file hashes: sorted relative UTF-8 path + NUL + SHA-256(file bytes), accumulated into SHA-256. Directory inputs include JSON only. Original document values are retained semantically; byte-identical export is not claimed.

## Player-visible compatibility status

The [master semantics audit](master-semantics-audit.md) fails the continuation gate.
Neutral craft finishing AP, partial TIME rounding and player turn boundaries now
have master-derived passing scenarios; crafting reach/checks/modifiers and
handling costs still differ. The capabilities and passing tests below describe BR;
they do not certify that a player gets the same result as master.

## Implemented capability boundary

| Capability | Native inventory adapter | Remaining scope |
|---|---|---|
| Counted generic items | Stable keys, names, descriptions, categories, checked volume/weight, tool qualities | Specialized item families and behaviors remain explicitly unsupported |
| Container pockets | Independent mounted pockets, checked projected volume/weight, counts, nested/rigid load and linked lifetime | Specialized pocket restrictions, fluids/charge dimensions, partial stacks and direct creation placement |
| Recipes | Composite keys, counted alternatives, quality requirements, checked work/result counts | Charges, requirement-list expansion, skill gating, byproducts, advanced flags/actions |
| Source semantics | Shared loader, ordered overrides including alias changes, inheritance, units, original/resolved records and located diagnostics | Type-specific upstream reader parity; generic patching alone is not proof of compatibility |
| Publication | Validate before native-world mutation; retain item/output snapshots across reload | Broad app loader, actor/body/AI reference migration, full native saves |
| Craft transactions | Validate output before consumption, reserve counts across all slots, backtrack alternatives, populated result spawning | Eligibility, reach, speed/exertion modifiers, tool/resource use and output placement |

`inventory_compat` reports diagnostic counts, not distinct unsupported definitions. `strict_catalog()` rejects both preserved-but-unimplemented and rejected content. Inspection keeps original and resolved values. The six-definition fixture is accepted; the entire core corpus is intentionally rejected by this narrow family. Broad legacy loading remains the app's content path and is not equivalent to strict playable-content validation.

Final headless reports for this pinned corpus:

| Input | Documents | Resolved definitions | Supported diagnostics | Preserved/unimplemented diagnostics | Rejected diagnostics | Strict playable |
|---|---:|---:|---:|---:|---:|---|
| Native fixture | 1 | 6 | 6 | 0 | 0 | Yes |
| Rust `data/core` | 3,203 | 52,952 | 0 | 209,684 | 5,781 | No |
| Original `data/json` | 3,204 | 52,955 | 0 | 209,689 | 5,781 | No |

A valid upstream family without an inventory-style identity (for example dreams) is preserved as unimplemented, not mislabeled malformed. Unsupported top-level fields and nested forms remain diagnostic findings even when the adapter can normalize a subset of that record.

Run `cargo run -p cdda_data --example inventory_compat -- <core path> [mod path ...]` for the live capability report. The driver prints aggregate counts and the first 20 diagnostics; the API retains all diagnostics. Inputs are sorted within each directory and explicitly ordered across mods. Unsupported semantics must never be described as implemented merely because a raw struct accepts them.

## Ownership and cached-reference audit

| Data | Owner / policy |
|---|---|
| Domain relationships, mutable item/actor state, semantic intents/results, schedules | `cdda_components`; authoritative mutations in focused `cdda_sim` subsystems |
| Input vocabulary / bindings | `cdda_input::vocabulary`; no input framework in shared gameplay components |
| Screen contexts / navigation | `cdda_context::state` |
| Recipe filter/focus/categories and inventory focus | `cdda_render`; model extraction is separate from widgets |
| Visible range, retained keyed text rows, pane geometry, focus reveal, wheel routing | `cdda_ui`; presenters translate game actions into widget state |
| Typed definition entity index, session tokens, native HTN input | `cdda_catalog`; data crate re-exports preserve existing adapter imports |
| Item/recipe stable keys and immutable normalized data | `cdda_catalog::inventory`; source-format translation stays in `cdda_data` |
| `DefOrigin` | Session-local token, not a persistent entity/index identity; native spawning aligns it with ItemTypeRegistry |
| Runtime native item definition | Arc snapshot retained across publication; stable ItemKey for serialized catalog identity |
| In-progress craft | PreparedItem output snapshot survives definition replacement; legacy recipe_entity remains diagnostic state and is not dereferenced to finish |
| RecipeIndex / menu recipe entities | Rebuilt on publication; presenters preserve selection using cached composite keys |
| DevSpawnCatalog entry def_entity | Re-extracted on entry and component additions/changes/removals while open; stable item ID preserves selection across replacement |
| RegistryCatalog inspection snapshot | Re-extracted on entry and definition/raw/index/token resource revisions (including removal); stable category name/entry ID preserve selection; direct definition component edits require publication invalidation |
| HTN ItemCatalog/HtnRuntime | Immutable domain/observation facts; integration still needs publication-time plan invalidation and generation tagging |
| Other actor/body/definition consumers | Require per-consumer auditing during broad publication adoption; no blanket safety claim |

`python3 scripts/check_runtime_dependencies.py` verifies normal transitive dependencies: simulation/catalog exclude raw schemas, data loaders, input frameworks and asset/render loading; cdda_ui excludes all gameplay crates. Dev fixture dependencies remain permitted. ItemDefinitionRef snapshots and catalog serialization do not constitute full actor/inventory save/load.

## Verification and remaining batches

Headless acceptance lives in `native_inventory_test`: normalization, alias/mod order, source diagnostics, rollback, production-schedule crafting across reload, competing ingredient consumption, alternatives, pocket lifetime and stable catalog serialization. Existing data bridge/round-trip and crafting-time suites remain active. Menu tests cover thousands of rows, fixed headers, manual scroll independence, bounded node counts, selection retention after model changes and idle writes.

Crafting, character, Settings, registry and spawn now use keyed retained rows with cell-level comparison. Headless tests check overlapping-key retention, recycling on distant jumps, unchanged text ticks on focus changes, shrink/regrow cleanup and pane lifetime. RecipeFilter membership survives 3,000 selection moves without rebuilding; model/filter/category changes invalidate it. Spawn catalog tests cover definition replacement and component removal. Registry/spawn fixtures cover 40,000 entries, source/selection separation, cached filter membership, same-frame reveal, fixed native-layout headings, empty-category details and resource removal. `cargo nextest run -p cdda_render -p cdda_ui --offline` passes all 35 tests (including nested sealing/unmounting invalidation, independent combat fields and examine command submission).

Counted-solid transfers now share a synchronous native/legacy boundary; letter assignment no longer mutates stacks, and explicit merges require colocation and compatible snapshots. Production-schedule tests cover competing transfers, stale legacy requests and AP conservation. The native fixture verifies imported pocket limits, nested crafting and reclaimed space. Direct creation placement and specialized pocket/charge/fluid semantics remain separate work.

Shared action/activity arbitration now consumes all craft-tick AP and truncates partial TIME cost, matching the audited neutral master scenarios. TurnBased player commands reuse remaining moves without world/effect ticks; other actors wait until player moves are exhausted. Ingress and post-commit inventory/spatial refresh are independent of world-time phases. Native craft intents and the legacy menu mailbox share selection and validation. Headless fixtures cover fast/slow/debt budgets, no double spend on completion, stable contested starts, interruption/owned resume, suspended/pause behavior, malformed activity combinations and 100-actor scheduling. Craft completion notifications use a persistent cursor.

Runtime combat data now separates MeleeCapability, DodgeDefense and IntrinsicArmor. The legacy CombatStats record converts explicitly and is not an ECS component. The character presenter tests independent changes/removals. Monster JSON projection uses the source dodge value/default independently of damage dice. Item-examine input now submits native commands through a persistent reader; interruption/resume use correlated intents before activity work.

Native combat execution/derived modifiers, remaining activity-start commands, creation/equipment mutation consolidation, broad content migration and local-submap persistence remain later batches in the plan. No wall-clock UI target is invented: existing tests assert bounded work/entity counts; hardware-dependent timing budgets require dedicated profiling.

Timing reconciliation verification: `cargo nextest run --workspace --exclude cdda_app --offline --no-fail-fast` passed 1,309 tests with 100 pre-existing skips. Four master-derived timing checks failed before the runtime fixes and passed afterwards; menu ingress and spatial refresh have additional production-schedule coverage. `cargo check --workspace --all-targets --offline`, the normal-dependency boundary script, formatting checks and `git diff --check` passed. Tests use no window/GPU; graphical appearance was not visually inspected in this batch.
