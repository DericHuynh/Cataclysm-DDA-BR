# cdda_render DOX

## Purpose
Owns all Bevy-side visuals: UI screens (Node/Button/Text), the Tile2d overmap
viewer, the Text2d ASCII dev-worldgen viewport, tileset loading, font loading,
and the shared `UiTheme`. Read-only with respect to simulation — renders
component/resource state into pixels. Layer 5 per `crates/AGENTS.md`.

It also hosts the **screen input adapters** (`render/input.rs`) — the
"presenter" systems that translate `InputAction` (UI vocabulary) into
`cdda_sim` use-case calls. This is why `cdda_sim` stays free of `GameAction`.

## Ownership
- `CddaRenderPlugin` (in `render/mod.rs`) is the single Bevy entry point. It
  registers resources (`UiFontHandle`, `UiTheme`, `SettingsState`,
  `CharacterSheetState`, `DevSpawnFocus`), the `Startup` systems
  `render_setup` + `tiles::load_tiles`, the shared `refresh_all_footer_hints`
  system, and per-screen `OnEnter`/`Update` schedules keyed on `Ctx` states.
- Bevy deps (per `Cargo.toml`): `bevy` with `bevy_asset`, `bevy_core_pipeline`,
  `bevy_pbr`, `bevy_render`, `bevy_sprite`, `bevy_text`, `bevy_ui`, `bevy_winit`,
  `x11`, `ui_picking`; plus `bevy_ecs`, `bevy_state`, `leafwing-input-manager`,
  `serde`, `serde_json`, `tracing`. The `ui_picking` feature enables
  `On<Pointer<Click>>` observers on UI `Button`s, giving mouse input to menus
  (Settings tabs, main-menu command buttons). **No** tilemap crate
  (`bevy_fast_tilemap` etc.) — tile rendering is plain `bevy_sprite`
  (`Sprite` + `Image` handles) driven by a hand-built `TileRegistry`.
- Per-screen renderers depend on `cdda_context` for the `CddaScreen` trait
  (`Ctx`, `ACTIONS`, `spawn`, `update`) and `cdda_input` for `BindableAction` /
  `ActiveKeybindings`. They never write sim or navigation state directly.

## Local Contracts
- All UI uses Bevy `Node` / `Button` / `Text` — no `bevy_fast_tilemap`, no
  custom shader pipeline. Tile sprites are `bevy_sprite` with
  `Sprite::custom_size` sized from `TileInfo::sprite_size()`.
- **Screen input adapters live here, not in `cdda_sim`.**
  `render/input.rs` holds `crafting_menu_input`, `inventory_screen_input`,
  `dev_pickup_drop_system`, and `dev_spawn_input`. These read `InputAction` and
  call `cdda_sim` use-case functions. `cdda_sim` never matches `GameAction`.
- Screen lifecycle follows `cdda_context::screen::CddaScreen`: each screen is
  a unit struct that implements the trait (`InventoryScreen`, `CraftingScreen`,
  `CharacterScreen`, `ExamineScreen`, `DevSpawnScreen`, `RegistryScreen`).
  `OnEnter(Self::CTX)` calls `spawn`; `Update.run_if(in_state(Self::CTX))` calls
  `update`. UI trees are tagged `DespawnOnExit(Self::CTX)` for atomic cleanup.
- Renderers read state only. State changes go through `cdda_input` actions
  (`Confirm`, `UseItem`, `NavigateUp`, `Drop`, …) and `cdda_context` nav.
- Theming is hand-coded, not JSON. `theme::UiTheme` (Resource) wraps a
  `ThemePreset` (`Blue` default, `Green`, `Amber`) plus a fixed-colour
  constants block (`BG`, `PANEL_BG`, `TEXT_BRIGHT`, `BUTTON_FOCUS_BG`, …).
  Switch presets via `SettingsScreen`; every screen reads `Res<UiTheme>`
  instead of hard-coding colour.
- Fonts: `UiFontHandle(Option<Handle<Font>>)` is loaded in `Startup` for
  `assets/fonts/Inter-VariableFont.ttf` (all `bevy_ui` `Text`). The ASCII
  viewport (`dev_worldgen`) loads `assets/fonts/ShareTechMono-Regular.ttf`
  separately for `Text2d`. Both assets live under `crates/cdda_app/assets/`
  and resolve through Bevy `AssetServer` by relative path.
