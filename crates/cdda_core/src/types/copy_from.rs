use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Represents a `copy-from` inheritance reference in a CDDA definition.
///
/// CDDA's `copy-from` is not just value merging — it supports field-level
/// operations: `extend` (add to arrays), `delete` (remove from arrays),
/// `relative` (delta modifications on numeric fields), and `proportional`
/// (multiplicative modifications on numeric fields).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyFromTarget {
    /// The ID of the definition to copy from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub copy_from: Option<String>,

    /// If true, this definition is abstract and should not appear in the final registry.
    #[serde(default)]
    pub abstract_: Option<bool>,

    /// Fields to extend (add elements to arrays).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extend: Option<serde_json::Value>,

    /// Fields to delete (remove elements from arrays).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delete: Option<serde_json::Value>,

    /// Fields to modify by a relative delta.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relative: Option<serde_json::Value>,

    /// Fields to modify by a proportional multiplier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proportional: Option<serde_json::Value>,
}

/// An operation to apply during `copy-from` resolution.
#[derive(Debug, Clone)]
pub enum CopyFromOp {
    /// Set a field to a literal value (the default merge operation).
    Set(String, Value),
    /// Extend an array field with new elements.
    Extend(String, Value),
    /// Delete elements from an array field.
    Delete(String, Value),
    /// Add a delta to a numeric field.
    Relative(String, Value),
    /// Multiply a numeric field by a factor.
    Proportional(String, Value),
}

/// A resolved chain of `copy-from` inheritance.
///
/// `base` is the first ancestor in the chain, `current` is the final
/// definition with all operations applied.
#[derive(Debug, Clone)]
pub struct CopyFromChain {
    /// The chain of IDs from base to current (excluding current).
    pub chain: Vec<String>,
}

impl CopyFromTarget {
    /// Returns true if this definition is abstract.
    pub fn is_abstract(&self) -> bool {
        self.abstract_.unwrap_or(false)
    }

    /// Returns true if this definition inherits from another.
    pub fn has_copy_from(&self) -> bool {
        self.copy_from.is_some()
    }

    /// Collect all `CopyFromOp`s from this definition's extend/delete/relative/proportional.
    pub fn collect_ops(&self) -> Vec<CopyFromOp> {
        let mut ops = Vec::new();
        if let Some(ref extend) = self.extend {
            if let Some(obj) = extend.as_object() {
                for (key, val) in obj {
                    ops.push(CopyFromOp::Extend(key.clone(), val.clone()));
                }
            }
        }
        if let Some(ref delete) = self.delete {
            if let Some(obj) = delete.as_object() {
                for (key, val) in obj {
                    ops.push(CopyFromOp::Delete(key.clone(), val.clone()));
                }
            }
        }
        if let Some(ref relative) = self.relative {
            if let Some(obj) = relative.as_object() {
                for (key, val) in obj {
                    ops.push(CopyFromOp::Relative(key.clone(), val.clone()));
                }
            }
        }
        if let Some(ref proportional) = self.proportional {
            if let Some(obj) = proportional.as_object() {
                for (key, val) in obj {
                    ops.push(CopyFromOp::Proportional(key.clone(), val.clone()));
                }
            }
        }
        ops
    }
}
