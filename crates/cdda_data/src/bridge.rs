//! # Part B — decoupled import/export bridge over Bevy components
//!
//! The data pipeline treats the **fully-resolved raw JSON** (the output of
//! [`Loader::resolve_type_raw`](crate::loader::Loader::resolve_type_raw)) as the
//! lossless source of truth. Typed def structs are a *projection* of that JSON,
//! not an independent store. Nothing may be dropped when moving between the
//! raw and typed representations.
//!
//! This module provides the import/export **adapters** that a GUI editor (or
//! any in-session tooling) builds on top of, with serialization fully decoupled
//! from deserialization so we can import, mutate the in-memory form, then export
//! in a new format without coupling the two directions:
//!
//! - **Import (raw → typed):** [`DefRecord`] — a Bevy `Component` (one per def
//!   instance) that holds both the *raw resolved JSON* (the source of truth) and
//!   the parsed *typed def*. Because it retains the raw value, nothing the
//!   structs do not model is ever lost, and edits can be applied against the raw
//!   canonical value rather than a re-serialized lossy projection.
//! - **Export (typed/edited → raw):** [`compute_overrides`] answers the question
//!   "what is the minimal raw JSON that reproduces this edited def given its
//!   `copy-from` parent's resolved value?" It is the inverse of the resolver:
//!   fields equal to the parent default are omitted (no override written for
//!   inherited defaults), fields absent from the parent are added, fields
//!   explicitly cleared that the parent had are emitted as a `delete`, and
//!   changed fields override. Re-resolving the produced object must reproduce
//!   the target value (`export_override_def` + re-resolve is the regression
//!   test).
//!
//! ## The raw defs are the source of truth
//!
//! ```text
//!                    ┌───────────────────────────┐
//!   JSON (copy-from) │   resolve_copy_from       ├─→ resolved raw JSON  ──┐
//!                    └───────────────────────────┘                         │
//!                                                                          ▼
//!                                                 ┌──────────────────────────────────┐
//!                                                 │  Import: resolved → DefRecord<T> │
//!                                                 │  (Bevy component: raw + parsed)  │
//!                                                 └──────────────────────────────────┘
//!                                                                          │  GUI / tooling edits
//!                                                                          ▼
//!                                                 ┌──────────────────────────────────┐
//!                                                 │  Export: compute_overrides(…)    │
//!                                                 │  → minimal copy-from delta       │
//!                                                 └──────────────────────────────────┘
//!                                                                          │
//!   JSON (rewritten) ←─────────────────────────────────────────────────────┘
//! ```
//!
//! Import and export are separate [`ImportConfig`]/[`ExportConfig`] so a new
//! wire format (v1, v2, a mod-pack delta) only needs a new adapter, never a
//! rewrite of the other direction.

use bevy_ecs::component::Component;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

use crate::loader::Loader;

/// The canonical control/operation keys CDDA resolution strips and the diff
/// engine must never emit as ordinary overrides (they are structural, not data).
const CONTROL_KEYS: &[&str] = &[
    "type",
    "id",
    "abstract",
    "abstract_",
    "copy-from",
    "copy_from",
    "extend",
    "delete",
    "relative",
    "proportional",
];

// ===========================================================================
// Import adapter
// ===========================================================================

/// A single parsed + raw definition, carried as a Bevy component during the
/// import side of the bridge.
///
/// Two forms of the same definition coexist so both directions are cheap:
/// - [`raw`](Self::raw) — the *source of truth* resolved JSON (never dropped).
/// - [`def`](Self::def) — the typed projection used by gameplay code.
#[derive(Component, Debug, Clone)]
pub struct DefRecord<T> {
    /// Stable string ID of the definition (its resolved `id`).
    pub id: String,
    /// The resolved raw JSON — authoritative.
    pub raw: Value,
    /// The typed / projected definition.
    pub def: T,
}

/// Errors from attempting to import a resolved JSON value into a typed def.
#[derive(Debug, Clone)]
pub struct ImportError {
    pub id: String,
    pub kind: ImportErrorKind,
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "import of '{}' failed: {}", self.id, self.kind)
    }
}

impl std::error::Error for ImportError {}

#[derive(Debug, Clone)]
pub enum ImportErrorKind {
    /// `serde_json::from_value` rejected the resolved JSON.
    Parse(String),
}

impl std::fmt::Display for ImportErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImportErrorKind::Parse(e) => write!(f, "typed parse error: {e}"),
        }
    }
}