- Tileset base path is hard-coded via
  `concat!(env!("CARGO_MANIFEST_DIR"), "/../../gfx/UltimateCataclysm")`
  (i.e. the repo-root `gfx/UltimateCataclysm/`). Tiles are loaded with
  `AssetServer::load` using the `gfx/...` subpath; `tile_info.json` drives
  per-sheet sprite dimensions and offsets; the final `TileRegistry` resource
  maps CDDA entity ID → `TileInfo` (image handle + sprite size + CDDA offset)
  with OMT-suffix fallback (`barn_0_south` → `barn`).
- Footer key hints: every screen tags one text entity with `FooterHint`; the
  shared `refresh_all_footer_hints` system rewrites it each frame from
  `ContextActions` + `ActiveKeybindings`. No per-screen footer systems.
- **Scroll uses Bevy's native `ScrollPosition` + `Overflow::scroll_y()`**, driven
  by the shared `render/scroll.rs` primitives (`KeyboardScroll` marker, arrow/page
  keys, mouse-wheel, and focused-row keep-visible), not hand-rolled virtual
  windowing. Item-heavy panes additionally carry a `VirtualList`, which **virtualizes
  rendering up-front**: the pane spawns only the visible row window plus top/bottom
  spacer nodes, so Bevy layout never processes 40k rows. `update_virtual_windows`
  (PreUpdate) syncs each `VirtualList`'s window from its `ScrollPosition`. A new
  long-list pane should attach `KeyboardScroll` + `VirtualList` (and `FocusedRow`
  if it has a focused-row index) to a `scroll_y()` node rather than spawning all
  rows or re-implementing scroll offsets.

## Work Guidance
- Add a new screen by (1) creating a unit struct in its own module,
  (2) implementing `CddaScreen` with the right `Ctx` and `ACTIONS` list,
  (3) wiring `OnEnter`/`Update` in `CddaRenderPlugin::build`, (4) tagging the
  root entity with `DespawnOnExit(Self::CTX)`. If the screen needs a focused
  resource, declare it in the per-screen file and `init_resource` it in
  `CddaRenderPlugin::build`.
- New colours belong in `theme.rs` (constants or `ThemePreset` method). Never
  inline `Color::srgb(...)` in a screen module.
- New tile assets go under `gfx/UltimateCataclysm/`; update
  `tile_info.json` if introducing a new sheet size. The `load_tiles` walker
  reads `tile_info.json` order to decide overwrite priority
  (small → normal → large → giant, higher wins).
- Coordinate with `cdda_input` when adding a new `BindableAction`; the footer's
  `ACTIONS` table must list it for the hint to appear.
- Mouse interaction: menu `Button`s may attach `.observe(On<Pointer<Click>>)`
  (enabled by the `ui_picking` feature) to react to mouse clicks. Prefer routing
  a click into the same `InputAction`/`NextState` path keyboard uses, so there
  is one source of truth per transition.

## Verification
- `cargo check -p cdda_render` for compile sanity.
- `cargo nextest run -p cdda_render` (fall back to `cargo test -p cdda_render` if `nextest` is unavailable).
- `cargo run -p cdda_app` for visual smoke validation after render changes.
- `cargo run -p cdda_cli -- schedule-graph` to confirm screen systems land in
  the expected `GameSet` (`Input`, `Sim`, `Render`).

## Child DOX Index

Per-screen files in `src/render/`:

- `mod.rs` — `CddaRenderPlugin`, `UiFontHandle`, `refresh_all_footer_hints`,
  `render_setup` (camera + Inter font), `FooterHint` marker, and the shared
  scroll systems (`scroll::scroll_with_keyboard`/`scroll_with_wheel`/
  `scroll_to_focused_row`). **Wiring file —
  edit this first** when adding/changing screens.
- `scroll.rs` — Shared, idiomatic scroll primitives (`KeyboardScroll` marker,
  `FocusedRow`, `VirtualList` row virtualization with spacer nodes, systems for
  arrow/page keys, mouse wheel, keep-focused-visible, and window sync). Wraps
  Bevy's `ScrollPosition + Overflow::scroll_y()`; attach to any scrollable pane
  instead of hand-rolling clip/window logic.
- `theme.rs` — `UiTheme` resource, `ThemePreset` enum (Blue/Green/Amber),
  fixed colour constants. Single source of truth for palette.
- `tiles.rs` — `TileRegistry`, `TileInfo`, `load_tiles` startup system,
  `tile_info.json` parser, OMT-suffix stripping, sprite manifest ingestion.
