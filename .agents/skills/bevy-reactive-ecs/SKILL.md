---
name: bevy-reactive-ecs
description: Advanced reactive design patterns in the Bevy game engine (v0.17+), covering immediate synchronous Observers, targeted EntityEvents with propagation, lifecycle events (Add, Remove), and double-buffered Messages.
disable-model-invocation: false
---

### Overview
This skill focuses on building highly responsive, event-driven game logic using Bevy's modern reactive architecture. It helps you decouple system design, implement clean hierarchies via event propagation (bubbling), observe entity lifecycle state changes (addition, removal, replacement), and balance immediate execution against double-buffered, frame-delayed message distribution.

---

### When to Use This Skill
* Use when developing games with Bevy v0.17 or v0.18, where events have been separated into synchronous immediate **Events** and asynchronous deferred **Messages**.
* Use when you need state changes or interactions (like UI clicks, hits, or collisions) to resolve **synchronously** in the same frame.
* Use when nesting entity logic where events must traverse upward through parent-child hierarchies (e.g., hitting armor blocks damage, then propagates up to the character).
* Use when asserting invariants or cleaning up external spatial/database indexes upon component creation or removal.

Do **not** use this skill for heavy, continuous frame-by-frame updates (e.g., simple movement, physics integration, or animation ticks); standard pull-based systems (`Update`, `FixedUpdate` queries) are far better suited and parallelize natively without overhead.

---

### Step-by-Step Guide

#### 1. Distinguishing Messages vs. Immediate Events
In modern Bevy, decoupled communication is split based on execution timing:
* **`Message`**: Asynchronous and double-buffered. Written to a queue and read on the next frame. Ideal for decoupled system-to-system messaging where one frame of delay is perfectly acceptable.
* **`Event` / `EntityEvent`**: Synchronous and immediate. Triggering one halts current execution to run all registered **Observers** instantly.

#### 2. Declaring and Writing Messages
Use `Message` when systems do not require an instant callback.

```rust
use bevy::prelude::*;

#[derive(Message)]
struct PlayerDetected {
    detector: Entity,
    target: Entity,
}

// Writing to the message queue
fn alert_system(
    mut writer: MessageWriter<PlayerDetected>,
    q_enemies: Query<(Entity, &Transform), With<Enemy>>,
) {
    for (enemy_ent, _) in &q_enemies {
        writer.write(PlayerDetected {
            detector: enemy_ent,
            target: Entity::PLACEHOLDER,
        });
    }
}

// Reading from the message queue (next frame)
fn response_system(mut reader: MessageReader<PlayerDetected>) {
    for msg in reader.read() {
        info!("Enemy {:?} spotted target {:?}", msg.detector, msg.target);
    }
}
```

#### 3. Defining Immediate Events & Observers
Observers are specialized systems whose first parameter is `On<E, B = ()>`. They can access standard system parameters like queries, commands, or resources.

```rust
#[derive(Event)]
struct Explosion {
    radius: f32,
    position: Vec3,
}

// Defining an Observer
fn on_explosion(
    trigger: On<Explosion>, 
    mut commands: Commands,
    q_units: Query<(Entity, &Transform)>,
) {
    for (entity, transform) in &q_units {
        if transform.translation.distance(trigger.position) <= trigger.radius {
            commands.entity(entity).despawn();
        }
    }
}
```

#### 4. Triggering and Registering Observers
Observers can be added **globally** to the app or attached **locally** to specific entities.

```rust
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Add global observer
        .add_observer(on_explosion)
        .add_systems(Update, trigger_logic)
        .run();
}

fn trigger_logic(mut commands: Commands) {
    // 1. Triggered via Commands (runs at the next Command flush / Sync point)
    commands.trigger(Explosion {
        radius: 5.0,
        position: Vec3::ZERO,
    });
}

// Immediate triggering with exclusive World access
fn immediate_world_trigger(world: &mut World) {
    // 2. Triggered on World (runs immediately, blocking current function execution)
    world.trigger(Explosion {
        radius: 10.0,
        position: Vec3::ONE,
    });
}
```

#### 5. Observing Component Lifecycle Events
Bevy provides built-in lifecycle events: `Add`, `Insert`, `Replace`, `Remove`, and `Despawn`. They take an optional second generic parameter acting as a component filter (using `OR` logic if a tuple is passed).

```rust
#[derive(Component)]
struct Burned;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Observes when 'Burned' is inserted on any entity
        .add_observer(|trigger: On<Insert, Burned>, mut q_health: Query<&mut Health>| {
            if let Ok(mut health) = q_health.get_mut(trigger.entity) {
                health.current -= 5.0; // Apply instant fire penalty
            }
        })
        .run();
}
```

---

### Examples

#### Example 1: EntityEvents and Hierarchy Propagation (Bubbling)
By deriving `EntityEvent` and using `#[entity_event(propagate, auto_propagate)]`, you allow events to bubble up your parent-child entity hierarchies (via standard `ChildOf` relations).

