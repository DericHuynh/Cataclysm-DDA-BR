//! JSON Schema generation & validation using `schemars`.
//!
//! Generates JSON Schema from Rust types and validates raw CDDA JSON data
//! against it. This makes the Rust types the authoritative source of truth
//! for the data format.

use crate::for_each_raw_def_kind;
use schemars::gen::SchemaGenerator;
use schemars::schema::RootSchema;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

/// Generate a JSON Schema document for a given type.
pub fn generate_schema<T: schemars::JsonSchema>() -> RootSchema {
    let gen = SchemaGenerator::default();
    gen.into_root_schema_for::<T>()
}

/// Generate a JSON Schema for a type and write it to a file.
pub fn write_schema<T: schemars::JsonSchema>(
    path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let schema = generate_schema::<T>();
    let json = serde_json::to_string_pretty(&schema)?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Write JSON Schema files for all known CDDA definition types.
pub fn write_all_schemas(out_dir: &Path) -> Result<(), Vec<Box<dyn std::error::Error>>> {
    let mut errors = Vec::new();
    std::fs::create_dir_all(out_dir).unwrap_or(());

    macro_rules! write_one {
        ($name:ident, $def_ty:ty, $json:expr, $field:ident, $strategy:ident) => {
            let path = out_dir.join(format!("{}.schema.json", $json));
            if let Err(e) = write_schema::<$def_ty>(&path) {
                errors.push(e);
            } else {
                eprintln!("  wrote {}", path.display());
            }
        };
    }

    for_each_raw_def_kind!(call write_one);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Validate a raw JSON value against a type's schema.
///
/// Returns `Ok(())` if valid, or a list of validation error strings.
pub fn validate_against_schema<T: schemars::JsonSchema + serde::de::DeserializeOwned>(
    value: &Value,
) -> Result<(), Vec<String>> {
    match serde_json::from_value::<T>(value.clone()) {
        Ok(_) => Ok(()),
        Err(e) => Err(vec![e.to_string()]),
    }
}

/// Validate all definitions of a specific type from raw data.
///
/// Returns a map of def_id → validation errors.
pub fn validate_all<T: schemars::JsonSchema + serde::de::DeserializeOwned>(
    type_name: &str,
    raw_defs: &HashMap<String, Vec<crate::data::loader::RawDef>>,
) -> HashMap<String, Vec<String>> {
    let mut results = HashMap::new();

    let Some(defs) = raw_defs.get(type_name) else {
        return results;
    };

    for raw in defs {
        let id = raw
            .value
            .get("id")
            .and_then(|v| v.as_str())
            .or_else(|| raw.value.get("abstract").and_then(|v| v.as_str()))
            .or_else(|| raw.value.get("result").and_then(|v| v.as_str()))
            .unwrap_or("<unknown>")
            .to_string();

        // Normalize: promote "abstract" to "id", and also handle "result" → "id",
        // so that definitions using "abstract" instead of "id" can be deserialized.
        let mut value = raw.value.clone();
        if value.get("id").is_none() {
            if let Some(abstract_val) = value.get("abstract").and_then(|v| v.as_str()) {
                value["id"] = serde_json::Value::String(abstract_val.to_string());
            } else if let Some(result_val) = value.get("result").and_then(|v| v.as_str()) {
                value["id"] = serde_json::Value::String(result_val.to_string());
            }
        }

        match serde_json::from_value::<T>(value) {
            Ok(_) => {}
            Err(e) => {
                results
                    .entry(id)
                    .or_insert_with(Vec::new)
                    .push(e.to_string());
            }
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::raw_defs::ItemDef;
    use serde_json::json;

    // -----------------------------------------------------------------------
    // generate_schema
    // -----------------------------------------------------------------------

    /// Should produce a valid RootSchema with a title.
    #[test]
    fn generate_schema_for_item_produces_valid_schema() {
        // Act
        let schema = generate_schema::<ItemDef>();

        // Assert
        assert!(schema.schema.metadata.is_some());
        assert!(schema.schema.metadata.as_ref().unwrap().title.is_some());
    }

    // -----------------------------------------------------------------------
    // validate_against_schema
    // -----------------------------------------------------------------------

    /// A valid minimal ItemDef JSON should pass validation.
    #[test]
    fn validate_valid_minimal_passes() {
        // Arrange
        let value = json!({
            "id": "test_rock",
            "name": {"str": "Test Rock"},
            "volume": "250 ml"
        });

        // Act
        let result = validate_against_schema::<ItemDef>(&value);

        // Assert
        assert!(result.is_ok());
    }

    /// An invalid ItemDef (missing id) should fail validation.
    #[test]
    fn validate_invalid_missing_id_fails() {
        // Arrange
        let value = json!({
            "name": {"str": "Nameless"}
        });

        // Act
        let result = validate_against_schema::<ItemDef>(&value);

        // Assert
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("id")));
    }

    // -----------------------------------------------------------------------
    // validate_all
    // -----------------------------------------------------------------------

    /// validate_all with no raw defs should return an empty map.
    #[test]
    fn validate_all_empty_raws_is_empty() {
        // Arrange
        let raw_defs: HashMap<String, Vec<crate::data::loader::RawDef>> = HashMap::new();

        // Act
        let results = validate_all::<ItemDef>("ITEM", &raw_defs);

        // Assert
        assert!(results.is_empty());
    }
}
