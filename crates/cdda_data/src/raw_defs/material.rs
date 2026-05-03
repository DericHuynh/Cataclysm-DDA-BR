use crate::raw_defs::cdda_types::{ChipResist, DamageResistance, LocalizedText, RawValue};
use crate::raw_types::{DefId, LocalizedString};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A material definition from JSON type `"material"`.
///
/// Defines material properties (e.g. "wood", "steel", "flesh", "glass").
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MaterialDef {
    /// Unique identifier (e.g. "wood", "steel", "flesh").
    pub id: DefId<MaterialDef>,

    /// Display name.
    pub name: LocalizedString,

    /// Type of material (solid, liquid, gas, etc).
    #[serde(default)]
    pub r#type: Option<String>,

    /// Density in g/cm^3.
    #[serde(default)]
    pub density: Option<f64>,

    /// Specific heat capacity.
    #[serde(default)]
    pub specific_heat_liquid: Option<f64>,
    #[serde(default)]
    pub specific_heat_solid: Option<f64>,

    /// Latent heat (melting/vaporization).
    #[serde(default)]
    pub latent_heat: Option<u32>,

    /// Material flags.
    #[serde(default)]
    pub flags: Vec<String>,

    /// Fuel data (if this material can be used as fuel).
    /// This is the structured form; see also `fuel_data` for JSON flexible form.
    #[serde(default)]
    pub fuel: Option<MaterialFuel>,

    /// Repair difficulty.
    #[serde(default)]
    pub repair_difficulty: Option<u32>,

    /// Salvage data.
    #[serde(default)]
    pub salvage: Option<MaterialSalvage>,

    /// Salvaged into item id.
    #[serde(default)]
    pub salvaged_into: Option<String>,

    /// Fuel data (object with energy and other fuel properties).
    /// Format: `{ "energy": "1000 kJ" }` or more complex.
    #[serde(default)]
    pub fuel_data: Option<HashMap<String, RawValue>>,

    /// Burn products (what items are produced when burning).
    #[serde(default)]
    pub burn_products: Option<HashMap<String, RawValue>>,

    /// Burn data (per-intensity fuel/smoke/burn values).
    /// Format: `[{ "fuel": 0.1, "smoke": 2, "burn": 0.001 }, ...]`
    #[serde(default)]
    pub burn_data: Option<Vec<HashMap<String, RawValue>>>,

    /// Soft material flag.
    #[serde(default)]
    pub soft: Option<bool>,

    /// Repaired with item id.
    #[serde(default)]
    pub repaired_with: Option<String>,

    /// Conductive flag.
    #[serde(default)]
    pub conductive: Option<bool>,

    /// Bash damage verb — can be a plain string like `"gouged"`
    /// or an object like `{"ctxt": "verb", "str": "dented"}`.
    #[serde(default)]
    pub bash_dmg_verb: Option<LocalizedText>,

    /// Cut damage verb — can be a plain string like `"gouged"`
    /// or an object like `{"ctxt": "verb", "str": "dented"}`.
    #[serde(default)]
    pub cut_dmg_verb: Option<LocalizedText>,

    /// Damage adjectives describing increasing damage states.
    /// Elements can be plain strings or objects with context/str keys.
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_dmg_adj")]
    pub dmg_adj: Option<Vec<String>>,

    /// Chip resistance — can be an integer or an object.
    #[serde(default)]
    pub chip_resist: Option<ChipResist>,

    /// Resist values by damage type, as a JSON object.
    /// Example: `{"bash": 4, "cut": 5, "acid": 10, "heat": 6, "bullet": 3}`
    #[serde(default)]
    pub resist: Option<DamageResistance>,

    /// copy-from parent material id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_from: Option<String>,
}

/// Custom deserializer for `dmg_adj` that accepts both plain strings
/// and objects like `{"ctxt": "...", "str": "..."}` in the array,
/// extracting just the `"str"` value from objects.
fn deserialize_dmg_adj<'de, D>(deserializer: D) -> Result<Option<Vec<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum DmgAdjElement {
        Plain(String),
        CtxtStr {
            #[serde(rename = "str")]
            str_val: String,
        },
    }

    let opt: Option<Vec<DmgAdjElement>> = Option::deserialize(deserializer)?;
    match opt {
        None => Ok(None),
        Some(elems) => {
            let strings: Vec<String> = elems
                .into_iter()
                .map(|e| match e {
                    DmgAdjElement::Plain(s) => s,
                    DmgAdjElement::CtxtStr { str_val } => str_val,
                })
                .collect();
            Ok(Some(strings))
        }
    }
}

/// Fuel properties for a material.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MaterialFuel {
    /// Energy per unit volume.
    #[serde(default)]
    pub energy: Option<f64>,
    /// Fuel type.
    #[serde(default)]
    pub type_: Option<String>,
    /// Companion fuel type.
    #[serde(default)]
    pub companion: Option<String>,
    /// Percentage of fuel that burns.
    #[serde(default)]
    pub burn: Option<f64>,
    /// Percentage of fuel per turn.
    #[serde(default)]
    pub burnt: Option<f64>,
    /// Whether fuel is perpetual.
    #[serde(default)]
    pub perpetual: Option<bool>,
}

/// Salvage yields from this material.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MaterialSalvage {
    /// Item produced.
    #[serde(default)]
    pub result: Option<String>,
    /// Amount per unit.
    #[serde(default)]
    pub amount: Option<u32>,
    /// Base volume consumed.
    #[serde(default)]
    pub volume_per_unit: Option<u32>,
    /// Skill required.
    #[serde(default)]
    pub skill: Option<String>,
}
