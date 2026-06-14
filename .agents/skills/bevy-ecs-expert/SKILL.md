---
name: bevy-ecs-expert
description: Master Bevy's fundamental Entity Component System (ECS) in Rust, covering modern component definitions, Required Components, Query APIs, Resource state management, and basic scheduling.
disable-model-invocation: false
---

### Overview
A foundational guide to building high-performance, data-oriented game logic using Bevy's ECS architecture. Learn how to structure components, declare required dependencies, optimize database queries, manage global resources, and register systems into the main execution pipeline.

### When to Use This Skill
* Use when developing games with the Bevy engine in Rust.
* Use when designing systems that retrieve and process entities and components.
* Use when leveraging the modern `#[require(...)]` syntax to declare structural dependencies on components.
* Use when organizing systems into stage-based execution blocks (e.g., `Startup`, `Update`).

---

### Step-by-Step Guide

#### 1. Defining Components with Required Components
Components are plain Rust structs. Use `#[require(...)]` to set compile-time requirements for other components, substituting the old manual "Bundle" nesting pattern. Required components must implement `Default` or use a custom inline constructor.

```rust
use bevy::prelude::*;

#[derive(Component, Default)]
struct Velocity {
    x: f32,
    y: f32,
}

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
// Player requires Velocity (initialized to default) and Name (using a custom constructor expression)
#[require(Velocity, Name = default_name())]
struct Player;

fn default_name() -> Name {
    Name::new("Unnamed Hero")
}
```

#### 2. Writing Systems and Querying Data
Systems are regular Rust functions that query the ECS world. Use specific component access constraints to maintain clean execution paths.

```rust
fn movement_system(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &Velocity), With<Player>>,
) {
    let dt = time.delta_seconds();
    for (mut transform, velocity) in &mut query {
        transform.translation.x += velocity.x * dt;
        transform.translation.y += velocity.y * dt;
    }
}
```

#### 3. Managing Global State with Resources
Resources store global, non-entity-bound game data.

```rust
#[derive(Resource, Default)]
struct GameScore {
    score: u32,
}

fn score_system(mut game_score: ResMut<GameScore>) {
    game_score.score += 100;
}
```

#### 4. Scheduling Systems
Register your resources and systems with the `App` builder.

```rust
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<GameScore>()
        .add_systems(Update, movement_system)
        .add_systems(Update, score_system)
        .run();
}
```

---

### Examples

#### Example 1: Overriding Required Components
When you spawn an entity with a component that specifies requirements, the engine automatically inserts the defaults unless you explicitly override them during spawn.

```rust
fn setup(mut commands: Commands) {
    // Spawns a Player with Name("Unnamed Hero") and Velocity { x: 0.0, y: 0.0 }
    commands.spawn(Player);

    // Spawns a Player but overrides Velocity with custom values
    commands.spawn((
        Player,
        Velocity { x: 15.0, y: -5.0 },
    ));
}
```

#### Example 2: Query Filters (With, Without, and Changed)
Filters keep loop iterations clean, avoiding unnecessary processing of entities that do not match the desired game state.

```rust
#[derive(Component)]
struct Stunned;

fn active_enemy_movement(
    mut query: Query<&mut Transform, (With<Enemy>, Without<Stunned>, Changed<Transform>)>,
) {
    for mut transform in &mut query {
        // This only processes enemies that are not stunned and whose transforms have changed
    }
}
```

---

### Best Practices
* ✅ **Do**: Use `#[require(...)]` to build component hierarchies, keeping your spawn sites clean and unified.
* ✅ **Do**: Prefer `Res` over `ResMut` where read-only access is sufficient; this allows Bevy's scheduler to run more systems in parallel.
* ❌ **Don't**: Store complex logic or references inside Components; they should remain as pure, serializable data structs.
* ❌ **Don't**: Use raw `RefCell` or lock mechanisms inside components to bypass borrow checker rules; let the ECS pipeline manage safe borrowing via system parameters.

---

### Troubleshooting
* **Problem**: The system panics at runtime with a "Query conflict" or "Resource conflict".
  * **Solution**: Two systems running in the same schedule phase are requesting mutable access to the same component/resource. Order them using `.chain()`, `.after()`, or split the logic into separate systems.
* **Problem**: Compilation fails because a required component does not implement `Default`.
  * **Solution**: You must either derive/implement `Default` on the required component, or specify an explicit constructor expression within the attribute, like `#[require(MyComponent = MyComponent::new(10))]`.

---

## FAQ

### Q: Are Bundles deprecated in modern Bevy?
**A**: Bundles are not fully deprecated, but their usage has changed. Instead of defining custom structs with `#[derive(Bundle)]` to group components, developers are encouraged to use the `#[require(...)]` attribute directly on a marker component. Tuples of components (e.g., `(Player, Velocity { ... })`) remain the standard way to insert multiple components during spawning.

### Q: What is the overhead of using required components?
**A**: Required components are evaluated at compile time and set up efficient archetype tables during app initialization. There is no additional runtime search overhead compared to manual spawning.
