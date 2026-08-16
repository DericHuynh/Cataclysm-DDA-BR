//! # CddaJsonFile — one CDDA `.json` data file as a Bevy asset.
//!
//! This mirrors the `bevy_htn`/`bevy_bae` asset pattern: a single on-disk source
//! file is wrapped in a tiny [`Asset`] so the Bevy asset server watches it and
//! hot-reloads it when the bytes change. Here the source is a CDDA data file
//! (a JSON array of definition objects), parsed into raw `serde_json::Value`s.
//!
//! # Why a dedicated per-file asset
//!
//! `CddaDataPack` (see [`crate::assets`]) is the *composed* result of many JSON
//! files, but to hot-reload "only the files we're actually using" Bevy must
//! watch the individual files — not just the composed pack. That requires
//! either real loader dependencies ([`LoadContext::read_asset_bytes`]) or one
//! asset per file. This type is the per-file asset; the pack loader opts into
//! either mechanism depending on the configured data root.

use bevy_asset::{io::Reader, Asset, AssetLoader, LoadContext};
use bevy_reflect::TypePath;
use serde_json::Value;

// ---------------------------------------------------------------------------
// Asset type
// ---------------------------------------------------------------------------

/// One CDDA JSON data file, parsed into its raw top-level array/object values.
///
/// `source` is the stable string the asset was loaded from (relative to the
/// configured data root), so a downstream loader can attribute errors to the
/// exact file.
#[derive(Asset, TypePath, Clone, Debug)]
pub struct CddaJsonFile {
    /// The raw top-level JSON values (usually an array of def objects).
    pub values: Vec<Value>,
    /// The asset path this file was loaded from (for diagnostics).
    pub source: String,
}

// ---------------------------------------------------------------------------
// Loader
// ---------------------------------------------------------------------------

/// Reads a `.json` CDDA data file (array or single object) into [`CddaJsonFile`].
#[derive(Default, bevy_reflect::TypePath)]
pub struct CddaJsonFileLoader;

impl AssetLoader for CddaJsonFileLoader {
    type Asset = CddaJsonFile;
    type Settings = ();
    type Error = CddaJsonFileError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
    ) -> Result<CddaJsonFile, CddaJsonFileError> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let value: Value = serde_json::from_slice(&bytes)?;

        let values = match value {
            Value::Array(items) => items,
            // A single def object (unusual but tolerated by the existing loader).
            obj @ Value::Object(_) => vec![obj],
            other => {
                return Err(CddaJsonFileError::NotArray {
                    found: describe_value(&other),
                });
            }
        };

        Ok(CddaJsonFile {
            values,
            source: _load_context.path().path().display().to_string(),
        })
    }

    fn extensions(&self) -> &[&str] {
        &["json"]
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors produced while reading a [`CddaJsonFile`] asset.
#[derive(Debug, thiserror::Error)]
pub enum CddaJsonFileError {
    /// The underlying I/O read failed.
    #[error("IO error reading CDDA JSON: {0}")]
    Io(#[from] std::io::Error),

    /// The file did not parse as JSON.
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    /// The JSON root was neither an array nor a single object.
    #[error("CDDA data root must be an array or object, found {found}")]
    NotArray {
        /// A description of the encountered value.
        found: String,
    },
}

/// Short `{"type": "..."}`-style descriptor for a non-array JSON value.
fn describe_value(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => format!("bool({b})"),
        Value::Number(n) => format!("number({n})"),
        Value::String(s) => format!("string({s:?})"),
        Value::Array(_) => "array".to_string(),
        Value::Object(o) => format!(
            "object(with {} keys)",
            o.keys().take(2).cloned().collect::<Vec<_>>().join(", ")
        ),
    }
}