impl std::error::Error for ImportErrorKind {}

/// Configuration for the import direction (placeholder today; exists so the
/// import path has a stable seam to add format flags without a breaking change).
#[derive(Debug, Clone, Default)]
pub struct ImportConfig {
    /// When set, unknown (unmodeled) JSON keys are retained in `DefRecord::raw`
    /// (they always are) but also surfaced in a side table keyed by path. Kept
    /// for forwards-compat; the raw value is never dropped regardless.
    pub track_unmodeled: bool,
}

/// Convenience default import config.
pub fn import_default_config() -> ImportConfig {
    ImportConfig::default()
}

/// Parse one resolved JSON value into a typed def, wrapping it (with its raw
/// source) in a [`DefRecord`]. Only the *typed* projection can fail; the raw is
/// always retained, so a failed parse simply short-circuits the record.
pub fn import_def<T>(
    id: &str,
    raw: &Value,
    _config: &ImportConfig,
) -> Result<DefRecord<T>, ImportError>
where
    T: DeserializeOwned,
{
    let def: T = serde_json::from_value(raw.clone()).map_err(|e| ImportError {
        id: id.to_string(),
        kind: ImportErrorKind::Parse(e.to_string()),
    })?;
    Ok(DefRecord {
        id: id.to_string(),
        raw: raw.clone(),
        def,
    })
}

// ===========================================================================
// Export adapter
// ===========================================================================

/// A minimal override delta: the set of changes needed to go from a `copy-from`
/// parent's resolved value to a child's resolved value, expressed in CDDA's
/// override terms (`fields` plus a `delete` set).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OverrideDelta {
    /// Keys whose final value must be (re)written on the child definition.
    /// These are only the values that **differ** from the inherited parent —
    /// inherited values are absent here so they are never redundantly written.
    pub fields: Map<String, Value>,
    /// Top-level keys the parent carries that the child explicitly **removes**.
    /// Expressed as CDDA `delete`.
    pub delete: Vec<String>,
}

impl OverrideDelta {
    /// True when the child adds no overrides and removes nothing — i.e. the
    /// child is (so far) exactly its parent, so it can be written with just
    /// `copy-from` and a new `id`.
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty() && self.delete.is_empty()
    }
}

/// Returns true when two JSON values are structurally equal. Equal values are
/// inherited from the parent and therefore need no override.
fn json_equal(a: &Value, b: &Value) -> bool {
    a == b
}

/// Compute the minimal override delta a child's resolved value needs to be
/// expressed *on top of* a parent's resolved value.
///
/// Rules (matching "use copy-from to get defaults" workflow):
/// - A field equal to the parent's → **omitted** (no redundant override, it
///   inherits the default silently).
/// - A field the parent lacks → **added** as an override.
/// - A field present in the parent but **absent** in the child → emitted as a
///   `delete` entry (so `apply_delete` removes the inherited value).
/// - A changed scalar or whole-array → override with the new value.
/// - Nested objects that differ are **replaced wholesale** (an override object),
///   never structurally diffed — CDDA patch semantics merge at one level only,
///   and emitting a partial nested object would silently drop its sibling keys.
pub fn compute_overrides(parent: &Value, child: &Value) -> OverrideDelta {
    let mut delta = OverrideDelta::default();
    let (Some(child_obj), Some(parent_obj)) = (child.as_object(), parent.as_object()) else {
        return delta;
    };

    // Keys to consider: union of both (excluding structural control keys).
    let mut keys: Vec<&str> = child_obj.keys().map(String::as_str).collect();
    for k in parent_obj.keys() {
        if !child_obj.contains_key(k) {
            keys.push(k);
        }
    }
    keys.retain(|k| !CONTROL_KEYS.contains(k));

    for key in keys {
        let cv = child_obj.get(key);
        let pv = parent_obj.get(key);
        match (cv, pv) {
            (Some(c), Some(p)) => {
                if !json_equal(c, p) {
                    delta.fields.insert(key.to_string(), c.clone());
                }
            }
            (Some(c), None) => {
                delta.fields.insert(key.to_string(), c.clone());
            }
            (None, Some(_)) => {
                delta.delete.push(key.to_string());
            }
            (None, None) => unreachable!("key came from the union of both objects"),
        }
    }
    delta
}

