//! CDDA patch logic — extend / delete / override semantics.
//!
//! Mirrors the C++ `generic_factory` extend/delete behavior:
//! - `extend` adds elements to an existing array; warns if the field doesn't exist.
//! - `delete` removes elements from an existing array; warns if the field doesn't exist.
//! - Other fields override (replace) existing values.

use serde_json::{Map, Value};
use tracing::warn;

pub fn apply_cdda_patch(base: &mut Value, modifier: &Value) {
    match (base, modifier) {
        (Value::Object(base_map), Value::Object(mod_map)) => {
            for (key, mod_val) in mod_map {
                match key.as_str() {
                    "extend" => {
                        if let Value::Object(obj) = mod_val {
                            apply_extend(base_map, obj);
                        }
                    }
                    "delete" => {
                        if let Value::Object(obj) = mod_val {
                            apply_delete(base_map, obj);
                        }
                    }
                    _ => match base_map.get_mut(key) {
                        Some(bv) => apply_cdda_patch(bv, mod_val),
                        None => {
                            base_map.insert(key.clone(), mod_val.clone());
                        }
                    },
                }
            }
        }
        (Value::Array(ba), Value::Array(ma)) => *ba = ma.clone(),
        (br, m) => *br = m.clone(),
    }
}

/// Apply `extend` semantics to a base object.
///
/// For each key in the extend object, the value must be an array.
/// Elements are appended to the existing array in the base.
/// If the key doesn't exist in the base, a warning is emitted (matching C++ behavior).
fn apply_extend(base: &mut Map<String, Value>, extend: &Map<String, Value>) {
    for (key, ext_val) in extend {
        let Value::Array(v) = ext_val else {
            continue;
        };
        match base.get_mut(key) {
            Some(Value::Array(arr)) => {
                for item in v {
                    if !arr.contains(item) {
                        arr.push(item.clone());
                    }
                }
            }
            Some(_) => {
                warn!("extend: field '{}' is not an array, skipping extend", key);
            }
            None => {
                warn!(
                    "extend: field '{}' does not exist in base, skipping (CDDA semantics: extend requires an existing field)",
                    key
                );
            }
        }
    }
}

/// Apply `delete` semantics to a base object.
///
/// For each key in the delete object, the value must be an array.
/// Elements are removed from the existing array in the base.
/// If the key doesn't exist in the base, a warning is emitted (matching C++ behavior).
fn apply_delete(base: &mut Map<String, Value>, delete: &Map<String, Value>) {
    for (key, del_val) in delete {
        let Value::Array(v) = del_val else {
            continue;
        };
        match base.get_mut(key) {
            Some(Value::Array(arr)) => {
                arr.retain(|i| !v.iter().any(|d| d == i));
            }
            Some(_) => {
                warn!("delete: field '{}' is not an array, skipping delete", key);
            }
            None => {
                warn!(
                    "delete: field '{}' does not exist in base, skipping (CDDA semantics: delete requires an existing field)",
                    key
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_override() {
        let mut b = json!({"name": "rock", "weight": 100});
        apply_cdda_patch(&mut b, &json!({"weight": 200}));
        assert_eq!(b["weight"], 200);
    }

    #[test]
    fn test_extend() {
        let mut b = json!({"flags": ["FIRE", "WET"]});
        apply_cdda_patch(&mut b, &json!({"extend": {"flags": ["HOT"]}}));
        let fs: Vec<&str> = b["flags"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert!(fs.contains(&"FIRE") && fs.contains(&"HOT"));
    }

    #[test]
    fn test_delete() {
        let mut b = json!({"flags": ["FIRE", "WET", "HOT"]});
        apply_cdda_patch(&mut b, &json!({"delete": {"flags": ["WET"]}}));
        let fs: Vec<&str> = b["flags"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert!(!fs.contains(&"WET"));
    }

    /// CDDA semantics: extending a field that doesn't exist in the base is a no-op.
    /// This test verifies we don't silently create new fields via extend.
    #[test]
    fn test_extend_missing_field_is_noop() {
        let mut b = json!({"name": "sword"});
        apply_cdda_patch(&mut b, &json!({"extend": {"flags": ["FLAMING"]}}));
        // "flags" should NOT be created — CDDA semantics require it to exist first.
        assert!(b.get("flags").is_none());
        assert_eq!(b["name"], "sword");
    }

    /// CDDA semantics: deleting from a field that doesn't exist is a no-op.
    #[test]
    fn test_delete_missing_field_is_noop() {
        let mut b = json!({"name": "sword"});
        apply_cdda_patch(&mut b, &json!({"delete": {"flags": ["FIRE"]}}));
        assert_eq!(b["name"], "sword");
    }
}
