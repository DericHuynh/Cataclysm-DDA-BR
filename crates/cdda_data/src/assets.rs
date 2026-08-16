//! # cdda_assets — Bevy asset integration for CDDA data definitions.
//!
//! ## Design
//!
//! CDDA JSON lives outside Bevy's default `assets/` root, so it is exposed to
//! the asset pipeline as individual [`crate::json_asset::CddaJsonFile`] assets.
//! `cdda_app` loads one [`crate::json_asset::CddaJsonFile`] per on-disk `.json`
//! (for the data roots it actually uses — core plus any active mods), which
//! makes each file a watched, hot-reloadable Bevy asset. A single resolved
//! snapshot ([`CddaDataPack`]) wraps the composed [`DefRegistry`] for runtime
//! consumers.
//!
//! ## Hot-reload
//!
//! [`crate::json_asset::CddaJsonFile`] is a normal Bevy asset, so with the
//! `file_watcher` feature enabled the asset server re-emits it whenever its
//! source file changes. React to `AssetEvent<CddaJsonFile>` in `cdda_app` and
//! re-run ingest → resolve → def-world. Only the files actually loaded drive
//! reloads; untouched data roots are not watched.

use bevy_asset::Asset;
use bevy_reflect::TypePath;

/// The fully-loaded CDDA data pack — composed [`DefRegistry`] snapshot.
///
/// Kept small and immutable so consumers (registry viewer, def-world builder)
/// can clone the `Arc` cheaply across reloads.
#[derive(Asset, TypePath, Clone)]
pub struct CddaDataPack {
    /// The resolved registry for the currently-loaded data set.
    pub registry: std::sync::Arc<crate::registry::DefRegistry>,
}

impl CddaDataPack {
    pub fn new(registry: crate::registry::DefRegistry) -> Self {
        Self {
            registry: std::sync::Arc::new(registry),
        }
    }
}

use bevy_app::{App, Plugin};
use bevy_asset::AssetApp;

/// Registers [`CddaJsonFile`] and the resolved [`CddaDataPack`] as Bevy assets
/// plus their loaders.
///
/// Add this plugin before systems that read the data. Data-root watching is
/// configured by the app (the JSON files must be reachable from an asset
/// source; see `cdda_app`).
pub struct CddaAssetsPlugin;

impl Plugin for CddaAssetsPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<crate::json_asset::CddaJsonFile>();
        app.init_asset_loader::<crate::json_asset::CddaJsonFileLoader>();
        app.init_asset::<CddaDataPack>();
    }
}
