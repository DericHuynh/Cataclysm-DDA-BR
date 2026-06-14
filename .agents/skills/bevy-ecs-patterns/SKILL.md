---
name: bevy-ecs-patterns
description: Bevy ECS architectural patterns including entity relationships, immutable components, conflict resolution, run conditions, and performance optimization.
disable-model-invocation: false
---

### Overview
Advanced techniques for designing clean, high-performance Bevy game code. This guide covers Bevy's bidirectional Relationships, immutable components, ParamSet for conflict resolution, custom run conditions, and optimizations for caching and batch spawning.

### When to Use This Skill
* Use when establishing parent-child or custom connections between multiple entities.
* Use when managing structural rules (such as marking components immutable) to prevent accidental mutation of key data.
* Use when designing complex system graphs with dependencies, run conditions, and custom system sets.
* Use when profiling and optimizing the performance of critical game loops to reduce cache misses and archetype fragmentation.

---

### Step-by-Step Guide

#### 1. Defining Bidirectional Relationships
Bevy v0.16+ supports bidirectional, non-fragmenting entity relationships. One component is marked as the "source of truth" while the other is automatically updated.

```rust
use bevy::prelude::*;

/// The source-of-truth relationship component.
#[derive(Component)]
#[relationship(relationship_target = Inventory)]
struct ItemInInventory(pub Entity);

/// The target component that automatically keeps track of items.
#[derive(Component, Deref)]
#[relationship_target(relationship = ItemInInventory)]
struct Inventory(Vec<Entity>);
```

#### 2. Spawning and Managing Relationships
Relationships can be set up dynamically during spawning or component insertion.

```rust
fn spawn_player_with_items(mut commands: Commands) {
    let player = commands.spawn_empty().id();
    
    // Spawning a child item and associating it to the parent
    commands.spawn(ItemInInventory(player));
}
```

#### 3. Defining Immutable Components
Immutable components enforce safety for variables that should not change after instantiation (like static definitions, relationship links, or identifiers).

```rust
#[derive(Component)]
#[component(mutable = false)] // Enforces read-only querying
struct StaticId(u32);
```

#### 4. Managing Conflicting Systems with ParamSet
Use `ParamSet` when a single system needs to access queries that would otherwise conflict with Rust's borrow checker rules.

```rust
fn health_balance_system(
    mut param_set: ParamSet<(
        Query<&mut Health, With<Player>>,
        Query<&Health, With<Enemy>>,
    )>,
) {
    // Read enemy health safely
    let mut total_enemy_health = 0;
    for enemy_health in param_set.p1().iter() {
        total_enemy_health += enemy_health.current;
    }

    // Mutate player health
    if total_enemy_health > 500 {
        for mut player_health in param_set.p0().iter_mut() {
            player_health.current += 10;
        }
    }
}
```

---

### Examples

#### Example 1: Built-in Hierarchies via ChildOf & Children
Bevy's parent-child relationship is built on top of relationships, replacing the older `Parent` and `Children` types with `ChildOf` and `Children`.

```rust
fn setup_scene(mut commands: Commands) {
    let parent_entity = commands.spawn(Name::new("Spaceship")).id();

    // Spawning a child in Bevy 0.16+
    commands.spawn((
        Name::new("Engine Thruster"),
        ChildOf(parent_entity), // Inserts relationship
    ));
}
```

#### Example 2: Entity Cloning
Bevy 0.16+ introduces native entity cloning via `EntityCommands`. Any component derived with `Clone` can be duplicated.

```rust
#[derive(Component, Clone)]
struct WeaponType {
    damage: u32,
}

fn clone_soldier(mut commands: Commands, q_source: Query<Entity, With<Player>>) {
    if let Ok(source_player) = q_source.get_single() {
        // Clones the source entity and spawns a copy
        commands.entity(source_player).clone_and_spawn();
    }
}
```

---

### Best Practices
* ✅ **Do**: Use custom `Relationship` and `RelationshipTarget` pairs to express graph hierarchies.
* ✅ **Do**: Use `#[component(mutable = false)]` to protect structural reference data from accidental runtime mutation.
* ✅ **Do**: Use `.chain()` when system order is strict; let unlinked systems execute in parallel to take advantage of multi-core CPU architectures.
* ❌ **Don't**: Modify relationship target vectors manually; always use the "source of truth" relationship component to mutate links.
* ❌ **Don't**: Write large, monolithic systems; partition your game logic into small systems coordinated by `SystemSet`s and `RunConditions`.

---

### Troubleshooting
* **Problem**: Compiler error when querying `&mut MyComponent`.
  * **Solution**: Your component is marked immutable via `#[component(mutable = false)]`. To modify it, replace it using `commands.insert()` rather than querying for a mutable reference.
* **Problem**: Custom relationship target list is out of sync.
  * **Solution**: You likely tried to mutate the target array directly. Ensure all modifications are done by adding, replacing, or removing the source relationship component instead.

---

## FAQ

### Q: How does Bevy handle child despawning in relationships?
**A**: When defining a `RelationshipTarget` component, you can annotate it with `#[relationship_target(linked_spawn)]`. If this is active (like on Bevy's built-in `Children` target), despawning the parent will automatically trigger a clean recursive despawn of its children.

### Q: Why use Relationships over normal query indexing?
**A**: Relationships are maintained efficiently under the hood via internal component hooks. This avoids manually maintaining lists, minimizes query traversal steps, and operates in constant-time ($O(1)$).