```rust
use bevy::prelude::*;

#[derive(Component)]
struct Armor(u32);

#[derive(Component)]
struct Health(u32);

// Target-specific entity event with bubbling
#[derive(EntityEvent, Clone)]
#[entity_event(propagate, auto_propagate)]
struct DamageEvent {
    amount: u32,
}

fn setup(mut commands: Commands) {
    // Parent Entity (Character)
    commands.spawn((Name::new("Knight"), Health(100)))
        .observe(on_character_damage)
        .with_children(|parent| {
            // Child Entity (Helmet Armor)
            parent.spawn((Name::new("Helmet"), Armor(15)))
                .observe(on_armor_damage);
        });
}

// 1. Resolves first on the child (Helmet)
fn on_armor_damage(mut trigger: On<DamageEvent>, mut q_armor: Query<&mut Armor>) {
    if let Ok(mut armor) = q_armor.get_mut(trigger.entity) {
        let absorbed = armor.0.min(trigger.amount);
        armor.0 -= absorbed;
        trigger.amount -= absorbed;
        
        info!("Armor absorbed {} damage. Remaining: {}", absorbed, trigger.amount);
        
        if trigger.amount == 0 {
            // Stop bubbling if all damage was completely absorbed
            trigger.propagate(false);
        }
    }
}

// 2. Bubbles up to parent (Knight) if propagation was not stopped
fn on_character_damage(trigger: On<DamageEvent>, mut q_health: Query<&mut Health>) {
    if let Ok(mut health) = q_health.get_mut(trigger.entity) {
        health.0 = health.0.saturating_sub(trigger.amount);
        info!("Character took {} damage! Current HP: {}", trigger.amount, health.0);
    }
}
```

#### Example 2: Scoped Entity Observers & Run Conditions
Instead of defining global listeners, you can attach observers directly to entities. In Bevy 0.17+, global and local observers can also be limited using run conditions.

```rust
#[derive(Component)]
struct Boss;

#[derive(Event)]
struct Enrage;

#[derive(Resource)]
struct GameState {
    is_paused: bool,
}

fn spawn_boss(mut commands: Commands) {
    commands.spawn((Boss, Name::new("Shadow Lord")))
        // Locally scoped observer that only responds on this specific entity
        .observe(|trigger: On<Enrage>, q_boss: Query<&Name>| {
            if let Ok(name) = q_boss.get(trigger.entity) {
                info!("{} is furious!", name);
            }
        });
}

fn configure_app(app: &mut App) {
    app.add_observer(
        // Observers support run conditions to filter triggers based on global state
        (|_trigger: On<Enrage>| {
            info!("Global enrage system executed");
        })
        .run_if(|state: Res<GameState>| !state.is_paused)
    );
}
```

---

### Best Practices

* ✅ **Use Messages for Decoupled Systems**: If immediate execution isn’t required, default to double-buffered `Message`s. This permits easier frame sorting, execution layout planning, and parallel schedule optimization.
* ✅ **Halt Propagation Early**: When leveraging `EntityEvent` bubbling, call `trigger.propagate(false)` as soon as the event is fully resolved to prevent wasted hierarchy traversal.
* ✅ **Keep Observers Lightweight**: Because observers run synchronously, heavy computation inside them will immediately stall system schedulers and command queues.
* ❌ **Don't Assume Execution Order**: Bevy does not guarantee the order in which multiple observers watching the same event will execute. Keep them independent.
* ❌ **Avoid Infinite Loops**: Triggering an event `A` inside an observer that watches event `A` will cause an immediate stack overflow. Always safeguard recursive triggers with conditional checks or structural limits.

---

### Troubleshooting

#### Problem: Stack Overflow / Recursive Panic
* **Cause**: An observer triggered an event that directly or indirectly invoked itself.
* **Solution**: Ensure any triggered commands or recursive events are bound within a condition, or defer execution. If possible, mutate components instead of re-triggering, and track mutations using `Changed<T>`.

#### Problem: Custom EntityEvent Propagation Fails
* **Cause**: Your custom event derive macro is missing parameters, or the target entity lacks structural hierarchy components.
* **Solution**: Ensure your event is marked with `#[entity_event(propagate, auto_propagate)]`. If you do not use `auto_propagate`, you must manually propagate the event using `trigger.propagate(true)`. Ensure parent entities hold child entities properly spawned via `.with_children(...)`.

#### Problem: Stale Data inside Observer Mutators
* **Cause**: Modifying a component on an entity inside an observer might result in stale lookups if you are mixing world access or raw exclusive commands concurrently in the same scheduling block.
* **Solution**: Use command buffers (`Commands::trigger`) rather than raw `world.trigger` if execution can wait until the nearest frame command flush/sync point. This ensures the hierarchy is cleanly finalized.

---

## FAQ

### Q: What is the difference between a Component Hook and an Observer?
**A**: Both were introduced in Bevy 0.14. Component hooks (`on_add`, `on_insert`, `on_remove`) are designed primarily for **upholding data invariants** at the lowest level. They do not support system parameters or arbitrary query access. Observers are **fully-featured systems** that can read and write resources, queries, and run complex scheduling logic on-demand.

### Q: Can observers run in parallel?
**A**: Standard pull-based systems in Bevy run in parallel depending on resource and query mutable access. However, an individual triggered event runs its corresponding observers in sequence. If an observer schedules a command queue trigger, those commands are buffered and deferred, maintaining overall performance safety.

### Q: Why did Bevy rename EventReader/EventWriter?
**A**: To allow the term "Event" to cleanly refer to immediate, observer-backed synchronous actions. The classic, double-buffered frame events were renamed to **Messages** with accompanying `MessageReader` and `MessageWriter` parameters.