- `registry.rs` — `RegistryScreen` (`Ctx::RegistryViewer`); debug viewer over
  the def registry: title, category panel, entry list, detail + raw-JSON +
  parsed-fields panels. **Pane-focus navigation**: `Tab`/`Shift+Tab` cycles the
  focused pane (categories → entries → raw JSON → parsed); `←/→` switch
  category / swap detail panes; `↑/↓`/`PgUp`/`PgDn` navigate within the focused
  list pane. The four panes are native `KeyboardScroll` scroll nodes; the two
  list panes (categories, entries) are index-navigated `VirtualList` panes
  (virtualized rendering, focus keep-visible), and the raw/parsed panes are
  free-scrolling text panes. Each def shows a STATUS line reporting the
  round-trip/coverage check result, and the active pane is tinted. Owns
  `RegistryViewerState` (`categories`, `entries`, `pane`).
- `main_menu.rs` — `main_menu::spawn` / `sync_focus` for `Ctx::MainMenu`;
  bevy_ui `Node` flex column of `CommandButton`s. Command buttons observe
  `On<Pointer<Click>>` (mouse) and set the focused command + emit a `Confirm`
  `InputAction`, so mouse and keyboard share the single `handle_navigation_input`
  dispatch path.
- `settings.rs` — Settings screen (`Ctx::SettingsMenu`); the five tabs
  (General, Graphics, Sound, Interface, Keybindings) are driven by the `SettingsTab`
  Bevy `SubStates` from `cdda_context::substate`. The frame spawns on
  `OnEnter(Ctx::SettingsMenu)`; the active tab's rows are rebuilt from
  `State<SettingsTab>` + `SettingsState` via `rebuild_content_panel` (running on
  `Changed`), the tab bar switches via `NextState<SettingsTab>`, and each tab
  button observes `On<Pointer<Click>>` for mouse navigation. Owns `SettingsState`
  (focused row, in-progress rebind, interface theme).
- `inventory.rs` — `InventoryScreen` (`Ctx::Inventory`); three-panel pocket /
  wielded / worn layout; reads `ItemTypeRegistry`, `TileRegistry`. All three
  lists are native `scroll::KeyboardScroll` panes (arrow keys + wheel scroll).
- `crafting.rs` — `CraftingScreen` (`Ctx::CraftingMenu`); recipe browser with
  category tabs, sub-tabs, recipe list, detail panel, filter bar. The recipe
  list is a native `KeyboardScroll` pane.
- `character.rs` — `CharacterScreen` (`Ctx::CharacterSheet`); two-column sheet
  with tabbed right pane (Skills | Traits | Effects | Bionics | Proficiencies);
  owns `CharacterSheetState`; the tabbed list is a native `KeyboardScroll` pane.
- `examine.rs` — `ExamineScreen` (`Ctx::ItemExamine`); overlay over inventory
  using the shared `item_detail` widget.
- `item_detail.rs` — Shared `ItemDetailQueries` `SystemParam` bundle +
  `spawn_item_detail` widget. Consumed by `dev_spawn`, `crafting`, `examine`.
- `dev_spawn.rs` — `DevSpawnScreen` (`Ctx::DevSpawnPanel`); debug catalog with
  filter and per-row detail; owns `DevSpawnFocus`; the item list is a native
  `KeyboardScroll` + `VirtualList` pane. Input (navigate/filter/confirm) lives in
  `render/input.rs` (`dev_spawn_input`).
- `dev_worldgen.rs` — `Ctx::DevWorldgen` menu + `Ctx::Gameplay` ASCII
  `Text2d` viewport. Loads `ShareTechMono-Regular.ttf`, drives
  `DevCamera`/`DevPlayer`/`HandCount` for the dev showcase.
- `input.rs` — **Screen adapters (presenter layer):**
  `crafting_menu_input`, `inventory_screen_input`, `dev_pickup_drop_system`,
  `dev_spawn_input`. Translate `InputAction` → `cdda_sim` use-calls + nav
  transitions. Keep new screen-keyboard input here, not in `cdda_sim`.
- `overmap.rs` — `Ctx::Overmap` viewer; `Node { display: Grid }` of
  tile-button cells, `OvermapCamera` pan/zoom, hover info sidebar, z-level
  switching; reads `OvermapGenConfig` + `TerrainQuery` from `cdda_overmap`.
