use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A hit range definition from JSON type `"hit_range"`.
///
/// Defines hit range values for combat calculations.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HitRangeDef {
    /// Array of good hit ranges (even values).
    #[serde(default)]
    pub even_good: Vec<i32>,
}