/// Export config — placeholder seam for future formats (e.g. whether to emit
/// `relative`/`proportional` instead of absolute values). Kept separate from
/// [`ImportConfig`] to preserve decoupling.
#[derive(Debug, Clone, Default)]
pub struct ExportConfig {
    /// When true, prefer emitting CDDA `relative` deltas for numeric ordinary
    /// fields instead of absolute values (a typical modding nicety). Default
    /// off; absolute values are always correct for round-tripping.
    pub prefer_relative: bool,
}

/// Build the full override JSON object that should be written to disk to
/// express a child def on top of `parent_id` via `copy-from`, given the delta
/// from [`compute_overrides`]. Re-resolving this object (with the same raw map)
/// reproduces the child's resolved value.
pub fn export_override_def(
    json_type: &str,
    id: &str,
    parent_id: &str,
    delta: &OverrideDelta,
) -> Value {
    let mut obj = Map::new();
    obj.insert("type".to_string(), Value::String(json_type.to_string()));
    obj.insert("id".to_string(), Value::String(id.to_string()));
    obj.insert(
        "copy-from".to_string(),
        Value::String(parent_id.to_string()),
    );
    for (k, v) in &delta.fields {
        obj.insert(k.clone(), v.clone());
    }
    if !delta.delete.is_empty() {
        let mut del = Map::new();
        for k in &delta.delete {
            del.insert(k.clone(), Value::Null);
        }
        obj.insert("delete".to_string(), Value::Object(del));
    }
    Value::Object(obj)
}

/// Convenience wrapper over [`compute_overrides`] that returns the delta plus
/// the full override JSON object via [`export_override_def`].
pub fn export_overrides(
    json_type: &str,
    id: &str,
    parent_id: &str,
    parent: &Value,
    child: &Value,
) -> (OverrideDelta, Value) {
    let delta = compute_overrides(parent, child);
    let obj = export_override_def(json_type, id, parent_id, &delta);
    (delta, obj)
}

/// Re-apply a delta produced by [`compute_overrides`] on top of the parent's
/// resolved value, reproducing the child's resolved value. This mirrors the
/// resolver's `apply_definition` + `apply_delete` semantics (fields replaced /
/// added; `delete` removed) and is the export direction's re-resolution step.
pub fn apply_delta(parent: &Value, delta: &OverrideDelta) -> Value {
    let mut out = parent.clone();
    if let Some(out_obj) = out.as_object_mut() {
        for (k, v) in &delta.fields {
            out_obj.insert(k.clone(), v.clone());
        }
        for k in &delta.delete {
            out_obj.remove(k);
        }
    }
    out
}

// ===========================================================================
// End-to-end bridge check over a loader (data-driven)
// ===========================================================================

/// Per-category result of the Part-B export round-trip over real data.
#[derive(Debug, Clone, Copy, Default)]
pub struct BridgeSummary {
    /// A def with a `copy-from` parent that verified round-trips through the
    /// export adapter (delta applied to the parent == child's resolved data).
    pub ok: usize,
    /// A def whose copy-from parent was missing/in-category-unresolvable — skipped.
    pub skipped: usize,
    /// A def whose export re-resolution did **not** reproduce the child.
    pub mismatches: usize,
    /// The JSON type key this summary is for.
    pub json_type: &'static str,
    /// The category display name.
    pub category: &'static str,
}

impl BridgeSummary {
    pub fn all_ok(&self) -> bool {
        self.mismatches == 0
    }
}

fn strip_control(value: &Value) -> serde_json::Map<String, Value> {
    let mut map = serde_json::Map::new();
    if let Some(obj) = value.as_object() {
        for (k, v) in obj {
            if !CONTROL_KEYS.contains(&k.as_str()) {
                map.insert(k.clone(), v.clone());
            }
        }
    }
    map
}

/// Mapper bound to `for_each_raw_def_kind!(list ...)`.
#[allow(unused)]
macro_rules! bridge_category {
    ($name:ident, $ty:ty, $json:expr, $_field:ident, $_strategy:expr) => {
        ($json, stringify!($name))
    };
}

