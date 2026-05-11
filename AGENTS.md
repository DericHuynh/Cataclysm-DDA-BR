# Cargo
For running tests, use `cargo nextest run`.

# Bevy ECS 0.18 — Agent Reference

This document exists to prevent common mistakes when working with Bevy ECS 0.18.
It covers the features that have changed most significantly in recent releases and
the patterns that are easy to get wrong. Read it before touching any ECS code.

---

## Table of Contents

1. [Relationships](#1-relationships)
2.[Query Filters vs Option](#2-query-filters-vs-option)
3. [Tag Components](#3-tag-components)
4. [Immutable Components](#4-immutable-components)
5. [Required Components](#5-required-components)
6. [Component Hooks](#6-component-hooks)
7.[Hierarchy — ChildOf / Children](#7-hierarchy--childof--children)
8. [Change Detection](#8-change-detection)
9. [Mutating Components](#9-mutating-components)
10.[Error Handling in Systems](#10-error-handling-in-systems)
11. [Entity Cloning](#11-entity-cloning)
12. [Commands vs Queries](#12-commands-vs-queries)
13. [Messages (Triggers) vs Events](#13-messages-triggers-vs-events)
14. [Common Pitfalls](#14-common-pitfalls)
15.[bevy_ui](#15-bevy_ui)

---

## 1. Relationships

Introduced in **0.16**, relationships are the correct way to model any bidirectional
entity-entity connection. Do not model relationships with manual back-reference
components — Bevy will not keep them in sync.

### Defining a relationship

A relationship requires two components: a **Relationship** (the source of truth,
lives on the "child" entity) and a **RelationshipTarget** (the reverse index,
lives on the "parent" entity, maintained automatically by Bevy hooks).

```rust
/// Lives on the item entity. Points to the container holding it.
#[derive(Component, Debug)]
#[relationship(relationship_target = ContainerContents)]
pub struct InsideContainer(pub Entity);

/// Lives on the container entity. Maintained automatically by Bevy.
/// Never modify the inner Vec directly — use commands to insert/remove InsideContainer.
#[derive(Component, Debug)]
#[relationship_target(relationship = InsideContainer)]
pub struct ContainerContents(Vec<Entity>);  // field is private by default in tuple struct

impl ContainerContents {
    pub fn iter(&self) -> impl Iterator<Item = Entity> + '_ {
        self.0.iter().copied()
    }
}
```

### Relationships with extra data fields

If the relationship needs to carry data beyond the target entity, use a named
struct and annotate the entity field with `#[relationship]`. All other fields
**must implement `Default`**.

```rust
#[derive(Component, Debug)]
#[relationship(relationship_target = WornBy)]
pub struct WornOn {
    #[relationship]
    pub wearer: Entity,
    pub slot: Option<BodyPartSlot>,  // Option<T> implements Default
}
```

### Cascade despawn with `linked_spawn`

Adding `linked_spawn` to the `#[relationship_target]` attribute means despawning
the target entity will also despawn all related entities.

```rust
#[derive(Component, Debug)]
#[relationship_target(relationship = InsideContainer, linked_spawn)]
pub struct ContainerContents(Vec<Entity>);
```

Use this for containment relationships (items in containers, bionics in creatures)
where the children have no meaningful existence without the parent.
Do **not** use it for non-ownership relationships (e.g. a creature targeting another
creature — despawning the target should not despawn the attacker).

### Mutating relationships

Relationship components are mutated by **reinsertion**, not by `&mut` query access.
The hooks that keep both sides in sync only fire on component insert and remove,
not when you modify a field via `&mut`.

```rust
// CORRECT — hooks fire, TargetedBy is updated on both old and new target
commands.entity(item).insert(InsideContainer(new_container));

// WRONG — hooks do not fire, ContainerContents on both containers is now stale
fn bad_system(mut query: Query<&mut InsideContainer>) {
    for mut inside in &mut query {
        inside.0 = new_container; // silent corruption
    }
}
```

Document this on every relationship component with a `# Mutation` doc section.

### Reading the RelationshipTarget

The `RelationshipTarget` inner `Vec` is private by default in a tuple struct.
Expose it through a method, never make the field `pub`:

```rust
impl ContainerContents {
    pub fn iter(&self) -> impl Iterator<Item = Entity> + '_ {
        self.0.iter().copied()
    }
}
```

### Traversal

Use `Query::iter_ancestors` and `Query::iter_descendants` to walk relationship
graphs. These are inherent methods on `Query` as of 0.16 — the old
`HierarchyQueryExt` trait no longer exists.

```rust
fn find_root_container(
    item: Entity,
    inside: Query<&InsideContainer>,
) -> Entity {
    inside.iter_ancestors(item).last().unwrap_or(item)
}
```

---

## 2. Query Filters vs Option

These are different tools. Do not confuse them.

### `With<T>` / `Without<T>` — archetype filters

Use these when a system should **only run on** entities that have (or lack) a
component. Bevy evaluates these at the archetype level — entire archetypes that
don't match are skipped before any iteration occurs. This is the fast path.

```rust
// Only iterates entities that have both Pocket and Sealed.
// Entire archetypes without Sealed are skipped.
fn process_sealed_pockets(query: Query<&Pocket, With<Sealed>>) {
    for pocket in &query { /* guaranteed sealed */ }
}
```

### `Option<&T>` — per-entity presence check

Use this when a system needs to handle entities **with or without** a component
and the logic differs. Every entity in the query is visited; you branch inside
the loop.

```rust
fn process_all_pockets(
    query: Query<(&Pocket, Option<&Sealed>, Option<&Rigid>)>,
) {
    for (pocket, sealed, rigid) in &query {
        if sealed.is_some() { /* ... */ }
        if rigid.is_some() { /* ... */ }
    }
}
```

### When to use which

| Situation | Use |
|---|---|
| System only makes sense for entities with tag | `With<Tag>` filter |
| System skips entities without a component | `With<T>` filter |
| System behaves differently based on presence | `Option<&T>` |
| Checking one flag among many on the same entity | `Option<&T>` |
| Performance-critical inner loop, one code path | `With<T>` filter + separate system |

### Do not query all combinations

Never write one system per combination of tags. The point of tag components is
that each system declares exactly the subset it cares about:

```rust
// WRONG — combinatorial explosion
fn sealed_rigid_pocket(q: Query<&Pocket, (With<Sealed>, With<Rigid>)>) {}
fn sealed_nonrigid_pocket(q: Query<&Pocket, (With<Sealed>, Without<Rigid>)>) {}
fn nonsealed_rigid_pocket(q: Query<&Pocket, (Without<Sealed>, With<Rigid>)>) {}
// ... and so on

// CORRECT — one system that handles the flags it cares about
fn process_sealed(q: Query<&Pocket, With<Sealed>>) {}
fn process_rigid(q: Query<&Pocket, With<Rigid>>) {}
```

---

## 3. Tag Components

Tag components (zero-sized marker types) are used to represent boolean properties
on entities. They enable efficient archetype-level filtering and avoid branching
in hot systems.

### Definition

```rust
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct Sealed;

#[derive(Component, Debug, Default, Clone, Copy)]
pub struct Rigid;
```

### Rules

- **One pattern per codebase.** If you use tag components for boolean properties
  on one type (e.g. `Container`), use them everywhere for the same purpose.
  Do not mix tag components and `bool` fields or bitflag structs on related types
  — it creates two query styles for the same concept.

- **Tags are not enums.** If a property is mutually exclusive (e.g. a pocket is
  either `Magazine` or `Container` type but not both), use an enum field, not
  two tags. Tags are for independent boolean properties.

- **`Default` is required** if you want to use the tag in `#[derive(Bundle)]`
  without specifying it. Always derive `Default` on tag components.

### Prefer focused systems over Option checks for tags

If a property fundamentally changes how a system processes an entity, split into
two systems filtered by `With`/`Without` rather than branching with `Option`:

```rust
// One system that only handles spoilable food not in a preserving container
fn tick_spoilage(
    mut query: Query<&mut Spoilable, Without<PreservesTemp>>,
    time: Res<Time>,
) {
    for mut spoilable in &mut query {
        spoilable.remaining -= time.delta();
    }
}
```

---

## 4. Immutable Components

As of **0.16**, components can be declared immutable. An immutable component
cannot be accessed as `&mut` — it can only be inserted and removed. This is
enforced at compile time.

### When to use

Use immutable components for:
- Any data that must stay consistent with another piece of state
- Components that should only change through a controlled API
- Relationship components where mutation must go through hooks

Bevy's own `ChildOf` is immutable for exactly this reason.

### Syntax

```rust
#[derive(Component)]
#[component(immutable)]
pub struct PlayerId(pub u64);
```

### Mutation pattern for immutable components

Since you cannot take `&mut`, you change the value by removing and reinserting:

```rust
// Remove old, insert new
commands.entity(e).remove::<PlayerId>().insert(PlayerId(new_id));
// Or just insert — inserting replaces the existing component
commands.entity(e).insert(PlayerId(new_id));
```

### Relationship components and immutability

The official Bevy relationships example does **not** mark relationship components
as `#[component(immutable)]` — they use reinsertion by convention rather than
compiler enforcement. However, for components where accidental `&mut` access
would corrupt invariants, immutability is the safer choice.

If you choose not to use `#[component(immutable)]` on a relationship component,
add a doc comment warning:

```rust
/// # Mutation
/// Do not query as `&mut`. Mutate by reinserting via commands:
/// `commands.entity(e).insert(InsideContainer(new_container));`
#[derive(Component)]
#[relationship(relationship_target = ContainerContents)]
pub struct InsideContainer(pub Entity);
```

---

## 5. Required Components

Introduced in **0.15**, required components allow you to declare that inserting
one component automatically inserts others with their default values.

### Definition

```rust
#[derive(Component, Default)]
#[require(Health, Faction)]  // automatically inserted when Creature is inserted
pub struct Creature {
    pub name: String,
    pub species: SpeciesId,
    pub symbol: char,
}
```

### Rules

- Required components use the `Default` implementation of the required type.
  If the required component has no sensible default, do not use this mechanism —
  use a `Bundle` instead.
- Required components are **not** the same as inheritance. Both components remain
  separate types that you query independently.
- Overriding: if you explicitly insert a required component in the same spawn
  call, your value wins over the default.

```rust
// Health is inserted with its Default, Faction is overridden
commands.spawn((Creature { .. }, Faction { id: player_faction }));
```

### Bundles vs Required Components

Use **required components** when a component always needs companion components
to function (structural dependency).

Use **bundles** when you want a convenient shorthand for a common spawn pattern
that isn't always true for that component type.

---

## 6. Component Hooks

Hooks are functions that run automatically when a component is added, replaced,
or removed from an entity. They are the mechanism that powers relationships.

### Available hooks

- `on_add` — fires when the component is first added (not on replacement)
- `on_insert` — fires when the component is added or replaced
- `on_replace` — fires just before the component is replaced or removed
- `on_remove` — fires when the component is removed (not on replacement)
- `on_despawn` — fires when the entity is despawned

### Defining hooks via derive attribute

```rust
#[derive(Component)]
#[component(on_insert = my_on_insert, on_remove = my_on_remove)]
pub struct Tracked;

fn my_on_insert(mut world: DeferredWorld, ctx: HookContext) {
    // ctx.entity is the entity the component was inserted on
    world.commands().entity(ctx.entity).insert(TrackedMarker);
}
```

### Important constraints

- Hooks run in deferred world context (`DeferredWorld`). You cannot make
  structural changes (spawn, despawn, add/remove components) directly — use
  `world.commands()` to defer them.
- Hooks must not panic. A panicking hook corrupts ECS state.
- Do not use hooks for game logic. Use them for maintaining invariants
  (keeping indexes in sync, enforcing constraints). Game logic belongs in systems.

---

## 7. Hierarchy — ChildOf / Children

The `Parent` component was renamed to `ChildOf` in **0.16**. All old code
referencing `Parent` must be updated.

### Current API

```rust
// Spawning with a parent
commands.spawn((MyComponent, ChildOf(parent_entity)));

// Spawning children ergonomically
commands.entity(parent).with_children(|p| {
    p.spawn(ChildComponent);
});

// Despawning
commands.entity(parent).despawn();                    // despawns parent only, removes ChildOf from children
commands.entity(parent).despawn_related::<Children>(); // despawns all children, not the parent
```

### Do not confuse ChildOf/Children with custom relationships

`ChildOf`/`Children` propagate `Transform` and `Visibility`. Do not use the
built-in hierarchy for inventory, equipment, or logical grouping — define custom
relationships for those. Putting an item "inside a container" using `ChildOf`
will trigger transform propagation you don't want.

### Querying ancestors and descendants

```rust
fn find_root(entity: Entity, child_of: Query<&ChildOf>) -> Entity {
    child_of.iter_ancestors(entity).last().unwrap_or(entity)
}
```

---

## 8. Change Detection

Bevy tracks which components changed each frame. Systems can filter on this to
avoid unnecessary work.

### Filters

```rust
Query<&Transform, Changed<Transform>>  // only entities whose Transform changed this frame
Query<&Health, Added<Armor>>           // only entities that had Armor added this frame
```

### The Vec-in-component problem

If you store a `Vec` inside a component (e.g. `Vec<StatusEffect>`), any mutation
to any element requires `&mut ComponentType`, which marks the **entire component**
as changed. This causes all systems using `Changed<ComponentType>` to re-run
even if the data they care about didn't change.

**Solution:** move each element to its own entity via a relationship. Then change
detection fires only on the specific effect/bonus entity that actually changed.

```rust
// BAD — ticking one effect marks ALL of StatusEffects as changed
#[derive(Component)]
pub struct StatusEffects {
    pub effects: Vec<StatusEffectInstance>,
}

// GOOD — ticking one effect only marks that one StatusEffect entity as changed
#[derive(Component)]
pub struct StatusEffect {
    pub effect_id: EffectId,
    pub intensity: u32,
    pub remaining: Time,
}
// Each effect is its own entity, related to the creature via EffectOn relationship
```

### Avoid over-mutating

Only take `&mut` when you will actually write. If you conditionally mutate,
use the `bypass_change_detection` escape hatch only as a last resort and document
why.

---

## 9. Mutating Components

Querying for `&mut T` gives you mutable access to a component, but it comes with hidden costs. You should only mutate components when strictly necessary. 

### The Scheduler Lock
Bevy's multi-threading scheduler analyzes your system signatures to determine what can run in parallel. If a system queries `&mut Health`, the scheduler will block **all other systems** from reading or writing `Health` until that system finishes.
*   **Rule:** Never query `&mut T` if you only intend to read it. Always use `&T`.
*   If mutation is rare, consider querying `&T`, and then applying updates via `Commands` or specific events, so you don't hold an exclusive lock on the component archetype for the entire system execution.

### Unintentional Change Detection
In Bevy, modifying a value via `&mut T` immediately flags the component as `Changed<T>`. This forces all downstream systems listening for `Changed<T>` to re-run.

If you overwrite a component with the *exact same value*, Bevy still marks it as changed because the `Mut<T>` deref occurred.

**Use `set_if_neq`:**
To avoid false change detection spam, always verify the value is actually different before mutating. Bevy provides the `.set_if_neq()` method for components that derive `PartialEq`.

```rust
// BAD — Triggers Changed<PlayerState> every frame, destroying performance downstream
fn update_state(mut q: Query<&mut PlayerState>) {
    for mut state in &mut q {
        state.0 = State::Idle;
    }
}

// GOOD — Only triggers Changed<PlayerState> once, when the state actually shifts
fn update_state(mut q: Query<&mut PlayerState>) {
    for mut state in &mut q {
        state.set_if_neq(PlayerState(State::Idle));
    }
}
```

### When is it OK to mutate?
| Component Type | Guideline |
|---|---|
| **Continuous State** (`Transform`, `Velocity`, `Timer`) | Change every frame. Mutate freely inside loops using `&mut T`. |
| **Discrete State** (`Health`, `Faction`, `Name`) | Change rarely. Mutate exclusively via `.set_if_neq()` or by replacing the component via `Commands` to ensure change detection only fires on actual modification. |
| **Relational State** (`InsideContainer`) | **Never** mutate directly via `&mut T`. Mutating a raw field bypasses component hooks. Always use `commands.entity().insert()`. |

---

## 10. Error Handling in Systems

As of **0.16**, systems can return `Result`. Bevy logs errors returned from
systems automatically.

### Fallible systems

```rust
fn my_system(
    query: Query<&SomeComponent>,
) -> Result<()> {
    let component = query.single()?;  // returns Err if not exactly one entity
    Ok(())
}
```

### `Query::single()` now returns `Result`

`Query::single()` and `Query::single_mut()` return `Result` as of 0.16.
The old `Query::get_single()` is deprecated. Update call sites:

```rust
// Old (0.15)
if let Ok(player) = query.get_single() { }

// New (0.16+)
if let Ok(player) = query.single() { }
// or in a fallible system:
let player = query.single()?;
```

### `Query::many()` is deprecated

Use `Query::get_many()` instead, which returns `Result` rather than panicking.

---

## 11. Entity Cloning

As of **0.16**, entity cloning is supported natively. Add `#[derive(Clone)]`
to a component to make it cloneable.

```rust
#[derive(Component, Clone)]
pub struct Item {
    pub name: String,
    pub volume: Volume,
}

// Clone an entity and all its cloneable components
let new_entity = world.commands().entity(source).clone().id();
```

Components without `Clone` are silently skipped during cloning. If a component
must always be cloned with its entity, derive `Clone` and document it. If a
component must **never** be cloned (e.g. a unique identity component), document
that explicitly.

---

## 12. Commands vs Queries

These represent the two fundamental ways to interact with the Bevy ECS World. Do not use them interchangeably.

### Queries (`Query<&T>`, `Query<&mut T>`)
Queries are for **immediate, synchronous data access**. When you mutate data via `&mut T` in a system loop, the change happens instantly at that line of code.
*   **Use when:** Reading or modifying the internal values of a component on entities that already exist.
*   **Performance:** Extremely fast, iterating directly over contiguous memory archetypes.

### Commands (`Commands`)
Commands are for **deferred, structural changes** to the ECS World. When you call a command, nothing happens immediately. Instead, Bevy queues the operation and applies it at the end of the schedule (the "command flush").
*   **Use when:** Spawning or despawning entities, inserting or removing components, or triggering observers.
*   **Performance:** Slower than queries because it allocates command objects and requires exclusive World access later to apply them.

### The Golden Rule
Never use `Commands` to change a component's internal state if a `Query` can do it. Use Commands only when the *structure* of the entity is changing (e.g., adding/removing the component entirely), unless the component is marked `#[component(immutable)]`.

---

## 13. Messages (Triggers) vs Events

Bevy provides two distinct mechanisms for systems to communicate and trigger logic without direct coupling.

### Events (`EventWriter<T>` / `EventReader<T>`)
Events are **global broadcasts**. Think of them as announcements over a loudspeaker: "Something happened, anyone who cares can react."
*   **Mechanism:** Written to a global queue. Read later in the frame by any number of systems using `EventReader`.
*   **Best for:** Game-wide occurrences where multiple unlinked systems must react independently (e.g., `TimeChanged`, `GameSaved`, `SoundEffectRequested`).
*   **Warning:** Events buffer in memory until consumed or the frame ends. Checking events requires iterating the reader in every system that listens, which is inefficient if the event is only meant for one specific entity.

### Messages / Triggers (Observers)
Messages (built on Bevy's Observer system) are **targeted interactions**. Think of them as direct mail: "Hey Entity X, execute this logic."
*   **Mechanism:** Fired via `commands.trigger(MyMessage)` or `commands.entity(e).trigger(MyMessage)`. Handled immediately (or on flush) by an `Observer` system attached to that entity or the world.
*   **Best for:** Entity-to-entity interactions (e.g., `TakeDamage`, `OpenContainer`), UI input (`Pointer<Click>`), or component lifecycle logic.
*   **Why use them?** They route logic directly to the entity that is being targeted. This skips the need for a global system to loop through every `DamageEvent` and match entity IDs.

| Scenario | Pattern | Implementation |
|---|---|---|
| The sun sets. Plants, zombies, and solar panels need to update. | **Event** | `EventWriter<NightFall>` |
| Zombie attacks a specific door. | **Message** | `commands.entity(door).trigger(TakeDamage)` |
| User clicks a UI button. | **Message** | Built-in `Pointer<Click>` observer on the button |

---

## 14. Common Pitfalls

### Multiple components of the same type per entity — impossible

Bevy silently overwrites a component if you insert the same type twice on the same entity. If you need multiple instances of something, either:
- Store a `Vec` inside the component (with change detection caveats above)
- Make each instance its own entity via a relationship

```rust
// WRONG — second insert silently replaces the first
commands.entity(e).insert(InstalledBionic { .. }).insert(InstalledBionic { .. });

// CORRECT — each bionic is its own entity related to the creature
commands.spawn((Bionic { .. }, BionicOf(creature)));
```

### Redundant marker components

Do not add a marker component that duplicates information already present via
another component. An entity with `PlayerData` is a player — `IsPlayer` is
redundant and will desync.

```rust
// BAD
commands.spawn((PlayerData { .. }, IsPlayer));  // IsPlayer can be removed without removing PlayerData

// GOOD — query for PlayerData directly
fn player_system(query: Query<&Health, With<PlayerData>>) {}
```

### Relationship target field must be private

The inner `Vec<Entity>` of a `RelationshipTarget` component must not be `pub`.
Making it public allows direct mutation that bypasses hooks and corrupts the
bidirectional link. Expose data through methods only:

```rust
// WRONG
#[derive(Component)]
#[relationship_target(relationship = InsideContainer)]
pub struct ContainerContents(pub Vec<Entity>);  // pub field — hooks can be bypassed

// CORRECT
#[derive(Component)]
#[relationship_target(relationship = InsideContainer)]
pub struct ContainerContents(Vec<Entity>);  // private field

impl ContainerContents {
    pub fn iter(&self) -> impl Iterator<Item = Entity> + '_ {
        self.0.iter().copied()
    }
}
```

### ChildOf is not for logical relationships

Do not use `ChildOf`/`Children` to express inventory, equipment, or any logical
grouping. It propagates `Transform` and `Visibility` and has special despawn
semantics. Use a custom relationship instead.

### Stale HierarchyQueryExt usage

`HierarchyQueryExt` no longer exists. Its methods (`iter_ancestors`,
`iter_descendants`) are now inherent methods on `Query`. Remove any
`use bevy::hierarchy::HierarchyQueryExt;` imports.

### PocketFlags / bool fields alongside tag components

If the codebase uses tag components for boolean properties, do not introduce
a bitflag struct or `bool` fields for the same purpose on a related type.
Pick one pattern and apply it everywhere. Mixed patterns mean some properties
are archetype-queryable and some are not, with no indication at the call site
which is which.

### Querying `&mut` on a relationship component

Querying a relationship component as `&mut` and modifying its entity field
does not fire hooks. The `RelationshipTarget` on the old and new target
entities will not be updated. Always use `commands.entity(e).insert(...)` to
change a relationship.

### `Vec` inside components and change detection

Storing `Vec<T>` inside a component causes the whole component to be marked
changed whenever any element changes. For frequently-ticking collections
(status effects, morale bonuses, skill experience), prefer per-entity
relationship-based designs so change detection is granular.

### `StackCount` / numeric components with no zero contract

Any component wrapping a count or quantity should document its valid range and
what happens at the boundary. A `StackCount(0)` is meaningless — the entity
should be despawned. Enforce this with a private field and a constructor.

---

## 15. bevy_ui

`bevy_ui` is Bevy's built-in, native ECS user interface system. It is powered by Taffy and primarily uses Flexbox and CSS Grid paradigms. Every UI element is a standard Bevy entity with a `Node` component.

### Root setup

Every UI tree requires a root node. Because UI nodes are required components, you no longer spawn "bundles". You simply spawn a `Node` and configure its layout properties.

Always use `with_children` to build the tree. Ensure your root node covers the whole screen by setting width and height to 100%.

```rust
commands.spawn((
    Node {
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    },
    MySceneMarker,
)).with_children(|ui| {
    // all child UI nodes go here
});
```

### Layout types (Flexbox & Grid)

`bevy_ui` supports two layout specifications, controlled via the `display` field:

| Layout | When to use |
|---|---|
| `Display::Flex` | (Default) Stacking children in a row or column. Used for 90% of UI elements. |
| `Display::Grid` | Complex 2D layouts. Aligns children into defined rows and columns. |
| `Display::None` | Visually hides the node and removes it from the layout calculations entirely. |

### Units (Val)

The `Val` enum dictates how dimensions are calculated. Always match the unit to your responsive design needs.

| Unit | Meaning |
|---|---|
| `Val::Px(v)` | Absolute physical pixels. |
| `Val::Percent(v)` | Percentage of the **parent node's** dimension. |
| `Val::Vw(v)` / `Val::Vh(v)` | Percentage of the total viewport width/height. |
| `Val::Auto` | Sizes based on the element's content (e.g., wrapping text). |

### Text rendering — critical rules

Text in Bevy UI is a native ECS component attached directly to a `Node`. You do not use `Text2d` (which is for world-space text).

**1. Use `Text::new()` alongside `TextFont`.**
If you do not specify a `TextFont`, Bevy will use a default built-in font. 

**2. Text wrapping requires boundaries.**
If text is bleeding off the screen or refusing to wrap, it means its parent `Node` has no constraints. You must set a `max_width` or `width` on the parent container, and ensure text wrapping is enabled on the `TextLayout`.

```rust
// CORRECT — Text component embedded in a standard node spawn
ui.spawn((
    Text::new("START GAME"),
    TextFont { font_size: 32.0, ..default() },
    TextColor(Color::srgb(0.9, 0.9, 0.9)),
    Node {
        margin: UiRect::all(Val::Px(10.0)),
        ..default()
    },
));
```

### Positioning: Relative vs Absolute

The `position_type` field dictates whether a node respects its parent's Flexbox/Grid flow.

- **`PositionType::Relative` (Default):** The node is pushed around by its siblings and parent `justify_content` rules.
- **`PositionType::Absolute`:** The node ignores siblings entirely. It positions itself relative to its closest explicitly sized parent using the `left`, `right`, `top`, and `bottom` fields. Useful for overlay badges, tooltips, or floating windows.

```rust
// An absolute badge anchored to the top-right of its parent
ui.spawn((
    Node {
        position_type: PositionType::Absolute,
        top: Val::Px(5.0),
        right: Val::Px(5.0),
        ..default()
    },
    BackgroundColor(Color::RED),
));
```

### Interaction and Observers (Hover / Click)

Do not constantly poll UI state in global systems. Instead, attach an Observer to the UI entity to handle input events directly. This is extremely fast and cleanly decouples logic.

```rust
ui.spawn((
    Node {
        width: Val::Px(150.0),
        height: Val::Px(50.0),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    },
    BackgroundColor(Color::srgb(0.2, 0.2, 0.2)),
))
// Trigger logic when clicked
.observe(|trigger: Trigger<Pointer<Click>>, mut commands: Commands| {
    commands.trigger(StartGameMessage); // Route logic via Message
})
// Visual hover states
.observe(|trigger: Trigger<Pointer<Over>>, mut queries: Query<&mut BackgroundColor>| {
    if let Ok(mut bg) = queries.get_mut(trigger.entity()) {
        bg.0 = Color::srgb(0.4, 0.4, 0.4);
    }
})
.observe(|trigger: Trigger<Pointer<Out>>, mut queries: Query<&mut BackgroundColor>| {
    if let Ok(mut bg) = queries.get_mut(trigger.entity()) {
        bg.0 = Color::srgb(0.2, 0.2, 0.2);
    }
});
```

### Picking / hit testing

By default, any UI node with a `BackgroundColor` or `Image` will block clicks from passing through to the entities behind it.

If you have a full-screen semi-transparent overlay or a decorative visual element that should not intercept clicks, attach `PickingBehavior::IGNORE`.

```rust
ui.spawn((
    Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
    BackgroundColor(Color::BLACK.with_alpha(0.5)),
    PickingBehavior::IGNORE, // Clicks will pass right through this node
));
```

### Scene lifecycle

Mark the root entity with a scene component and despawn by querying for it when changing screens:

```rust
#[derive(Component)]
pub struct MainMenuScene;

// Spawn
commands.spawn((
    Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
    MainMenuScene
)).with_children(|ui| { /* ... */ });

// Despawn (on state exit or screen change)
fn despawn_main_menu(mut commands: Commands, q: Query<Entity, With<MainMenuScene>>) {
    for e in &q { 
        commands.entity(e).despawn_recursive(); 
    }
}
```

### Common bevy_ui pitfalls

- **Using `ZIndex` to fix layout bugs:** If you find yourself slapping `ZIndex(100)` onto nodes to force them to the front, your tree hierarchy is likely wrong. Sibling order dictates render order. Only use `GlobalZIndex` for things like floating tooltips or detached context menus.
- **`Transform` manipulation:** Do not add a `Transform` component to scale or move UI nodes. `bevy_ui` manages positions exclusively through the `Node` properties.
- **Forgetting `despawn_recursive()`:** If you call `.despawn()` on a UI parent, it leaves all the child UI nodes orphaned and broken on the screen. Always use `despawn_recursive()` or rely on custom hierarchical relationships.
