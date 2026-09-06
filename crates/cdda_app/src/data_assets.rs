//! # Asset-driven CDDA data loading + hot reload.
//!
//! The legacy disk path ([`crate::startup::load_data_system`]) walks `data_dirs`
//! with `std::fs`, ingests JSON, resolves, and builds the definition-world.
//! This module does the same thing but sources the file **bytes** through
//! Bevy's asset server, loading one [`cdda_data::json_asset::CddaJsonFile`] per
//! on-disk `.json` file. Because each file is a real Bevy asset behind a
//! `FileAssetReader`, the asset server watches it and re-emits it on change —
//! so only the data roots actually loaded drive reloads, and a changed file
//! rebuilds the whole registry + definition-world in place.
//!
//! ## Asset source
//!
//! CDDA JSON lives outside the standard `assets/` root, so [`run`](crate::run)
//! registers a named `"cdda"` asset source rooted at the repo `data` directory
//! *before* `DefaultPlugins` builds the `AssetPlugin` (asset sources are built
//! there and not after). A file discovered as `data/core/items/weapons.json`
//! therefore becomes the asset path `cdda://core/items/weapons.json`.
//! Enumeration still uses `std::fs` (Bevy has no directory-listing on
//! `LoadContext` in 0.18); only the *reading* of bytes goes through the asset
//! server so files are watched.
//!
//! ## Hot reload
//!
//! [`reload_modified_data`] runs every `Update` while the app is `InGame`. It
//! keeps a persistent [`MessageCursor`] over [`AssetEvent`]`::<CddaJsonFile>`
//! (skipping `Added`, reacting to `Modified` / `LoadedWithDependencies`), then
//! re-ingests every in-use file from the `Assets<CddaJsonFile>` store into a
//! fresh [`Loader`], resolves it, and calls
//! [`crate::startup::apply_registry_to_world`] — the exact tail the startup
//! path uses — so the running game reflects data edits without restarting.

use crate::startup::apply_registry_to_world;
use cdda_data::json_asset::CddaJsonFile;
use cdda_data::Loader;
use cdda_sim::runtime::state::StartupConfig;
use std::path::{Path, PathBuf};

use bevy::asset::io::AssetSourceBuilder;
use bevy::asset::Handle;
use bevy::asset::{AssetApp, AssetEvent, AssetPath, AssetServer, Assets};
use bevy_ecs::message::{MessageCursor, Messages};
use bevy_ecs::prelude::*;

// ---------------------------------------------------------------------------
// Asset source registration
// ---------------------------------------------------------------------------

pub const CDDA_ASSET_SOURCE: &str = "cdda";

/// The `"cdda"` asset source root. `AssetSourceBuilder::platform_default`
/// resolves paths relative to the Bevy base path (`CARGO_MANIFEST_DIR` for the
/// `cdda_app` crate at runtime), so `../../data` lands on the repo `data/` dir.
pub const CDDA_ASSET_SOURCE_REL: &str = "../../data";

/// Absolute path of the asset root (`repo/data/`) that `CDDA_ASSET_SOURCE_REL`
/// resolves to, mirroring `bevy_asset::io::get_base_path`.
pub fn data_root_abs() -> PathBuf {
    let base = if let Ok(dir) = std::env::var("BEVY_ASSET_ROOT") {
        PathBuf::from(dir)
    } else if let Ok(dir) = std::env::var("CARGO_MANIFEST_DIR") {
        PathBuf::from(dir)
    } else {
        std::env::current_exe()
            .map(|p| p.parent().map(Path::to_path_buf).unwrap_or_default())
            .unwrap_or_default()
    };
    base.join(CDDA_ASSET_SOURCE_REL)
}

/// Registers the `"cdda"` source. **Must** run before `DefaultPlugins` (which
/// adds `AssetPlugin`), since asset sources are built at `AssetPlugin` startup.
pub fn register_cdda_asset_source(app: &mut bevy::app::App) -> &mut bevy::app::App {
    app.register_asset_source(
        CDDA_ASSET_SOURCE,
        AssetSourceBuilder::platform_default(CDDA_ASSET_SOURCE_REL, None),
    )
}

// ---------------------------------------------------------------------------
// Resource: discovered files + strong handles
// ---------------------------------------------------------------------------

/// Records which CDDA data files the asset pipeline is watching/loading.
#[derive(Resource, Default)]
pub struct CddaDataFiles {
    /// The on-disk absolute path of each in-use `.json` file, in load order.
    pub absolute_paths: Vec<PathBuf>,
    /// Strong handles keep every in-use `CddaJsonFile` alive (and watched).
    pub handles: Vec<Handle<CddaJsonFile>>,
}

// ---------------------------------------------------------------------------
// Entry creation -> reload-only; initial build stays in startup.rs
// ---------------------------------------------------------------------------

