use crate::raw_types::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A jmath function definition from JSON type `"jmath_function"`.
///
/// Defines a custom mathematical function that can be used in JSON math expressions.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct JmathFunctionDef {
    /// Unique identifier (e.g. "scaling_factor", "dew_point").
    pub id: DefId<JmathFunctionDef>,

    /// Number of arguments the function takes.
    #[serde(default)]
    pub num_args: Option<i32>,

    /// The return expression (a string containing a math expression).
    pub r#return: Option<String>,
}
