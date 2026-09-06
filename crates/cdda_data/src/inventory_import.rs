//! Conservative CDDA adapter for the native counted-item / crafting family.
//! Raw records survive inspection. A strict candidate never silently drops a field.
use crate::loader::Loader;
use cdda_catalog::inventory::*;
use cdda_core_types::core::units::{Time, Volume, Weight};
use cdda_defs_raw::raw_types::LocalizedString;
use serde::Deserialize;
use serde_json::Value;
use std::{collections::BTreeMap, path::PathBuf, sync::Arc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Support {
    Supported,
    PreservedUnimplemented,
    Rejected,
}
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub source: PathBuf,
    pub mod_id: String,
    pub definition: String,
    /// JSON pointer into the resolved record, not a byte offset in the source.
    pub path: String,
    pub support: Support,
    pub reason: String,
}
#[derive(Debug, Clone)]
pub struct SourceDocument {
    pub path: PathBuf,
    pub mod_id: String,
    pub values: Vec<Value>,
}
#[derive(Debug)]
pub struct InventoryImport {
    /// Ordered, original semantic documents (including unknown fields).
    pub documents: Vec<SourceDocument>,
    pub resolved: BTreeMap<(String, String), Value>,
    pub diagnostics: Vec<Diagnostic>,
    candidate: InventoryCatalog,
}
impl InventoryImport {
    pub fn strict_catalog(&self) -> Result<InventoryCatalog, &[Diagnostic]> {
        if self
            .diagnostics
            .iter()
            .any(|d| d.support != Support::Supported)
        {
            Err(&self.diagnostics)
        } else {
            Ok(self.candidate.clone())
        }
    }
}

const META: &[&str] = &[
    "type",
    "id",
    "abstract",
    "abstract_",
    "copy-from",
    "$schema",
    "//",
];
const ITEM: &[&str] = &[
    "name",
    "description",
    "category",
    "volume",
    "weight",
    "qualities",
    "pocket_data",
];
const RECIPE: &[&str] = &[
    "result",
    "result_mult",
    "id_suffix",
    "variant",
    "time",
    "category",
    "subcategory",
    "components",
    "qualities",
];

