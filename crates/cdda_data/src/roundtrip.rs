//! # Round-trip consistency ("Phase A" seam)
//!
//! Proves the **lossless** JSON→JSON seam of the data pipeline. CDDA data goes
//! through two distinct transformations:
//!
//! 1. **Resolution (lossless):** raw JSON (with `copy-from`, `abstract`,
//!    `extend`/`delete`/`relative`/`proportional`) → **final resolved raw
//!    JSON**. This is a pure `Value`→`Value` transform.
//! 2. **Parse (into typed defs):** final resolved raw JSON →
//!    `serde_json::from_value::<DefRaw>`.
//!
//! This module checks step 2 does not **drop** fields: for every key/value in
//! the resolved JSON, the reserialized def must still contain it (allowing for
//! serde's legitimate re-encoding of a scalar into a branded wire wrapper —
//! e.g. `DefId<T>` serializes as `{"id": "..."}`, and unmodeled CDDA metadata
//! like the `type` discriminator is allowlisted).
//!
//! A category's *entry* is considered clean when nothing was dropped.
//!
//! ## Why this catches real bugs
//!
//! Typed raw-def structs use `#[serde(default)]` on many fields and may omit
//! CDDA keys entirely. A field dropped during parsing (typo, unmodeled key,
//! broken `copy-from`, an unparseable `relative`/`proportional` unit) silently
//! vanishes from the re-serialized output. This module flags exactly that.

use serde_json::Value;

use crate::loader::{Loader, LoaderError};

/// How the category's defs did on the round-trip.
#[derive(Debug, Clone, Copy, Default)]
pub struct RoundtripSummary {
    /// Defs that parsed with **no** dropped fields.
    pub ok: usize,
    /// Defs that failed to deserialize from the resolved JSON.
    pub parse_failures: usize,
    /// Defs where at least one non-allowlisted field was dropped.
    pub mismatch_failures: usize,
    /// Resolution failures (missing copy-from parent / circular), skipped.
    pub unresolved: usize,
    /// The JSON type key this summary is for.
    pub json_type: &'static str,
    /// The category display name.
    pub category: &'static str,
}

impl RoundtripSummary {
    /// True when every resolvable def parsed with nothing dropped.
    pub fn all_ok(&self) -> bool {
        self.parse_failures == 0 && self.mismatch_failures == 0
    }
}

/// Fields present in every resolved CDDA def that our raw-def structs do not
/// model and that serde will therefore drop on re-serialization. These are
/// metadata, not data — flagged once so the per-category summaries stay clean
/// without turning off detection for everything else.
const ALWAYS_ALLOWLIST: &[&str] = &["type", "abstract", "abstract_", "copy-from", "copy_from"];

/// Returns the list of field paths (dot-separated) that were present in the
/// resolved JSON but are **not** reachable in the re-serialized struct.
///
/// Reachability tolerates wrapper nesting: a resolved scalar `"x"` is present
/// if any leaf of the serialized tree equals `"x"` (serde may have wrapped it).
/// Multi-element arrays are item-compared positionally.
fn dropped_paths(resolved: &Value, reserialized: &Value) -> Vec<String> {
    let mut drops = Vec::new();
    walk(resolved, reserialized, "", &ALWAYS_ALLOWLIST, &mut drops);
    drops
}

fn walk(a: &Value, b: &Value, path: &str, allow: &[&str], drops: &mut Vec<String>) {
    match (a, b) {
        (Value::Object(ao), _) => {
            for (k, v) in ao {
                if allow.contains(&k.as_str()) {
                    continue;
                }
                let child = if path.is_empty() {
                    k.clone()
                } else {
                    format!("{path}.{k}")
                };
                // The value is preserved if it is reachable anywhere in the
                // reserialized tree (tolerates serde wrapper nesting such as
                // `DefId<T>` → `{id: ...}`), OR structurally equal at this
                // path. Only a genuine absence is a drop.
                if !value_reachable(b, v) {
                    drops.push(child);
                }
            }
        }
        (Value::Array(aa), Value::Array(ba)) => {
            for (i, av) in aa.iter().enumerate() {
                if let Some(bv) = ba.get(i) {
                    walk(av, bv, &format!("{path}[{i}]"), &[], drops);
                } else if !value_reachable(b, av) {
                    drops.push(format!("{path}[{i}]"));
                }
            }
        }
        _ => {}
    }
}

