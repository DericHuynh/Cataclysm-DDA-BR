//! # cdda_assets — Bevy asset integration for CDDA data definitions.
//!
//! ## Design
//!
//! `CddaDataPack` is a Bevy [`Asset`] wrapping the fully-resolved [`DefRegistry`].
//! A single `.pack` JSON manifest names the data directories to load; the
//! `CddaDataPackLoader` runs the two-pass `cdda_data` loader and produces the
//! asset.
//!
//! ## Hot-reload
//!
//! Register `CddaAssetsPlugin` and load `assets/core.pack` via `AssetServer`.
//! When the manifest or any watched data file changes, Bevy automatically reloads
//! the pack. React to `AssetEvent::<CddaDataPack>::Modified` in your systems.
//!
//! ## Handles to individual definitions
//!
//! Currently the pack is monolithic. Future work can add labeled sub-assets
//! (`load_context.add_labeled_asset("item/rock", ItemDefAsset(...))`)
//! so systems can hold `Handle<ItemDefAsset>` references.

use std::path::PathBuf;
use std::sync::Arc;

use bevy_asset::{Asset, AssetLoader, LoadContext, io::Reader};
use bevy_reflect::{Reflect, TypePath};
use serde::Deserialize;
use tracing::info;

pub use crate::data::registry::DefRegistry;

// ---------------------------------------------------------------------------
// Asset type
// ---------------------------------------------------------------------------

/// The fully-loaded CDDA data pack — all game definitions resolved and
/// ready for ECS entity spawning.
///
/// Access via `Res<Assets<CddaDataPack>>` and a stored `Handle<CddaDataPack>`.
#[derive(Asset, TypePath, Clone)]
pub struct CddaDataPack {
    pub registry: Arc<DefRegistry>,
}

impl CddaDataPack {
    pub fn new(registry: DefRegistry) -> Self {
        Self {
            registry: Arc::new(registry),
        }
    }
}

// ---------------------------------------------------------------------------
// Manifest (the .pack file format)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct PackManifest {
    /// Data directories to load, relative to the current working directory.
    data_dirs: Vec<PathBuf>,
}

// ---------------------------------------------------------------------------
// Loader error
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum PackLoaderError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Data loading failed with {count} error(s): {first}")]
    Load {
        count: usize,
        first: crate::data::loader::LoaderError,
    },
}

// ---------------------------------------------------------------------------
// AssetLoader impl
// ---------------------------------------------------------------------------

/// Loads a `.pack` JSON manifest and runs the two-pass CDDA data loader.
#[derive(Default, Reflect)]
pub struct CddaDataPackLoader;

impl AssetLoader for CddaDataPackLoader {
    type Asset = CddaDataPack;
    type Settings = ();
    type Error = PackLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
    ) -> Result<CddaDataPack, PackLoaderError> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;

        let manifest: PackManifest = serde_json::from_slice(&bytes)?;

        info!(
            "CddaDataPackLoader: loading {} data dir(s)",
            manifest.data_dirs.len()
        );

        let mut loader = crate::data::loader::Loader::new(manifest.data_dirs);
        loader.ingest_all();
        let registry = loader.load().map_err(|mut errs| {
            let first = errs.remove(0);
            PackLoaderError::Load {
                count: errs.len() + 1,
                first,
            }
        })?;

        info!(
            "CddaDataPackLoader: loaded {} definitions",
            registry.total_count()
        );

        Ok(CddaDataPack::new(registry))
    }

    fn extensions(&self) -> &[&str] {
        &["pack"]
    }
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

use bevy_app::{App, Plugin};
use bevy_asset::AssetApp;

/// Registers [`CddaDataPack`] as a Bevy asset and wires up the loader.
///
/// Add this plugin early (before systems that read the pack). Then load the
/// manifest via `AssetServer::load("core.pack")` and store the resulting
/// `Handle<CddaDataPack>` as a resource.
pub struct CddaAssetsPlugin;

impl Plugin for CddaAssetsPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<CddaDataPack>();
        app.init_asset_loader::<CddaDataPackLoader>();
    }
}