/// Documents are supplied in dependency/mod order; the importer does no I/O.
pub fn import_inventory(documents: Vec<SourceDocument>) -> InventoryImport {
    let mut loader = Loader::new(Vec::new());
    loader.ingest_values(
        documents
            .iter()
            .map(|d| (d.path.clone(), d.values.clone()))
            .collect(),
    );
    let mut result = InventoryImport {
        documents,
        resolved: BTreeMap::new(),
        diagnostics: Vec::new(),
        candidate: InventoryCatalog::default(),
    };
    let mut provenance = BTreeMap::new();
    for document in &result.documents {
        if provenance
            .insert(document.path.clone(), document.mod_id.clone())
            .is_some()
        {
            result.diagnostics.push(Diagnostic {
                source: document.path.clone(),
                mod_id: document.mod_id.clone(),
                definition: String::new(),
                path: String::new(),
                support: Support::Rejected,
                reason: "Source paths must uniquely identify documents across mods".into(),
            });
        }
        for (i, value) in document.values.iter().enumerate() {
            let kind = value.get("type").and_then(Value::as_str);
            let issue = if !value.is_object() || kind.is_none() {
                Some((
                    Support::Rejected,
                    "Definition requires an object with a type",
                ))
            } else if ["id", "result", "abstract"]
                .iter()
                .all(|key| value.get(key).is_none())
            {
                if matches!(kind, Some("ITEM" | "GENERIC" | "recipe")) {
                    Some((
                        Support::Rejected,
                        "Native inventory definition requires a stable identity",
                    ))
                } else {
                    Some((Support::PreservedUnimplemented, "This content family's identity/behavior has no native inventory translator"))
                }
            } else {
                None
            };
            if let Some((support, reason)) = issue {
                result.diagnostics.push(Diagnostic {
                    source: document.path.clone(),
                    mod_id: document.mod_id.clone(),
                    definition: kind.unwrap_or_default().into(),
                    path: format!("/{i}"),
                    support,
                    reason: reason.into(),
                });
            }
        }
    }

    let mut families: Vec<_> = loader.raw_by_type().keys().cloned().collect();
    families.sort();
    for family in families {
        let sources = loader.keyed_sources(&family);
        let (records, failures) = loader.resolve_type_raw(&family);
        let diagnostic = |key: &str, path: String, support: Support, reason: String| {
            let source = sources
                .get(key)
                .map(|r| r.source.clone())
                .unwrap_or_default();
            let mod_id = provenance.get(&source).cloned().unwrap_or_default();
            Diagnostic {
                source,
                mod_id,
                definition: format!("{family}:{key}"),
                path,
                support,
                reason,
            }
        };
        for key in failures {
            result.diagnostics.push(diagnostic(
                &key,
                String::new(),
                Support::Rejected,
                "Unresolved copy-from chain".into(),
            ));
        }
        for (key, value) in records {
            result
                .resolved
                .insert((family.clone(), key.clone()), value.clone());
            if sources.get(&key).is_some_and(|raw| {
                raw.value.get("abstract").is_some()
                    && raw.value.get("id").is_none()
                    && raw.value.get("result").is_none()
            }) {
                continue;
            }
            if family == "ITEM"
                && !matches!(
                    value.get("type").and_then(Value::as_str),
                    Some("ITEM" | "GENERIC")
                )
            {
                result.diagnostics.push(diagnostic(
                    &key,
                    "/type".into(),
                    Support::PreservedUnimplemented,
                    "Specialized item subtype is outside the counted-item family".into(),
                ));
            }
            let supported = match family.as_str() {
                "ITEM" => ITEM,
                "recipe" => RECIPE,
                _ => {
                    result.diagnostics.push(diagnostic(
                        &key,
                        "/type".into(),
                        Support::PreservedUnimplemented,
                        "Content family has no native inventory translator".into(),
                    ));
                    continue;
                }
            };
            let before = result.diagnostics.len();
            for field in value.as_object().into_iter().flat_map(|o| o.keys()) {
                if !META.contains(&field.as_str()) && !supported.contains(&field.as_str()) {
                    result.diagnostics.push(diagnostic(
                        &key,
                        format!("/{}", field.replace('~', "~0").replace('/', "~1")),
                        Support::PreservedUnimplemented,
                        "Field has no native behavior in this family".into(),
                    ));
                }
            }
            let translated = if family == "ITEM" {
                translate_item(&key, value).map(|item| {
                    result
                        .candidate
                        .items
                        .insert(item.key.clone(), Arc::new(item));
                })
            } else {
                translate_recipe(&key, value).map(|recipe| {
                    result
                        .candidate
                        .recipes
                        .insert(recipe.key.clone(), Arc::new(recipe));
                })
            };
            match translated {
                Err(reason) => result.diagnostics.push(diagnostic(
                    &key,
                    String::new(),
                    Support::Rejected,
                    reason,
                )),
                Ok(()) if before == result.diagnostics.len() => {
                    result.diagnostics.push(diagnostic(
                        &key,
                        String::new(),
                        Support::Supported,
                        "Normalized native inventory definition".into(),
                    ))
                }
                _ => {}
            }
        }
    }
    // Index provenance once. Reporting a large corpus must not scan all previous
    // diagnostics (and allocate formatted keys) for every missing reference.
    let owners: BTreeMap<_, _> = result
        .diagnostics
        .iter()
        .map(|d| (d.definition.clone(), (d.source.clone(), d.mod_id.clone())))
        .collect();
    // Report every missing reference at its recipe field, with source provenance.
    for recipe in result.candidate.recipes.values() {
        let mut refs = vec![("/result".to_string(), &recipe.result)];
        for (i, slot) in recipe.ingredients.iter().enumerate() {
            for (j, ingredient) in slot.iter().enumerate() {
                refs.push((format!("/components/{i}/{j}"), &ingredient.item));
            }
        }
        for (path, key) in refs {
            if !result.candidate.items.contains_key(key) {
                let definition = format!("recipe:{}", recipe.key.0);
                let owner = owners.get(&definition);
                result.diagnostics.push(Diagnostic {
                    source: owner.map(|(source, _)| source.clone()).unwrap_or_default(),
                    mod_id: owner.map(|(_, mod_id)| mod_id.clone()).unwrap_or_default(),
                    definition,
                    path,
                    support: Support::Rejected,
                    reason: format!("Unknown item {}", key.0),
                });
            }
        }
    }
    result
}

