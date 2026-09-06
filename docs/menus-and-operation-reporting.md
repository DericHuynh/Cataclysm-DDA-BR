# Menus, settings and operation reporting

## Delivered foundation

- Main menu uses the original newspaper illustration at
  `gfx/loading_screens/loading_img_01.png`. Loading uses `loading_img_hub.png`.
  Assets remain in gfx and load through the existing assets/gfx symlink.
- One retained command-menu frame covers the main/new-game/world menus,
  character setup/confirmation, Help, Credits, bulletin, load-game and pause
  screens. Keyboard commands and mouse selection use the same navigation path.
  Back buttons and explanatory text replace blank screens. Unimplemented
  workflows are identified explicitly; this does not implement character/world
  editing or saved-game loading.
- Settings keeps the shared virtual row pool and offers functional interface
  scale (70–150%), fullscreen, menu artwork and color presets. Key rebinding
  remains the existing session editor. General/audio and unfinished interface
  options are labelled unavailable rather than showing invented active values.
- Display preferences persist to `config/interface.json`, or
  `$CDDA_CONFIG_DIR/interface.json`. Transient focus/rebinding state is excluded.
  Writes use a temporary file and rename; unreadable/malformed files are preserved
  and changes remain session-only. Errors use the shared visual/terminal report.
- Quit requests Bevy AppExit instead of terminating the process inside navigation.
- The pause menu gates simulation and restores the previous pause value on exit.

## Illustrated screen composition and motion

The command shell uses a constrained text column beside aspect-preserving original
artwork. Every UI view shares the selected Blue, Green or Amber palette;
the common black canvas matches the baked source images. Narrow logical viewports hide
the decorative menu art and center the commands. Disabling artwork also centers
the controls. Command rows remain scrollable and retain keyboard selection reveal.
Menu hints have a dedicated text owner, so gameplay's “close” hint cannot overwrite them.

Loading reserves nonoverlapping regions for the full Hub artwork and a compact
status area. Only diagnostics scroll; the 2px progress track and retry/return controls
stay fixed. Known totals ease toward reported stage progress, and unknown totals use
a moving segment labelled as indeterminate. A failed operation stops this animation.
These are stage measurements, not estimated whole-operation completion.

`render/cinematic.rs` provides ECS components for artwork fades and smooth
selection/hover accents. Controls remain stationary; there are no sliding entrance
animations. Artwork reveal components remove themselves when settled. Colors animate
without rebuilding text or changing the simulation clock. Geometry responds to window size and UI scale, updating Nodes only
when dimensions change. The development World Inspector is restricted to gameplay.

Capture production screens without an OS window (a working graphics adapter is needed):

```sh
cargo run -p cdda_app --example menu_capture --offline -- menu /tmp/menu.png 1600 900 100
cargo run -p cdda_app --example menu_capture --offline -- loading /tmp/loading.png 1600 900 100
cargo run -p cdda_app --example menu_capture --offline -- error /tmp/error.png 1280 720 150
```

The numeric arguments are physical width, height and UI-scale percent. An optional
final theme index selects Blue=0, Green=1 or Amber=2; `settings` captures the theme
changer and `menu-last` captures last-command selection reveal. This fixture
renders actual production screen systems, bundled images and font, with sample report
data; it does not benchmark the real loader. Main-thread publication/worldgen can still
interrupt animation within a stage (see remaining work below).

## Unified theme contract

Settings → Interface → Theme is the first row. Left/right or Confirm cycles the
palette, and the existing display-preference adapter persists it. One scheduled writer
applies SettingsState to UiTheme. Themes cover command menus (including developer
worldgen), loading/errors, Settings, crafting, character, inventory/examine/item details,
registry/spawn tools, overmap chrome and the gameplay status text. Source artwork,
terrain colors and semantic danger/warning signals keep their meaning.

Static UI entities retain `TextPaint`, `SurfacePaint` and `BorderPaint` roles. A shared
PostUpdate system repaints new/changed roles or a changed theme, without touching
labels or geometry. Virtual row presenters resolve the same roles through UiTheme;
character row caches store roles so changing a theme cannot leave cached text colors
behind. Animated button fills have a single owner and use the same selection palette.

## Reporting contract

`cdda_core_types::progress::ReportEvent` is a presentation-independent record:
level, stage, message and optional completed/total units. Unknown totals are
indeterminate; there is no estimated overall percentage. Terminal consumers use
its Display representation on stderr. `cdda_components::progress::OperationReport`
is the ECS read model, with aggregate warning/error counts and a bounded 128-record
history; the screen shows the latest six retained diagnostics. The terminal keeps
all consumed records. OperationCommand carries Retry/ReturnToMenu requests.

The loader's `load_reported` discovers files in stable directory order, parses each
file once, ingests definitions, then reports each category's inheritance resolution
and typed conversion. File-read/JSON errors stop loading. Omitted definitions and
unhandled categories are warnings under the existing broad compatibility-loader
policy; these do not imply strict native content support. Empty startup results
are rejected. Tooling can still inspect an empty partial registry.

The app runs disk parsing/resolution on a worker with a bounded message queue.
It consumes up to 64 reports per frame, including all fatal diagnostics, then
separates terrain validation, ECS definition creation, and registry publication
across frames. Each stage is announced before its main-thread work. Worldgen
reports start/completion; missing definitions/terrain stop it. Failed operations
retain diagnostics and provide retry/return controls. Ctx::Loading isolates normal
menu input. Cancellation discards queued publication; the background operation
may finish its current computation, but its dropped channel cannot publish results.

The same records drive loading text, stage progress, failure controls, general
warning/error notices and terminal output. CLI load/check/validation commands
consume the loader protocol directly. The old string-only LoadingStatus is removed.

## Remaining menu work

1. Implement character/scenario/profession and world/mod selection against a
   validated pre-game catalog; replace the explicit unavailable screens.
2. Persist keybindings with conflict validation and restore-default controls.
3. Add audio only with real playback support, then expose volume controls.
4. Implement saved-world selection, session teardown and new-session lifecycle.
5. Make ECS definition publication and world generation incrementally budgeted;
   they currently yield between stages but each stage can still block a frame.
   Asset-watcher discovery also remains synchronous.
6. Add whole-operation timing, exported diagnostic reports, responsive layout
   screenshot baselines and richer stage reporting inside world generation.

## Verification

`cargo nextest run -p cdda_render -p cdda_app --offline` covers production headless
presenters, loader worker/cancellation, preference persistence, pause restoration,
loading diagnostics, native scaled layout and retained virtual lists. The unified-theme
revision adds live repaint checks for every shared command context and loading,
retained crafting/spawn labels across all three palettes, keyboard/Confirm theme
selection, idle paint ticks and stationary button transforms. Workspace/all-target
compilation and whitespace/format checks also run.

Offscreen GPU captures use the production screen and palette systems, bundled artwork
and shared font. They validate rendered composition, including the theme changer;
fixture reports do not constitute an end-to-end real-loader benchmark. Earlier native
layout checks cover 70–150% scaling, last-command selection reveal and diagnostics that
cannot move the fixed progress track. Large synchronous publication/worldgen stages
remain the animation limitation described above.