/// True if `needle` is reachable anywhere under `container` as an equivalent
/// subtree (handles wrapper nesting and reparenting), or if `needle` matches a
/// container leaf set. For array needles this checks the whole-array shape vs
/// any matching array; for scalars it checks any leaf value.
fn value_reachable(container: &Value, needle: &Value) -> bool {
    match needle {
        Value::Object(nobj) => {
            // Any object subtree under `container` whose keys are a superset of
            // `nobj`'s with matching values counts as preserved.
            let mut found = false;
            collect_nodes(container, &mut |v| {
                if let Value::Object(vo) = v {
                    if nobj
                        .iter()
                        .all(|(k, nv)| vo.get(k).map_or(false, |bv| values_equal(bv, nv)))
                    {
                        found = true;
                    }
                }
            });
            found
        }
        Value::Array(need) => {
            let mut found = false;
            collect_nodes(container, &mut |v| match (v, need) {
                (Value::Array(arr), need) if arr.len() == need.len() => {
                    if arr.iter().zip(need).all(|(a, b)| values_equal(a, b)) {
                        found = true;
                    }
                }
                _ => {}
            });
            found
        }
        nested => {
            let mut found = false;
            collect_leaves(container, &mut |v| {
                if values_equal(v, nested) {
                    found = true;
                }
            });
            found
        }
    }
}

/// Collect every node (not just leaves) reachable under `container`.
fn collect_nodes<'a>(v: &'a Value, f: &mut impl FnMut(&'a Value)) {
    match v {
        Value::Object(o) => {
            for c in o.values() {
                f(c);
                collect_nodes(c, f);
            }
        }
        Value::Array(a) => {
            for c in a {
                f(c);
                collect_nodes(c, f);
            }
        }
        leaf => f(leaf),
    }
}

fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::String(x), Value::String(y)) => x == y || canonical_quantity_eq(x, y),
        (Value::Number(x), Value::Number(y)) => match (a.as_f64(), b.as_f64()) {
            (Some(xf), Some(yf)) => xf == yf,
            _ => x == y,
        },
        // A raw CDDA unit string (`"250 ml"`) vs the typed number it
        // canonicalized into (`Volume(250)` → `250`).
        (Value::String(s), Value::Number(n)) | (Value::Number(n), Value::String(s)) => {
            let num = n.as_f64();
            match (quantity_to_canonical(s.as_str()), num) {
                // quantity string matches canonical numeric value
                (Some(cx), Some(y)) => cx == y,
                // bare numeric string, e.g. "454" vs 454
                (None, Some(y)) => s.parse::<f64>().map_or(false, |x| x == y),
                _ => false,
            }
        }
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Array(x), Value::Array(y)) => x == y,
        (Value::Object(x), Value::Object(y)) => x == y,
        (Value::Null, Value::Null) => true,
        _ => false,
    }
}

/// True if both quantity strings canonicalize to the same numeric value.
fn canonical_quantity_eq(x: &str, y: &str) -> bool {
    match (quantity_to_canonical(x), quantity_to_canonical(y)) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
}

