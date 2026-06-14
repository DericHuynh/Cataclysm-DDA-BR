use cdda_core_types::core::id::DefId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// An emit definition from JSON type `"emit"`.
///
/// Defines an emission of a field (gas, smoke, etc.) from a source.
/// Emissions can be produced by monsters, vehicles, or other game objects.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EmitDef {
    /// Unique identifier (e.g. "emit_shadow", "emit_smoke_plume").
    pub id: DefId<EmitDef>,

    /// Field type emitted (e.g. "fd_shadow", "fd_smoke", "fd_fire").
    pub field: String,

    /// Intensity of the emitted field.
    #[serde(default)]
    pub intensity: Option<u32>,

    /// Quantity of field emitted per tick.
    #[serde(default)]
    pub qty: Option<u32>,

    /// Percentage chance of emission occurring each tick.
    #[serde(default)]
    pub chance: Option<u32>,
}
