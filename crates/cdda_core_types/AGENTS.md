# cdda_core_types DOX

## Purpose
Layer 1 of the workspace. Owns pure value types, coordinates, IDs, units, damage, flags, RNGs, and raw JSON definition structs. No game systems, no schedules, no resources live here — only data and pure functions over it.

## Ownership
- `Cargo.toml` deps: `serde`, `serde_json`, `schemars`, `bevy_ecs`, `bevy_reflect`, `thiserror`, `rand`. Dev-only: `cdda_components`.
- Bevy is present **only** for `#[derive(Component)]` (`Pos<_, _>`, `SimId`, `WyRand`) and `#[derive(Reflect)]` (units, `DefId<T>`). No system code, plugins, or schedule references. The `lib.rs` doc comment claiming "Zero `bevy_ecs` dependency" is stale — it predates the `Component` derives.
- Top-level modules: `core`, `rng`, `sim_id`, `wyrand`. Re-exported through `crate::core::{Damage, FlagSet, WorldPos, ...}`.
- ECS components and Bevy resource types belong in `cdda_components`; data loading and entity spawning belong in `cdda_data`.

## Local Contracts
- **Units** (all `Copy`, all accept CDDA-style strings via custom `Deserialize`):
  - `Length(pub u32)` — millimeters; `from_millimeters`/`from_centimeters`/`from_meters`; `Display` picks mm/cm/m.
  - `Volume(pub u64)` — milliliters; `from_milliliters`/`from_liters`; `Display` picks ml/L.
  - `Weight(pub u64)` — grams; `from_grams`/`from_kilograms`/`from_kilograms_u64`; `Display` picks g/kg.
  - `Energy(pub u64)` — Joules; `from_joules`; `Display` is bare. Accepts `"1 kJ"` strings.
  - `Time(pub i64)` — turns (signed, 1 h = 3600, 1 m = 60); `Display` formats as `"H h M m S s"`. Invalid strings deserialize to `Time::ZERO` (no error).
  - `Sub` is saturating for all non-`Time` units; `Time` is plain wrapping subtraction.
