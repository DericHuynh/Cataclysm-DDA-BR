//! Read-only headless capability report. Positional directories are ordered mods.
use cdda_data::inventory_import::{import_inventory, SourceDocument, Support};
use std::{collections::BTreeMap, path::Path};
fn read(
    path: &Path,
    mod_id: &str,
    docs: &mut Vec<SourceDocument>,
) -> Result<(), Box<dyn std::error::Error>> {
    if path.is_dir() {
        let mut paths = std::fs::read_dir(path)?
            .map(|e| e.map(|e| e.path()))
            .collect::<Result<Vec<_>, _>>()?;
        paths.sort();
        for path in paths {
            read(&path, mod_id, docs)?;
        }
    } else if path.extension().is_some_and(|s| s == "json") {
        let value: serde_json::Value = serde_json::from_slice(&std::fs::read(path)?)?;
        docs.push(SourceDocument {
            path: path.into(),
            mod_id: mod_id.into(),
            values: match value {
                serde_json::Value::Array(a) => a,
                value => vec![value],
            },
        });
    }
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let paths: Vec<_> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        return Err("usage: inventory_compat <core directory or file> [mod directory ...]".into());
    }
    let mut docs = Vec::new();
    for (i, path) in paths.iter().enumerate() {
        read(Path::new(path), &format!("{i}:{path}"), &mut docs)?;
    }
    let report = import_inventory(docs);
    let mut counts = BTreeMap::<String, usize>::new();
    for diagnostic in &report.diagnostics {
        *counts
            .entry(format!("{:?}", diagnostic.support))
            .or_default() += 1;
    }
    println!(
        "documents={} resolved_definitions={} diagnostics={counts:?} strict_playable={}",
        report.documents.len(),
        report.resolved.len(),
        report.strict_catalog().is_ok()
    );
    for diagnostic in report
        .diagnostics
        .iter()
        .filter(|d| d.support != Support::Supported)
        .take(20)
    {
        println!(
            "{} {} {} {}: {}",
            diagnostic.source.display(),
            diagnostic.mod_id,
            diagnostic.definition,
            diagnostic.path,
            diagnostic.reason
        );
    }
    Ok(())
}
