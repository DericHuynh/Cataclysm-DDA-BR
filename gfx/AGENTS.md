# Graphics DOX

## Purpose
Owns render assets, tilesets, and loading screens consumed by `cdda_render` and the Bevy asset pipeline.

## Ownership
- The renderer reads from this folder through the `gfx/` symlink at `crates/cdda_app/assets/gfx`.
- Tileset layout, fallback images, and tile config JSONs belong here.
- Loading screens are a separate bundle picked up at app startup.

## Local Contracts
- Tileset directories contain a `tile_config.json` (or `tile_info.json`), a `tileset.txt`, and one or more PNG atlases. New tilesets must follow the same structure or `cdda_render` will fail to register them.
- `gfx/tile_config_template.json` is the template for new tilesets — copy it when adding one.
- `gfx/loading_screens/` is loaded on app startup; new loading screens just drop a PNG with the same naming convention.

## Work Guidance
- Do not hand-edit generated atlases. Regenerate tileset atlases with the upstream toolchain if a tileset change needs new sprites.
- When adding a new tileset, register it in `cdda_app` startup and the tileset selector UI in `cdda_render`.

## Verification
- `cargo run -p cdda_app` boots the renderer and exercises the asset pipeline end-to-end. No automated visual-diff check exists yet.

## Child DOX Index
- `gfx/ASCIITileset/` — ASCII fallback tileset. Always loaded.
- `gfx/UltimateCataclysm/` — Main high-resolution tileset bundle (multiple PNG atlases organized by sprite size category).
- `gfx/loading_screens/` — Loading screen art shown at startup.