#[derive(Deserialize)]
struct ItemWire {
    name: Option<LocalizedString>,
    description: Option<LocalizedString>,
    category: Option<String>,
    volume: Option<Volume>,
    weight: Option<Weight>,
    #[serde(default)]
    qualities: Vec<(String, i32)>,
    #[serde(default)]
    pocket_data: Vec<PocketWire>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PocketWire {
    pocket_type: String,
    #[serde(alias = "max_volume")]
    max_contains_volume: Volume,
    #[serde(alias = "max_weight")]
    max_contains_weight: Weight,
}
fn translate_item(key: &str, value: Value) -> Result<ItemDefinition, String> {
    let wire: ItemWire = serde_json::from_value(value).map_err(|e| e.to_string())?;
    let pockets = wire
        .pocket_data
        .into_iter()
        .map(|p| {
            if p.pocket_type != "CONTAINER" {
                return Err("Only unrestricted CONTAINER pockets are implemented".to_string());
            }
            Ok(PocketDefinition {
                volume_ml: u32::try_from(p.max_contains_volume.0)
                    .map_err(|_| "Pocket volume overflow")?,
                weight_g: u32::try_from(p.max_contains_weight.0)
                    .map_err(|_| "Pocket weight overflow")?,
            })
        })
        .collect::<Result<_, String>>()?;
    Ok(ItemDefinition {
        key: ItemKey(key.into()),
        name: wire
            .name
            .map(|n| n.to_string())
            .unwrap_or_else(|| key.into()),
        description: wire.description.map(|d| d.to_string()).unwrap_or_default(),
        category: wire.category.unwrap_or_default(),
        volume_ml: u32::try_from(wire.volume.map(|v| v.0).unwrap_or(0))
            .map_err(|_| "Item volume overflow")?,
        weight_g: u32::try_from(wire.weight.map(|w| w.0).unwrap_or(0))
            .map_err(|_| "Item weight overflow")?,
        qualities: wire.qualities,
        pockets,
    })
}
#[derive(Deserialize)]
struct RecipeWire {
    result: String,
    result_mult: Option<u32>,
    time: Option<Time>,
    category: Option<String>,
    subcategory: Option<String>,
    #[serde(default)]
    components: Vec<Vec<(String, u32)>>,
    #[serde(default)]
    qualities: Vec<QualityWire>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct QualityWire {
    id: String,
    level: u32,
}
fn translate_recipe(key: &str, value: Value) -> Result<RecipeDefinition, String> {
    let wire: RecipeWire = serde_json::from_value(value).map_err(|e| e.to_string())?;
    let result_count = wire.result_mult.unwrap_or(1);
    let seconds = wire.time.map(|t| t.0).unwrap_or(1);
    if seconds < 0
        || result_count == 0
        || wire
            .components
            .iter()
            .any(|s| s.is_empty() || s.iter().any(|(_, n)| *n == 0))
    {
        return Err("Invalid recipe count, time, or ingredient slot".into());
    }
    let work_ap = seconds
        .checked_mul(100)
        .and_then(|v| i32::try_from(v.max(100)).ok())
        .ok_or("Recipe work overflow")?;
    Ok(RecipeDefinition {
        key: RecipeKey(key.into()),
        result: ItemKey(wire.result),
        result_count,
        work_ap,
        category: wire.category.unwrap_or_default(),
        subcategory: wire.subcategory.unwrap_or_default(),
        ingredients: wire
            .components
            .into_iter()
            .map(|s| {
                s.into_iter()
                    .map(|(id, count)| Ingredient {
                        item: ItemKey(id),
                        count,
                    })
                    .collect()
            })
            .collect(),
        qualities: wire
            .qualities
            .into_iter()
            .map(|q| (q.id, q.level))
            .collect(),
    })
}