/// Discovers every `.json` under each `StartupConfig::data_dirs`, loads each
/// through the `"cdda"` asset source as a [`CddaJsonFile`], and records the
/// absolute paths + strong handles so [`reload_modified_data`] can rebuild on
/// change. Runs from `OnEnter(AppState::DataLoading)`; the initial definition
/// build still happens in `startup.rs::load_data_system`.
pub fn request_data_files(
    asset_server: Res<AssetServer>,
    startup: Res<StartupConfig>,
    mut files: ResMut<CddaDataFiles>,
) {
    let root = data_root_abs();
    let mut absolute_files: Vec<PathBuf> = Vec::new();

    for dir in &startup.data_dirs {
        let dir_abs = if dir.is_absolute() {
            dir.clone()
        } else {
            // Relative data dirs (default `data/core`) resolve against the CWD
            // (repo root when run via `cargo run`), i.e. the parent of `data/`.
            root.parent().unwrap_or(Path::new(".")).join(dir)
        };
        discover_json_files(&dir_abs, &root, &mut absolute_files);
    }

    let mut handles: Vec<Handle<CddaJsonFile>> = Vec::new();
    for abs in &absolute_files {
        let rel = abs
            .strip_prefix(&root)
            .unwrap_or(abs)
            .to_string_lossy()
            .into_owned();
        // Source-qualify with our owned, `'static` source name so the loader
        // resolves through the `cdda` asset source.
        let asset_path = AssetPath::from_path(Path::new(&rel))
            .into_owned()
            .with_source(CDDA_ASSET_SOURCE);
        let handle: Handle<CddaJsonFile> = asset_server.load(asset_path);
        handles.push(handle);
    }

    files.absolute_paths = absolute_files;
    files.handles = handles;
}

/// Recursively collect `.json` files (excluding CDDA mod metadata) into `out`,
/// as absolute paths, mirroring `Loader::ingest_directory`'s selection rules.
fn discover_json_files(dir: &Path, root: &Path, out: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) => {
            tracing::warn!("cdda asset discovery: cannot read {:?}: {e}", dir);
            return;
        }
    };
    for entry in entries {
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        if path.is_dir() {
            discover_json_files(&path, root, out);
        } else if path.extension().map_or(false, |ext| ext == "json") {
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if file_name == "modinfo.json" || file_name == "mod_tileset.json" {
                continue;
            }
            out.push(path);
        }
    }
}

// ---------------------------------------------------------------------------
// Hot reload
// ---------------------------------------------------------------------------

/// Rebuilds the definition-world whenever any in-use CDDA data file changes.
///
/// This is an exclusive system (`&mut World`) so it can reuse the exact
/// "build everything" path ([`apply_registry_to_world`]) without introducing
/// ECS borrow conflicts. A persistent [`MessageCursor`] (via [`Local`]) tracks
/// [`AssetEvent`]::`<CdJsonFile>` streams; `Added` (initial load) is skipped,
/// and `Modified` / `LoadedWithDependencies` trigger a full re-ingest + resolve
/// + rebuild of only the data roots actually loaded.
pub fn reload_modified_data(
    world: &mut World,
    mut cursor: Local<MessageCursor<AssetEvent<CddaJsonFile>>>,
) {
    let messages = match world.get_resource::<Messages<AssetEvent<CddaJsonFile>>>() {
        Some(m) => m,
        None => {
            tracing::debug!("CDDA asset event channel not yet initialized");
            return;
        }
    };

    let mut changed = false;
    for event in cursor.read(messages) {
        match event {
            AssetEvent::Added { .. } => {}
            AssetEvent::Modified { .. }
            | AssetEvent::LoadedWithDependencies { .. }
            | AssetEvent::Removed { .. }
            | AssetEvent::Unused { .. } => changed = true,
        }
    }
    // Drop the borrow on `messages` (hence `world`) before the mutable pass.
    if !changed {
        return;
    }

    // Gather every in-use file's parsed values while holding shared borrows.
    let injected: Vec<(PathBuf, Vec<serde_json::Value>)> = {
        let files = match world.get_resource::<CddaDataFiles>() {
            Some(f) => f,
            None => {
                tracing::warn!("reload requested but no CddaDataFiles resource");
                return;
            }
        };
        let assets = match world.get_resource::<Assets<CddaJsonFile>>() {
            Some(a) => a,
            None => {
                tracing::warn!("reload requested but no Assets<CddaJsonFile> resource");
                return;
            }
        };

        let mut out: Vec<(PathBuf, Vec<serde_json::Value>)> = Vec::new();
        for (abs_path, handle) in files.absolute_paths.iter().zip(&files.handles) {
            match assets.get(handle.id()) {
                Some(file) => out.push((abs_path.clone(), file.values.clone())),
                None => tracing::warn!(
                    "CDDA data file not yet parsed by the asset server: {:?}",
                    abs_path
                ),
            }
        }
        out
    };

    tracing::info!(
        "CDDA data changed — rebuilding definitions from {} files",
        injected.len()
    );

    let mut loader = Loader::new(Vec::new());
    loader.ingest_values(injected);
    match loader.resolve() {
        Ok(registry) => {
            let count = registry.total_count();
            if apply_registry_to_world(world, &registry, count) {
                tracing::info!("CDDA hot-reload complete: {} resolved definitions", count);
            } else {
                tracing::warn!("CDDA hot-reload rejected; keeping previous definitions active");
            }
        }
        Err(errors) => {
            for err in &errors {
                tracing::warn!("CDDA reload error: {err:?}");
            }
            tracing::warn!(
                "CDDA reload finished with {} errors; keeping previous definitions active",
                errors.len()
            );
        }
    }
}