/// Verify the export adapter is lossless across every def carrying a
/// `copy-from` parent. For each such def:
/// 1. Resolve the whole category to get resolved parent + child values.
/// 2. `compute_overrides(parent, child)` → minimal delta.
/// 3. `apply_delta(parent, delta)` must reproduce `child` exactly (ignoring
///    structural control keys), proving import→export is decoupled and inverse.
///
/// Returns a per-category summary. See the `Bridge` CLI subcommand.
pub fn bridge_all_types(loader: &Loader) -> Vec<BridgeSummary> {
    let categories: Vec<(&'static str, &'static str)> =
        crate::for_each_raw_def_kind!(list bridge_category);
    categories
        .into_iter()
        .map(|(json, cat)| {
            let mut s = BridgeSummary {
                json_type: json,
                category: cat,
                ..Default::default()
            };
            run_category_bridge(loader, json, &mut s);
            s
        })
        .collect()
}

fn run_category_bridge(loader: &Loader, json_type: &str, s: &mut BridgeSummary) {
    let (linked, _failures) = loader.resolve_type_raw_with_parent(json_type);
    // Map id → resolved value, and collect (id, parent_id) copy-from links.
    let mut resolved: std::collections::HashMap<String, &Value> = std::collections::HashMap::new();
    let mut children: Vec<(String, String)> = Vec::new();
    for (id, value, parent) in &linked {
        resolved.insert(id.clone(), value);
        if let Some(p) = parent {
            children.push((id.clone(), p.clone()));
        }
    }

    for (id, parent_id) in children {
        let (Some(child), Some(parent)) = (resolved.get(&id), resolved.get(&parent_id)) else {
            continue;
        };
        let delta = compute_overrides(parent, child);
        let rebuilt = apply_delta(parent, &delta);
        if strip_control(&rebuilt) == strip_control(child) {
            s.ok += 1;
        } else {
            s.mismatches += 1;
            if s.mismatches <= 3 {
                eprintln!("[{json_type}] BRIDGE MISMATCH {id} vs parent {parent_id}");
            }
        }
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------
    // Import adapter
    // -------------------------------------------------------------------

    #[test]
    fn import_def_keeps_raw_and_parses_typed() {
        let raw = serde_json::json!({
            "type": "ITEM",
            "id": "x",
            "name": { "str": "X" },
            "weight": "10 g",
            "volume": "1 L",
            "unmodeled_weekend_project_key": { "a": 1 }
        });
        let cfg = import_default_config();
        let rec = import_def::<cdda_defs_raw::raw_defs::ItemDef>("x", &raw, &cfg).unwrap();
        assert_eq!(rec.id, "x");
        // Raw is the source of truth and is retained verbatim, including keys
        // the typed struct does not model.
        assert_eq!(rec.raw, raw);
        assert_eq!(rec.def.name.as_ref().unwrap().singular(), "X");
    }

    #[test]
    fn import_def_reports_typed_parse_error() {
        // `volume` is modeled by Volume which rejects a non-unit string.
        let raw = serde_json::json!({ "id": "b", "volume": "definitely-not-a-volume" });
        let cfg = import_default_config();
        let err = import_def::<cdda_defs_raw::raw_defs::ItemDef>("b", &raw, &cfg).unwrap_err();
        assert!(matches!(err.kind, ImportErrorKind::Parse(_)));
    }

    // -------------------------------------------------------------------
    // Export adapter — compute_overrides
    // -------------------------------------------------------------------

    #[test]
    fn equal_fields_are_omitted() {
        let parent = serde_json::json!({
            "weight": 100, "volume": 250, "name": { "str": "base" }
        });
        let child = serde_json::json!({
            "weight": 100, "volume": 250, "name": { "str": "base" }
        });
        let delta = compute_overrides(&parent, &child);
        assert!(delta.is_empty(), "child identical to parent → no overrides");
    }

    #[test]
    fn changed_and_new_fields_override_removed_fields_delete() {
        let parent = serde_json::json!({
            "weight": 100,
            "volume": 250,
            "material": ["steel"]
        });
        let child = serde_json::json!({
            "weight": 200,               // differs → override
            "volume": 250,               // equal → omit
            "published": true            // new → override
        });
        let delta = compute_overrides(&parent, &child);
        assert!(delta.fields.get("volume").is_none());
        assert_eq!(delta.fields.get("weight"), Some(&serde_json::json!(200)));
        assert_eq!(
            delta.fields.get("published"),
            Some(&serde_json::json!(true))
        );
        assert_eq!(delta.delete, vec!["material".to_string()]);
    }

    #[test]
    fn nested_object_difference_overrides_wholesale() {
        let parent = serde_json::json!({ "name": { "str": "base", "str_pl": "bases" } });
        let child = serde_json::json!({ "name": { "str": "child" } });
        let delta = compute_overrides(&parent, &child);
        // Whole object override, not a partial diff.
        assert_eq!(
            delta.fields.get("name"),
            Some(&serde_json::json!({"str": "child"}))
        );
    }

    #[test]
    fn control_keys_never_leak_into_deltas() {
        let parent = serde_json::json!({
            "type": "ITEM", "id": "parent", "abstract": true, "copy-from": "x",
            "relative": {"weight": 5}, "extend": {"flags": ["A"]}, "data": 1
        });
        let child = serde_json::json!({
            "type": "ITEM", "id": "child", "abstract": true, "copy-from": "x",
            "relative": {"weight": 5}, "extend": {"flags": ["B"]}, "data": 2
        });
        let delta = compute_overrides(&parent, &child);
        // Only the non-control field that changed shows up.
        assert!(delta.fields.get("data").is_some());
        assert!(delta.fields.get("type").is_none());
        assert!(delta.fields.get("id").is_none());
        assert!(delta.fields.get("abstract").is_none());
        assert!(delta.fields.get("copy-from").is_none());
        assert!(delta.fields.get("relative").is_none());
        assert!(delta.fields.get("extend").is_none());
    }

    #[test]
    fn export_override_def_builds_valid_copy_from_object() {
        let parent = serde_json::json!({ "weight": 100, "volume": 250, "material": ["steel"] });
        let child = serde_json::json!({ "weight": 200, "volume": 250, "symbol": "%" });
        let (delta, obj) = export_overrides("ITEM", "child", "parent", &parent, &child);
        assert_eq!(obj["type"], "ITEM");
        assert_eq!(obj["id"], "child");
        assert_eq!(obj["copy-from"], "parent");
        assert_eq!(obj["weight"], 200);
        assert_eq!(obj["symbol"], "%");
        assert_eq!(obj["delete"]["material"], Value::Null);
        assert!(!delta.is_empty());
    }

    /// Round-trip: `compute_overrides` + `export_override_def` produces a def
    /// that, when resolved on top of the parent, reproduces the child. This is
    /// the core lossless guarantee of the export direction.
    #[test]
    fn export_roundtrips_against_resolver() {
        use crate::resolve;

        // Minimal raw map with a boolean-abstract base and a child meant to be
        // the "source of truth" resolved value.
        let parent_resolved = serde_json::json!({
            "id": "base",
            "name": { "str": "base" },
            "weight": 100,
            "volume": 250,
            "material": ["plastic"]
        });
        let child_resolved = serde_json::json!({
            "id": "child",
            "name": { "str": "child" },
            "weight": 100,          // inherits
            "volume": 300,          // override
            "material": ["steel"],  // override
            "symbol": "%"           // new
        });

        let delta = compute_overrides(&parent_resolved, &child_resolved);
        let exported = export_override_def("ITEM", "child", "base", &delta);

        // Re-run the resolver: chain base (parent) → child (exported).
        let mut map = std::collections::HashMap::new();
        map.insert(
            "base".to_string(),
            serde_json::json!({
                "type": "ITEM", "id": "base", "abstract": true,
                "name": { "str": "base" }, "weight": 100, "volume": 250,
                "material": ["plastic"]
            }),
        );
        map.insert("child".to_string(), exported.clone());

        let mut chain = Vec::new();
        let re_resolved = resolve::resolve_copy_from("child", &map, &mut chain).expect("resolve");

        // Strip control/structure keys from both for data comparison.
        let re_resolved_data: Map<String, Value> = re_resolved
            .as_object()
            .unwrap()
            .iter()
            .filter(|(k, _)| !CONTROL_KEYS.contains(&k.as_str()))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        let child_data: Map<String, Value> = child_resolved
            .as_object()
            .unwrap()
            .iter()
            .filter(|(k, _)| !CONTROL_KEYS.contains(&k.as_str()))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        assert_eq!(
            exported.get("id").and_then(Value::as_str),
            Some("child"),
            "exported def keeps its id"
        );
        assert_eq!(
            exported.get("copy-from").and_then(Value::as_str),
            Some("base"),
            "exported def copy-froms the parent"
        );
        // Nothing from the parent material default leaks through, and the
        // edited fields reproduce exactly.
        assert_eq!(
            re_resolved_data, child_data,
            "exported def must re-resolve to the child's resolved value"
        );
    }
}