- **Coordinates** — generic `Pos<Scale, Origin>` with `x: i32`, `y: i32`, `z: ZLevel`. Marker enums: `Ms` (1 tile), `Sm` (12 tiles), `Omt` (24 tiles), `Om` (180 omts); `Abs`/`Bubble`/`Rel` origins. Scale constants: `TILES_PER_SM=12`, `TILES_PER_OMT=24`, `OMT_PER_OM=180`. Type aliases: `WorldPos=Pos<Ms,Abs>`, `SubmapPos=Pos<Sm,Abs>`, `SubmapLocal=Pos<Ms,Rel>`, `BubblePos=Pos<Ms,Bubble>`, `OmtPos=Pos<Omt,Abs>`, `OmPos=Pos<Om,Abs>`, `VehicleMountPos=Pos<Ms,Rel>`, `VehicleMapPos=Pos<Ms,Rel>`. `Pos<Ms,Abs>` has `to_submap`/`from_submap`/`to_omt`/`from_omt`/`to_om`; all use `div_euclid`/`rem_euclid` for negative coords.
- **ZLevel(pub i8)** — clamped to `[-10, 10]` on construction (`ZLevel::new`); `checked_add`/`checked_sub` return `Option`. Errors via `CoreError::ZLevelOverflow`.
- **IDs** — `DefId<T>` is a `String` newtype with `PhantomData<T>`, `Deref<Target=str>`, `From<String/&str/u32/i32>`, `JsonSchema`, `Reflect`, `Serialize`/`Deserialize` (custom deser that ignores the marker). `DefCategory` is a flat `Copy` enum of ~110 variants (`Item`, `Monster`, `Terrain`, `Furniture`, `Recipe`, `Bionic`, `Mutation`, `Effect`, `Skill`, `VehiclePart`, `MapgenPalette`, `OvermapTerrain`, `Spell`, `Vehicle`, `MartialArt`, `MonsterGroup`, `BodyGraph`, `Anatomy`, …) — one variant per JSON `"type"`.
- **Damage** — `Damage { entries: Vec<DamageEntry> }` where `DamageEntry { damage_type: DefId<DamageTypeDef>, amount: u32 }`. `Damage::add` saturates by `damage_type` and drops zero amounts. `Add`/`AddAssign`/`IntoIterator`/`total`/`by_type`/`iter`/`clear`.
- **Flags** — `FlagSet { flags: BTreeSet<String> }` (BTreeSet for ordered iteration).
- **RNGs** — `SeededRng` (wraps `rand::rngs::StdRng`, `Serialize`/`Deserialize` stores `seed: u64`, *not* a Component) and `WyRand` (wyhash v4.2 state, `#[derive(Component)]`, derives from `(world_seed, sim_id)`, exposes `fork`). `SimId(pub u64)` is a deterministic `Component` produced from `(world_seed, counter)` via splitmix64; `Deref<Target=u64>`. Sortable to get stable spawn order.
- **Copy-from** — `CopyFromTarget` (raw JSON struct: `copy_from`, `abstract_`, `extend`, `delete`, `relative`, `proportional`). `CopyFromOp` enum: `Set(String, Value)`, `Extend(String, Value)`, `Delete(String, Value)`, `Relative(String, Value)`, `Proportional(String, Value)`. `CopyFromChain { chain: Vec<String> }` is the resolved base-to-current ancestor list.
- **Localized text** — `LocalizedString` is `#[serde(untagged)]` `Plain(String)` or `Object { str, str_sp, str_pl, context }`. `singular()`/`plural()` accessors; full i18n extraction is deferred.
- **Errors** — `CoreError { ZLevelOverflow(i8), InvalidValue(String) }` (thiserror).

## Work Guidance
- Add new units by mirroring the `Length`/`Volume`/`Weight`/`Energy`/`Time` pattern: `pub struct Foo(pub INTEGER_KIND)`, `from_X` constructors, custom `Deserialize` visitor for both bare numbers and CDDA unit strings, `Add`/`Sub`/`Ord`/`Display`.
- Add a new coordinate family only by picking a new `Scale` × `Origin` pair; never overload an existing alias. Cross-scale math must use `div_euclid`/`rem_euclid`.
- Add a new def category by: (1) adding a variant to `DefCategory`, (2) creating a new `XxxDef` struct in `core/raw_defs/` with `id: DefId<XxxDef>`, (3) adding a re-export to `core/raw_defs/mod.rs`. Use `LocalizedString` for user-facing text and `DefId<...>` for cross-references.
- Internal unit tests live as `#[cfg(test)] mod tests` inside each source file; cross-crate tests live under `tests/`. The `tests/stats_test.rs` file imports from `cdda_components` (not this crate) — it is intentionally a smoke test, not a core_types test.
- Do not introduce Bevy `Resource`, `Bundle`, or system code here. If you find yourself wanting a `Component` on something with lifecycle, move it to `cdda_components`.

## Verification
- `cargo check -p cdda_core_types`
- `cargo nextest run -p cdda_core_types` (or `cargo test -p cdda_core_types` if nextest is unavailable).
- `cargo test -p cdda_components` after touching `DefId<T>` or `SimId` (they are consumed by components and stats tests).

## Child DOX Index

