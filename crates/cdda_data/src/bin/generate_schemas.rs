//! Generate JSON Schema files for all CDDA definition types.
//!
//! Usage: cargo run -p cdda_data --bin generate_schemas
//!
//! Writes one `.schema.json` file per type to `data/schemas/`.
//! These files give editor LSP tooling (VS Code, vim-json, etc.)
//! the ability to validate and autocomplete CDDA JSON files against
//! the authoritative Rust types.

use cdda_data::schema::write_all_schemas;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/schemas");

    eprintln!("Generating schemas in {:?}", out_dir);

    match write_all_schemas(&out_dir) {
        Ok(()) => {
            eprintln!(
                "Done. {} files written.",
                out_dir.read_dir().map(|d| d.count()).unwrap_or(0)
            );
        }
        Err(errors) => {
            for e in &errors {
                eprintln!("  Error: {}", e);
            }
            std::process::exit(1);
        }
    }
}
