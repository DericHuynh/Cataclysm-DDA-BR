---
name: Architecture decisions
description: Key architectural patterns and decisions for the Bevy ECS game
type: project
---

## ECS relationship pattern (Bevy 0.18)

For one-to-many data (skills, mutations, bionics, morale bonuses, status effects, body parts, inventory):

```rust
#[derive(Component, Reflect)]
#[component(immutable)]
#[relationship(relationship_target = CreatureSkills)]
pub struct SkillOf(pub Entity);

#[derive(Component, Reflect)]
#[relationship_target(relationship = SkillOf, linked_spawn)]
pub struct CreatureSkills(Vec<Entity>);

#[derive(Component, Debug, Clone, Reflect)]
pub struct SkillEntry { pub skill_id: SkillId, pub level: u32, pub experience: u32 }
```

Spawn: `commands.spawn((SkillOf(creature), SkillEntry { ... }))`.
Query: `Query<(&SkillOf, &SkillEntry)>` filtered by `sk_of.0 == creature`.

**Why:** Replaces Vec<T> inside components; each sub-entity has independent lifecycle, can be queried directly, follows Bevy 0.18's idiomatic relationship approach.

## Per-crate plugin ownership

Each domain crate owns its `register_type` calls via a plugin:
- `cdda_actor::plugin::ActorPlugin`
- `cdda_item::plugin::ItemPlugin`
- `cdda_assets::CddaAssetsPlugin`

`CddaPlugin::build` in `cdda_app` calls `app.add_plugins((ActorPlugin, ItemPlugin, CddaAssetsPlugin))`.
`register_reflect_types` in `cdda_app` only registers sim-owned spatial types.

## cdda_assets — Bevy asset integration

`CddaDataPack` is a `bevy_asset::Asset` wrapping `Arc<DefRegistry>`.
`CddaDataPackLoader` implements `AssetLoader` for `.pack` JSON manifests listing data directories.
`assets/core.pack` = `{"data_dirs": ["data/core"]}`.

Future: react to `AssetEvent::<CddaDataPack>::Modified` for hot-reload; add labeled sub-assets for per-def handles.

## cdda_ui dependency boundary

`cdda_sim` must NOT import `cdda_ui`. The `AppState → Screen` transition is handled in `cdda_app` via `OnEnter(AppState::InGame)` systems. Any `cdda_sim` code that wants to change UI state should only set `AppState`; the `OnEnter` systems in `cdda_app` handle the rest.