- `src/core/coords/AGENTS.md` — coordinate system: `Pos<Scale, Origin>`, scales (`Ms`/`Sm`/`Omt`/`Om`), origins (`Abs`/`Bubble`/`Rel`), `ZLevel`, `Direction`/`Facing`, type aliases.
- `src/core/units/AGENTS.md` — type-safe unit newtypes: `Length`, `Volume`, `Weight`, `Energy`, `Time` (with custom `Deserialize` for CDDA unit strings).
- `src/core/raw_types/AGENTS.md` — cross-def helpers: `LocalizedString`, `CopyFromTarget`/`CopyFromOp`/`CopyFromChain`.
- `src/core/raw_defs/AGENTS.md` — ~130 typed JSON definition structs grouped by domain:
  - **Items & inventory:** `item`, `item_action`, `item_category`, `item_group`, `item_migration`, `ammunition_type`, `ammo_effect`, `uncraft`, `pocket`/`item_variant` (in `item.rs` and `cdda_types.rs`).
  - **Monsters & combat:** `monster`, `monster_attack`, `monster_flag`, `monster_faction`, `monstergroup`, `monster_blacklist`, `species`, `bash_damage_profile`, `attack_vector`, `hit_range`, `weakpoint_set`.
  - **Terrain, traps, and mapgen:** `terrain`, `furniture`, `trap`, `trap_migration`, `ter_furn_migration`, `ter_furn_transform`, `field`, `mapgen`, `map_extra`, `map_extra_collection`, `forest_biome_component`, `forest_biome_mapgen`, `region_settings`, `region_settings_terrain_furniture`, `region_terrain_furniture`, `gate`, `rotatable_symbol`, `connect_group`, `climbing_aid`, `omt_placeholder`, `overlay_order`.
  - **Overmap & weather:** `overmap_terrain` (also `OvermapConnectionDef`, `OvermapLandUseCodeDef`, `OvermapLocationDef`, `OvermapSpecialDef`), `oter_id_migration`, `oter_vision`, `weather_type`, `weather_generator`, `pp_generator`, `city_building`, `vehicle_placement`, `vehicle_spawn`.
  - **Bodies & biology:** `body_part`, `sub_body_part`, `anatomy`, `body_graph`, `limb_score`, `bionic`, `mutation`, `mutation_type`, `mutation_category` (in `mutation.rs`), `harvest`, `harvest_drop_type`, `vitamin`, `addiction_type`, `disease_type`.
  - **Recipes, crafting, requirements:** `recipe`, `recipe_category`, `recipe_group`, `requirement`, `tool_quality`, `practice`, `proficiency`, `proficiency_category`, `proficiency_migration`, `butchery_requirement`, `profession_item_substitutions`.
  - **Vehicles:** `vehicle_def`, `vehicle_part`, `vehicle_part_migration`.
  - **NPCs, factions, scenarios:** `npc`, `npc_class`, `faction`, `faction_mission`, `profession`, `profession_group`, `scenario`, `scenario_blacklist`, `start_location`, `talk_topic`, `mission_definition`, `conduct`, `dream`, `achievement`, `end_screen`, `score`.
  - **Skills, effects, combat verbs:** `skill`, `skill_display_type`, `effect`, `effect_migration`, `effect_on_condition`, `morale_type`, `movement_mode`, `mood_face`, `scent_type`, `speed_description`, `speech`, `snippet`, `spell`, `technique`, `martial_art`, `activity_type`, `emit`, `event_statistic`, `event_transformation`.
  - **Materials, damage, flags, and shared shape:** `material`, `damage_type`, `damage_info_order`, `json_flag`, `jmath_function`, `cdda_types` (shared `CddaColor`, `ArmorValues`, `BodyPartArmor`, `MeleeDamage`, `UseAction`, `RawValue`, `CountRange`, `StringOrArray`, etc.).
  - **Migrations, blacklists, misc:** `item_migration`, `trait_migration`, `var_migration`, `camp_migration`, `shopkeeper_blacklist`, `shopkeeper_consumption_rates`, `charge_removal_blacklist`, `temperature_removal_blacklist`, `relic_procgen_data`, `widget`, `weapon_category`, `item_category`, `nested_category`, `character_mod`, `clothing_mod`, `fault`, `fault_fix`, `fault_group`, `ascii_art`, `damage_info_order`.

Root-level files: `lib.rs`, `rng.rs`, `sim_id.rs`, `wyrand.rs`.
