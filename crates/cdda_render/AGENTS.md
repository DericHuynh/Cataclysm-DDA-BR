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
  `x11`; plus `bevy_ecs`, `bevy_state`, `leafwing-input-manager`, `serde`,
  `serde_json`, `tracing`. **No** tilemap crate (`bevy_fast_tilemap` etc.) —
  tile rendering is plain `bevy_sprite` (`Sprite` + `Image` handles) driven by
  a hand-built `TileRegistry`.
- Per-screen renderers depend on `cdda_context` for the `CddaScreen` trait
  (`Ctx`, `ACTIONS`, `spawn`, `update`) and `cdda_input` for `BindableAction` /
  `ActiveKeybindings`. They never write sim or navigation state directly.

## Local Contracts
- All UI uses Bevy `Node` / `Button` / `Text` — no `bevy_fast_tilemap`, no
  custom shader pipeline. Tile sprites are `bevy_sprite` with
  `Sprite::custom_size` sized from `TileInfo::sprite_size()`.
- **Screen input adapters live here, not in `cdda_sim`.**
  `render/input.rs` holds `crafting_menu_input`, `inventory_screen_input`, and
  `dev_pickup_drop_system`. These read `InputAction` and call `cdda_sim`
  use-case functions. `cdda_sim` never matches `GameAction`.
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

## Verification
- `cargo check -p cdda_render` for compile sanity.
- `cargo nextest run -p cdda_render` (fall back to `cargo test -p cdda_render` if `nextest` is unavailable).
- `cargo run -p cdda_app` for visual smoke validation after render changes.
- `cargo run -p cdda_cli -- schedule-graph` to confirm screen systems land in
  the expected `GameSet` (`Input`, `Sim`, `Render`).

## Child DOX Index

Per-screen files in `src/render/`:

- `mod.rs` — `CddaRenderPlugin`, `UiFontHandle`, `refresh_all_footer_hints`,
  `render_setup` (camera + Inter font), `FooterHint` marker. **Wiring file —
  edit this first** when adding/changing screens.
- `theme.rs` — `UiTheme` resource, `ThemePreset` enum (Blue/Green/Amber),
  fixed colour constants. Single source of truth for palette.
- `tiles.rs` — `TileRegistry`, `TileInfo`, `load_tiles` startup system,
  `tile_info.json` parser, OMT-suffix stripping, sprite manifest ingestion.
- `registry.rs` — `RegistryScreen` (`Ctx::RegistryViewer`); debug viewer over
  the def registry: title, category panel, entry list, detail + raw-JSON +
  parsed-fields panels. **Pane-focus navigation**: `Tab`/`Shift+Tab` cycles the
  focused pane (categories → entries → raw JSON → parsed); `←/→/↑/↓` navigate
  within the focused pane; `PgUp`/`PgDn` page the entries or scroll the detail
  panes. Each def shows a STATUS line reporting the round-trip/coverage check
  result, and the active pane is tinted. Owns `RegistryViewerState`
  (`pane`, `detail_scroll`).
- `main_menu.rs` — `main_menu::spawn` / `sync_focus` for `Ctx::MainMenu`;
  bevy_ui `Node` flex column of `CommandButton`s.
- `settings.rs` — `SettingsScreen` (`Ctx::SettingsMenu`); tabbed settings UI
  (General, Graphics, Sound, Interface, Keybindings) including live key-rebind
  capture. Owns `SettingsState` resource.
- `inventory.rs` — `InventoryScreen` (`Ctx::Inventory`); three-panel pocket /
  wielded / worn layout; reads `ItemTypeRegistry`, `TileRegistry`.
- `crafting.rs` — `CraftingScreen` (`Ctx::CraftingMenu`); recipe browser with
  category tabs, sub-tabs, recipe list, detail panel, filter bar.
- `character.rs` — `CharacterScreen` (`Ctx::CharacterSheet`); two-column sheet
  with tabbed right pane (Skills | Traits | Effects | Bionics | Proficiencies);
  owns `CharacterSheetState`.
- `examine.rs` — `ExamineScreen` (`Ctx::ItemExamine`); overlay over inventory
  using the shared `item_detail` widget.
- `item_detail.rs` — Shared `ItemDetailQueries` `SystemParam` bundle +
  `spawn_item_detail` widget. Consumed by `dev_spawn`, `crafting`, `examine`.
- `dev_spawn.rs` — `DevSpawnScreen` (`Ctx::DevSpawnPanel`); debug catalog with
  filter and per-row detail; owns `DevSpawnFocus`.
- `dev_worldgen.rs` — `Ctx::DevWorldgen` menu + `Ctx::Gameplay` ASCII
  `Text2d` viewport. Loads `ShareTechMono-Regular.ttf`, drives
  `DevCamera`/`DevPlayer`/`HandCount` for the dev showcase.
- `input.rs` — **Screen input adapters (presenter layer):**
  `crafting_menu_input`, `inventory_screen_input`, `dev_pickup_drop_system`.
  Translate `InputAction` → `cdda_sim` use-calls + nav transitions. Keep new
  screen-keyboard input here, not in `cdda_sim`.
- `overmap.rs` — `Ctx::Overmap` viewer; `Node { display: Grid }` of
  tile-button cells, `OvermapCamera` pan/zoom, hover info sidebar, z-level
  switching; reads `OvermapGenConfig` + `TerrainQuery` from `cdda_overmap`.
