//! `copy-from` inheritance resolver.
//!
//! CDDA's `copy-from` is not just value merging. It supports:
//! - **extend**: Add elements to array/object fields.
//! - **delete**: Remove elements from array/object fields.
//! - **relative**: Numeric delta (e.g. weight+200g).
//! - **proportional**: Numeric multiplier (e.g. weight×0.5).
//! - **abstract**: Base definitions that never appear in the final registry.

use serde_json::Value;
use std::collections::HashMap;
use tracing::debug;

/// Resolve a `copy-from` inheritance chain for a given definition.
///
/// Returns the fully-resolved `Value` with all inheritance operations applied.
pub fn resolve_copy_from(
    def_id: &str,
    raw_defs: &HashMap<String, Value>,
    chain: &mut Vec<String>,
) -> Result<Value, String> {
    // Detect cycles
    if chain.contains(&def_id.to_string()) {
        chain.push(def_id.to_string());
        return Err(format!("Circular copy-from dependency: {:?}", chain));
    }

    let raw = raw_defs
        .get(def_id)
        .ok_or_else(|| format!("Definition '{}' not found", def_id))?;

    chain.push(def_id.to_string());

    // Check if this def has a copy-from parent
    let parent_id = raw
        .get("copy_from")
        .or_else(|| raw.get("copy-from"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let base = if let Some(ref parent) = parent_id {
        resolve_copy_from(parent, raw_defs, chain)?
    } else {
        Value::Object(serde_json::Map::new())
    };

    // Apply current definition's fields
    let mut resolved = base;
    apply_definition(raw, &mut resolved);

    // Apply extend/delete/relative/proportional
    if let Some(ops) = raw.get("extend").and_then(|v| v.as_object()) {
        for (key, val) in ops {
            apply_extend(&mut resolved, key, val);
        }
    }
    if let Some(ops) = raw.get("delete").and_then(|v| v.as_object()) {
        for (key, val) in ops {
            apply_delete(&mut resolved, key, val);
        }
    }
    if let Some(ops) = raw.get("relative").and_then(|v| v.as_object()) {
        for (key, val) in ops {
            apply_relative(&mut resolved, key, val);
        }
    }
    if let Some(ops) = raw.get("proportional").and_then(|v| v.as_object()) {
        for (key, val) in ops {
            apply_proportional(&mut resolved, key, val);
        }
    }

    // A *named* abstract (`"abstract": "some_id"`) is a real definition whose
    // identifier lives in the `abstract` field; the boolean form
    // (`"abstract": true`) is a template that never reaches the registry.
    // `apply_definition` strips the `abstract` key, so restore it as `id` here
    // (unless the def already carries an explicit `id`) to keep named-abstra
    // defs deserializable after resolution.
    if resolved
        .get("id")
        .and_then(serde_json::Value::as_str)
        .is_none()
    {
        if let Some(abs_id) = raw.get("abstract").and_then(serde_json::Value::as_str) {
            if let Some(obj) = resolved.as_object_mut() {
                obj.insert(
                    "id".to_string(),
                    serde_json::Value::String(abs_id.to_string()),
                );
            }
        }
    }

    chain.pop();
    Ok(resolved)
}

/// Apply a definition's own fields onto a resolved base value.
fn apply_definition(def: &Value, target: &mut Value) {
    let Some(def_obj) = def.as_object() else {
        return;
    };
    let Some(target_obj) = target.as_object_mut() else {
        return;
    };
    for (key, val) in def_obj {
        if matches!(
            key.as_str(),
            "copy-from"
                | "copy_from"
                | "extend"
                | "delete"
                | "relative"
                | "proportional"
                | "abstract"
        ) {
            continue;
        }
        target_obj.insert(key.clone(), val.clone());
    }
}

/// Apply `extend`: add elements to array or object fields.
fn apply_extend(target: &mut Value, key: &str, val: &Value) {
    let Some(target_obj) = target.as_object_mut() else {
        return;
    };
    let entry = target_obj.entry(key.to_string()).or_insert_with(|| {
        if val.is_array() {
            Value::Array(Vec::new())
        } else if val.is_object() {
            Value::Object(serde_json::Map::new())
        } else {
            Value::Null
        }
    });

    match (entry, val) {
        (Value::Array(arr), Value::Array(extras)) => {
            for extra in extras {
                if !arr.contains(extra) {
                    arr.push(extra.clone());
                }
            }
        }
        (Value::Object(obj), Value::Object(extras)) => {
            for (k, v) in extras {
                obj.insert(k.clone(), v.clone());
            }
        }
        _ => debug!("Cannot extend non-array/non-object field '{}'", key),
    }
}

/// Apply `delete`: remove elements from array or object fields.
fn apply_delete(target: &mut Value, key: &str, val: &Value) {
    let Some(target_obj) = target.as_object_mut() else {
        return;
    };
    let entry = match target_obj.get_mut(key) {
        Some(e) => e,
        None => return,
    };
    match (entry, val) {
        (Value::Array(arr), Value::Array(removals)) => {
            arr.retain(|item| !removals.contains(item));
        }
        (Value::Object(obj), Value::Object(removals)) => {
            for k in removals.keys() {
                obj.remove(k);
            }
        }
        _ => debug!("Cannot delete from non-array/non-object field '{}'", key),
    }
}

/// Apply `relative`: add a delta to numeric fields.
fn apply_relative(target: &mut Value, key: &str, val: &Value) {
    let Some(target_obj) = target.as_object_mut() else {
        return;
    };
    let entry = match target_obj.get_mut(key) {
        Some(e) => e,
        None => return,
    };
    if let (Some(current), Some(delta)) = (entry.as_i64(), val.as_i64()) {
        *entry = Value::Number(serde_json::Number::from(current + delta));
    } else if let (Some(current), Some(delta)) = (entry.as_f64(), val.as_f64()) {
        if let Some(n) = relative_number(current, delta) {
            *entry = Value::Number(n);
        }
    }
}

/// Apply `proportional`: multiply a numeric field by a factor.
///
/// Handles both plain JSON numbers (`25`) and CDDA string-encoded unit values
/// (`"5 g"`, `"250 ml"`, `"10 cent"`, …).  String values are parsed as
/// `<number> <unit>`, multiplied, and written back in the same format.
fn apply_proportional(target: &mut Value, key: &str, val: &Value) {
    let Some(target_obj) = target.as_object_mut() else {
        return;
    };
    let Some(entry) = target_obj.get_mut(key) else {
        return;
    };
    let Some(factor) = val.as_f64() else {
        return;
    };

    match entry {
        Value::Number(n) => {
            if let Some(current) = n.as_f64() {
                if let Some(new_n) = scaled_number(current, factor) {
                    *entry = Value::Number(new_n);
                }
            }
        }
        Value::String(s) => {
            if let Some(new_s) = multiply_cdda_value_string(s, factor) {
                *entry = Value::String(new_s);
            }
        }
        _ => {}
    }
}

/// Add `delta` to `current`, preserving integer-ness for integer operands so
/// integer fields stay integer (CDDA `relative` on ints stays int).
fn relative_number(current: f64, delta: f64) -> Option<serde_json::Number> {
    if !delta.is_finite() {
        return None;
    }
    let result = current + delta;
    if result.is_infinite() || result.is_nan() {
        return None;
    }
    if current.fract() == 0.0 && delta.fract() == 0.0 {
        return number_from_integral_f64(result);
    }
    clean_float(result)
}

/// Multiply `current` by `factor`. If `current` is an integral amount (CDDA
/// integer attribute like `damage`/`range`/`recoil`), emit an integer rounded to
/// nearest (CDDA semantics) so it still deserializes into `i32`/`u64` fields.
/// Otherwise snap to a bounded number of decimals to kill FP noise.
fn scaled_number(current: f64, factor: f64) -> Option<serde_json::Number> {
    if !factor.is_finite() {
        return None;
    }
    let result = current * factor;
    if result.is_infinite() || result.is_nan() {
        return None;
    }
    // Integer operand → integer result (round to nearest), matching CDDA's
    // integer-field scaling.
    if current.fract() == 0.0 {
        let rounded = result.round();
        return number_from_integral_f64(rounded);
    }
    clean_float(result)
}

/// FP-noise tolerance and round-trip precision bounds.
const EPSILON: f64 = 1e-9;
const MAX_DECIMALS: u32 = 10;

/// Emit an integer JSON number when `v` is (within epsilon of) an integral
/// value, otherwise a float snapped to a bounded number of decimals.
fn clean_float(v: f64) -> Option<serde_json::Number> {
    if !v.is_finite() {
        return None;
    }
    let rounded = v.round();
    if (v - rounded).abs() < EPSILON {
        return number_from_integral_f64(rounded);
    }
    let scale = 10f64.powi(MAX_DECIMALS as i32);
    let snapped = (v * scale).round() / scale;
    serde_json::Number::from_f64(snapped)
}

/// Convert an integral `f64` into an integer JSON number (so it deserializes
/// into `i32`/`u64` fields), falling back to a float if out of integer range.
fn number_from_integral_f64(v: f64) -> Option<serde_json::Number> {
    if v >= 0.0 && v <= u64::MAX as f64 {
        return Some(serde_json::Number::from(v as u64));
    }
    if v >= i64::MIN as f64 && v <= i64::MAX as f64 {
        return Some(serde_json::Number::from(v as i64));
    }
    serde_json::Number::from_f64(v)
}

/// Parse a CDDA `"<number> <unit>"` string, multiply the numeric part by
/// `factor`, and return the result in the same `"<number> <unit>"` format.
///
/// Examples:
/// - `"5 g"` × 6 → `"30 g"`
/// - `"250 ml"` × 0.5 → `"125 ml"`
/// - `"10 cent"` × 6 → `"60 cent"`
fn multiply_cdda_value_string(s: &str, factor: f64) -> Option<String> {
    let s = s.trim();
    // Find where the numeric part ends (digits + optional decimal point/sign).
    let num_end = s
        .char_indices()
        .find(|(_, c)| !c.is_ascii_digit() && *c != '.' && *c != '-')
        .map(|(i, _)| i)
        .unwrap_or(s.len());
    if num_end == 0 {
        return None;
    }
    let (num_str, rest) = s.split_at(num_end);
    let num: f64 = num_str.parse().ok()?;
    let unit = rest.trim();
    let result = num * factor;
    Some(if result.fract() < 1e-9 {
        if unit.is_empty() {
            format!("{}", result as i64)
        } else {
            format!("{} {}", result as i64, unit)
        }
    } else {
        if unit.is_empty() {
            format!("{}", result)
        } else {
            format!("{} {}", result, unit)
        }
    })
}

/// Topological sort of definitions by copy-from dependency.
///
/// Returns definition IDs in resolution order (base first).
pub fn topological_sort<'a>(
    defs: &'a HashMap<String, Value>,
) -> Result<Vec<&'a str>, Vec<Vec<String>>> {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum VisitState {
        Unvisited,
        Visiting,
        Visited,
    }

    let mut visited: HashMap<&str, VisitState> = HashMap::new();
    let mut order: Vec<&str> = Vec::new();
    let mut cycles: Vec<Vec<String>> = Vec::new();

    fn dfs<'a>(
        id: &'a str,
        defs: &'a HashMap<String, Value>,
        visited: &mut HashMap<&'a str, VisitState>,
        order: &mut Vec<&'a str>,
        stack: &mut Vec<String>,
    ) -> Result<(), Vec<String>> {
        match visited.get(id).copied().unwrap_or(VisitState::Unvisited) {
            VisitState::Visited => return Ok(()),
            VisitState::Visiting => {
                stack.push(id.to_string());
                return Err(stack.clone());
            }
            VisitState::Unvisited => {}
        }
        visited.insert(id, VisitState::Visiting);
        stack.push(id.to_string());

        if let Some(parent) = defs
            .get(id)
            .and_then(|v| v.get("copy-from").or_else(|| v.get("copy_from")))
            .and_then(|v| v.as_str())
        {
            if defs.contains_key(parent) {
                dfs(parent, defs, visited, order, stack)?;
            }
        }

        stack.pop();
        visited.insert(id, VisitState::Visited);
        order.push(id);
        Ok(())
    }

    for id in defs.keys() {
        if !visited.contains_key(id.as_str()) {
            let mut stack: Vec<String> = Vec::new();
            if let Err(cycle) = dfs(id.as_str(), defs, &mut visited, &mut order, &mut stack) {
                cycles.push(cycle);
            }
        }
    }

    if cycles.is_empty() {
        Ok(order)
    } else {
        Err(cycles)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_basic_copy_from() {
        // Arrange
        let mut defs = HashMap::new();
        defs.insert(
            "base_item".into(),
            json!({ "id": "base_item", "volume": "250 ml", "weight": "100 g" }),
        );
        defs.insert(
            "child_item".into(),
            json!({ "id": "child_item", "copy-from": "base_item", "color": "red" }),
        );
        let mut chain = Vec::new();

        // Act
        let resolved = resolve_copy_from("child_item", &defs, &mut chain).unwrap();

        // Assert
        assert_eq!(
            resolved.get("volume").and_then(|v| v.as_str()),
            Some("250 ml")
        );
        assert_eq!(
            resolved.get("weight").and_then(|v| v.as_str()),
            Some("100 g")
        );
        assert_eq!(resolved.get("color").and_then(|v| v.as_str()), Some("red"));
    }

    #[test]
    fn test_extend_flags() {
        // Arrange
        let mut defs = HashMap::new();
        defs.insert("base".into(), json!({"id": "base", "flags": ["FLAG_A"]}));
        defs.insert(
            "child".into(),
            json!({"id": "child", "copy-from": "base", "extend": {"flags": ["FLAG_B"]}}),
        );
        let mut chain = Vec::new();

        // Act
        let resolved = resolve_copy_from("child", &defs, &mut chain).unwrap();
        let flags = resolved["flags"].as_array().unwrap();
        let strs: Vec<&str> = flags.iter().map(|v| v.as_str().unwrap()).collect();

        // Assert
        assert!(strs.contains(&"FLAG_A"));
        assert!(strs.contains(&"FLAG_B"));
    }

    #[test]
    fn test_delete_flags() {
        // Arrange
        let mut defs = HashMap::new();
        defs.insert(
            "base".into(),
            json!({"id": "base", "flags": ["A", "B", "C"]}),
        );
        defs.insert(
            "child".into(),
            json!({"id": "child", "copy-from": "base", "delete": {"flags": ["B"]}}),
        );
        let mut chain = Vec::new();

        // Act
        let resolved = resolve_copy_from("child", &defs, &mut chain).unwrap();
        let flags = resolved["flags"].as_array().unwrap();
        let strs: Vec<&str> = flags.iter().map(|v| v.as_str().unwrap()).collect();

        // Assert
        assert_eq!(strs, vec!["A", "C"]);
    }

    #[test]
    fn test_relative() {
        // Arrange
        let mut defs = HashMap::new();
        defs.insert("base".into(), json!({"id": "base", "weight": 1000}));
        defs.insert(
            "child".into(),
            json!({"id": "child", "copy-from": "base", "relative": {"weight": 200}}),
        );
        let mut chain = Vec::new();

        // Act
        let resolved = resolve_copy_from("child", &defs, &mut chain).unwrap();

        // Assert
        assert_eq!(resolved["weight"].as_i64(), Some(1200));
    }

    #[test]
    fn test_cycle_detection() {
        // Arrange
        let mut defs = HashMap::new();
        defs.insert("a".into(), json!({"id": "a", "copy-from": "b"}));
        defs.insert("b".into(), json!({"id": "b", "copy-from": "a"}));
        let mut chain = Vec::new();

        // Act
        let result = resolve_copy_from("a", &defs, &mut chain);

        // Assert
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Circular"));
    }

    #[test]
    fn test_topological_sort() {
        // Arrange
        let mut defs = HashMap::new();
        defs.insert("a".into(), json!({"id": "a"}));
        defs.insert("b".into(), json!({"id": "b", "copy-from": "a"}));
        defs.insert("c".into(), json!({"id": "c", "copy-from": "b"}));

        // Act
        let order = topological_sort(&defs).unwrap();

        // Assert
        assert_eq!(order, vec!["a", "b", "c"]);
    }

    /// proportional multiplies numeric fields.
    #[test]
    fn test_proportional() {
        // Arrange
        let mut defs = HashMap::new();
        defs.insert("base".into(), json!({"id": "base", "weight": 1000}));
        defs.insert(
            "child".into(),
            json!({"id": "child", "copy-from": "base", "proportional": {"weight": 0.5}}),
        );
        let mut chain = Vec::new();

        // Act
        let resolved = resolve_copy_from("child", &defs, &mut chain).unwrap();

        // Assert
        assert_eq!(resolved["weight"].as_f64(), Some(500.0));
    }

    /// Child fields override parent fields.
    #[test]
    fn test_override_parent_field() {
        // Arrange
        let mut defs = HashMap::new();
        defs.insert(
            "base".into(),
            json!({"id": "base", "volume": "1 L", "color": "red"}),
        );
        defs.insert(
            "child".into(),
            json!({"id": "child", "copy-from": "base", "color": "blue"}),
        );
        let mut chain = Vec::new();

        // Act
        let resolved = resolve_copy_from("child", &defs, &mut chain).unwrap();

        // Assert
        assert_eq!(resolved["color"].as_str(), Some("blue"));
        assert_eq!(resolved["volume"].as_str(), Some("1 L"));
    }

    /// Chained copy-from resolves all levels.
    #[test]
    fn test_three_level_chain() {
        // Arrange
        let mut defs = HashMap::new();
        defs.insert("1".into(), json!({"id": "1", "a": "from_1"}));
        defs.insert(
            "2".into(),
            json!({"id": "2", "copy-from": "1", "b": "from_2"}),
        );
        defs.insert(
            "3".into(),
            json!({"id": "3", "copy-from": "2", "c": "from_3"}),
        );
        let mut chain = Vec::new();

        // Act
        let resolved = resolve_copy_from("3", &defs, &mut chain).unwrap();

        // Assert
        assert_eq!(resolved["a"].as_str(), Some("from_1"));
        assert_eq!(resolved["b"].as_str(), Some("from_2"));
        assert_eq!(resolved["c"].as_str(), Some("from_3"));
    }

    /// proportional multiplies string-encoded unit values (CDDA format).
    #[test]
    fn test_proportional_string_units() {
        let mut defs = HashMap::new();
        defs.insert(
            "base".into(),
            json!({"id": "base", "weight": "5 g", "volume": "10 ml", "price": "10 cent"}),
        );
        defs.insert(
            "child".into(),
            json!({"id": "child", "copy-from": "base",
                   "proportional": {"weight": 6, "volume": 6, "price": 6}}),
        );
        let mut chain = Vec::new();
        let resolved = resolve_copy_from("child", &defs, &mut chain).unwrap();

        assert_eq!(resolved["weight"].as_str(), Some("30 g"));
        assert_eq!(resolved["volume"].as_str(), Some("60 ml"));
        assert_eq!(resolved["price"].as_str(), Some("60 cent"));
    }

    /// proportional with fractional factor preserves decimal.
    #[test]
    fn test_proportional_string_fractional() {
        let mut defs = HashMap::new();
        defs.insert("base".into(), json!({"id": "base", "weight": "100 g"}));
        defs.insert(
            "child".into(),
            json!({"id": "child", "copy-from": "base", "proportional": {"weight": 0.5}}),
        );
        let mut chain = Vec::new();
        let resolved = resolve_copy_from("child", &defs, &mut chain).unwrap();
        assert_eq!(resolved["weight"].as_str(), Some("50 g"));
    }

    // ── Regression: B bug 1 ── integer fields stay integer after relative /
    // proportional (no FP noise like `825.0000000000001`), so the resolved JSON
    // still deserializes into i32/u64 fields.
    #[test]
    fn test_proportional_on_integer_emits_integer() {
        let mut defs = HashMap::new();
        defs.insert("base".into(), json!({"id": "base", "recoil": 750}));
        defs.insert(
            "child".into(),
            json!({"id": "child", "copy-from": "base", "proportional": {"recoil": 1.1}}),
        );
        let mut chain = Vec::new();
        let resolved = resolve_copy_from("child", &defs, &mut chain).unwrap();
        // 750 * 1.1 = 825.0000000000001 → must be integer 825.
        assert_eq!(resolved["recoil"].as_i64(), Some(825));
        assert!(resolved["recoil"].is_i64());
    }

    #[test]
    fn test_proportional_integer_rounding() {
        // CDDA rounds fractional results of integer fields (0.6 * 13 = 7.8 → 8).
        let mut defs = HashMap::new();
        defs.insert("base".into(), json!({"id": "base", "range": 13}));
        defs.insert(
            "child".into(),
            json!({"id": "child", "copy-from": "base", "proportional": {"range": 0.6}}),
        );
        let mut chain = Vec::new();
        let resolved = resolve_copy_from("child", &defs, &mut chain).unwrap();
        assert_eq!(resolved["range"].as_i64(), Some(8));
    }

    #[test]
    fn test_relative_on_integer_stays_integer() {
        let mut defs = HashMap::new();
        defs.insert("base".into(), json!({"id": "base", "dispersion": 10}));
        defs.insert(
            "child".into(),
            json!({"id": "child", "copy-from": "base", "relative": {"dispersion": 2}}),
        );
        let mut chain = Vec::new();
        let resolved = resolve_copy_from("child", &defs, &mut chain).unwrap();
        assert_eq!(resolved["dispersion"].as_i64(), Some(12));
        assert!(resolved["dispersion"].is_i64());
    }

    // ── Regression: B bug 2 — a *named* abstract (`"abstract": "some_id"`)
    // is a real def whose identifier must be preserved through resolution as
    // `id` (not stripped), so it deserializes.
    #[test]
    fn test_named_abstract_promotes_to_id() {
        let mut defs = HashMap::new();
        defs.insert(
            "20x66_abstract".into(),
            json!({
                "abstract": "20x66_abstract",
                "type": "ITEM",
                "weight": "66 g",
                "volume": "415 ml",
            }),
        );
        defs.insert(
            "20x66_slug".into(),
            json!({"id": "20x66_slug", "copy-from": "20x66_abstract", "price": "2 USD"}),
        );
        let mut chain = Vec::new();
        let resolved = resolve_copy_from("20x66_abstract", &defs, &mut chain).unwrap();
        assert_eq!(resolved["id"].as_str(), Some("20x66_abstract"));
        assert_eq!(resolved["weight"].as_str(), Some("66 g"));

        // And it still works as a copy-from parent: the child inherits.
        let mut chain = Vec::new();
        let child = resolve_copy_from("20x66_slug", &defs, &mut chain).unwrap();
        assert_eq!(child["id"].as_str(), Some("20x66_slug"));
        assert_eq!(child["weight"].as_str(), Some("66 g"));
    }

    #[test]
    fn test_boolean_abstract_template_has_no_id() {
        // `"abstract": true` is a template — it must NOT get an id.
        let mut defs = HashMap::new();
        defs.insert(
            "tpl".into(),
            json!({"abstract": true, "type": "ITEM", "weight": "1 g"}),
        );
        let mut chain = Vec::new();
        let resolved = resolve_copy_from("tpl", &defs, &mut chain).unwrap();
        assert_eq!(resolved["id"].as_str(), None);
    }
}