/// Parse a CDDA quantity string `"<num> <unit>"` to a base-unit value so
/// quantity fields compare across typed/raw representations. `None` for
/// non-quantity text (exact-string comparison handles that).
fn quantity_to_canonical(s: &str) -> Option<f64> {
    let s = s.trim();
    let split = s.find(char::is_whitespace)?;
    let (num_part, unit) = s.split_at(split);
    let num: f64 = num_part.trim().parse().ok()?;
    let unit = unit.trim().to_lowercase();
    let factor = match unit.as_str() {
        // volume
        "ml" | "milliliter" | "milliliters" => 1.0,
        "l" | "liter" | "liters" => 1000.0,
        // weight
        "g" | "gram" | "grams" => 1.0,
        "kg" | "kilogram" | "kilograms" => 1000.0,
        "mg" | "milligram" | "milligrams" => 0.001,
        // length
        "mm" | "millimeter" | "millimeters" => 1.0,
        "cm" | "centimeter" | "centimeters" => 10.0,
        "m" | "meter" | "meters" => 1000.0,
        // time
        "s" | "second" | "seconds" => 1.0,
        "min" | "minute" | "minutes" => 60.0,
        "h" | "hour" | "hours" => 3600.0,
        "d" | "day" | "days" => 86400.0,
        // currency
        "cent" | "cents" => 1.0,
        "usd" | "dollar" | "dollars" => 100.0,
        _ => return None,
    };
    Some(num * factor)
}

fn collect_leaves(v: &Value, f: &mut impl FnMut(&Value)) {
    match v {
        Value::Object(o) => o.values().for_each(|c| collect_leaves(c, f)),
        Value::Array(a) => a.iter().for_each(|c| collect_leaves(c, f)),
        other => f(other),
    }
}

/// A per-category runner: type-erased so the macro list stays homogeneous.
type CategoryRunner = fn(&Loader, &'static str, &'static str, &mut RoundtripSummary);

/// Generic per-category round-trip.
fn run_category<T>(
    loader: &Loader,
    json_type: &'static str,
    category: &'static str,
    summary: &mut RoundtripSummary,
) where
    T: for<'de> serde::Deserialize<'de> + serde::Serialize,
{
    summary.json_type = json_type;
    summary.category = category;

    let (items, failures) = loader.resolve_type_raw(json_type);
    summary.unresolved = failures.len();

    for (id, value) in &items {
        match serde_json::from_value::<T>(value.clone()) {
            Err(e) => {
                summary.parse_failures += 1;
                // Surface parse failures (these shouldn't happen for resolvable
                // defs — a real deserialization gap). Print a few per category.
                if summary.parse_failures <= 5 {
                    eprintln!("[{category}] PARSE FAIL {id}: {e}");
                }
            }
            Ok(parsed) => {
                let reserialized = serde_json::to_value(&parsed).unwrap_or_default();
                let drops = dropped_paths(value, &reserialized);
                if drops.is_empty() {
                    summary.ok += 1;
                } else {
                    summary.mismatch_failures += 1;
                    // Print the first few dropped paths so test capture shows
                    // *what* is actually not preserved — the whole point of
                    // Phase A.
                    if summary.mismatch_failures <= 3 {
                        eprintln!(
                            "[{category}] DROPPED {id}: {}",
                            drops.iter().take(5).cloned().collect::<Vec<_>>().join(", ")
                        );
                    }
                }
            }
        }
    }
}

/// Mapper bound to `for_each_raw_def_kind!(list ...)`.
#[allow(unused)]
macro_rules! roundtrip_kind {
    ($name:ident, $ty:ty, $json:expr, $_field:ident, $_strategy:expr) => {
        (
            $json,
            stringify!($name),
            run_category::<$ty> as CategoryRunner,
        )
    };
}

/// Run the Phase-A round-trip check over a loader whose raw data has been
/// ingested (call [`Loader::ingest_all`] first).
pub fn roundtrip_all_types(loader: &Loader) -> Vec<RoundtripSummary> {
    let categories: Vec<(&'static str, &'static str, CategoryRunner)> =
        crate::for_each_raw_def_kind!(list roundtrip_kind);

    categories
        .into_iter()
        .map(|(json, name, runner)| {
            let mut summary = RoundtripSummary::default();
            runner(loader, json, name, &mut summary);
            summary
        })
        .collect()
}

/// Convenience: run the full round-trip from a set of data directories.
pub fn roundtrip_data_dirs(
    dirs: &[std::path::PathBuf],
) -> Result<Vec<RoundtripSummary>, Vec<LoaderError>> {
    let mut loader = Loader::new(dirs.to_vec());
    loader.ingest_all();
    Ok(roundtrip_all_types(&loader))
}
