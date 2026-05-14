//! # cdda-cli — CLI tools for CDDA mod development
//!
//! ## Commands
//!
//! - `schema`         — Generate static JSON Schema files for all definition types
//! - `gen-schemas`    — Generate dynamic schemas for core + mods (with autocomplete)
//! - `validate <dir>` — Validate all JSON files in a directory against schemas
//! - `stats <dir>`    — Print definition statistics for a data directory
//! - `check <dir>`    — Full load check: load + resolve + validate all definitions
//! - `ablation <baseline> <dirs>...` — Test removing mods to isolate load errors

use std::path::PathBuf;
use std::time::Instant;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cdda-cli", about = "CDDA data tooling for mod developers")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generate static JSON Schema files for all definition types (no dynamic enums).
    Schema,
    /// Generate dynamic schemas for core and mods with autocomplete enums.
    /// Outputs to `schemas/<name>/` for each mod.
    GenSchemas {
        /// Path to the core data directory (e.g. data/core).
        #[arg(long, default_value = "data/core")]
        core: PathBuf,
        /// Mod directories as NAME=PATH pairs (e.g. Magiclysm=data/mods/Magiclysm).
        #[arg(short, long = "mod", value_parser = parse_mod_pair)]
        mods: Vec<ModEntry>,
    },
    /// Validate all JSON files in a directory against schemas
    Validate {
        /// Path to directory containing CDDA JSON data
        path: PathBuf,
    },
    /// Print definition count statistics for a data directory
    Stats {
        /// Path to directory containing CDDA JSON data
        path: PathBuf,
    },
    /// Full load check: load, resolve copy-from, validate all definitions
    Check {
        /// Path to directory containing CDDA JSON data
        path: PathBuf,
    },
    /// Load a baseline directory, then test adding/removing mods to find conflicts
    Ablation {
        /// Baseline data directory (e.g. data/core)
        baseline: PathBuf,
        /// Mod directories to test (loaded on top of baseline)
        mods: Vec<PathBuf>,
    },
}

/// A mod entry parsed from NAME=PATH CLI arguments.
#[derive(Clone, Debug)]
struct ModEntry {
    name: String,
    path: PathBuf,
}

fn parse_mod_pair(s: &str) -> Result<ModEntry, String> {
    let (name, path) = s
        .split_once('=')
        .ok_or_else(|| format!("expected NAME=PATH, got: {s}"))?;
    let p = PathBuf::from(path);
    if !p.exists() {
        return Err(format!("mod directory not found: {path}"));
    }
    Ok(ModEntry {
        name: name.to_string(),
        path: p,
    })
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(tracing::level_filters::LevelFilter::WARN.into())
                .from_env_lossy(),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();
    let start = Instant::now();

    match cli.command {
        Command::Schema => cmd_schema(),
        Command::GenSchemas { core, mods } => cmd_gen_schemas(&core, &mods),
        Command::Validate { path } => cmd_validate(&path),
        Command::Stats { path } => cmd_stats(&path),
        Command::Check { path } => cmd_check(&path),
        Command::Ablation { baseline, mods } => cmd_ablation(&baseline, &mods),
    }

    eprintln!("Elapsed: {:.2}s", start.elapsed().as_secs_f64());
}

// ── Dynamic schema generation (multi-mod) ────────────────────────────

fn cmd_gen_schemas(core: &PathBuf, mods: &[ModEntry]) {
    let out_base = get_default_schema_dir();
    eprintln!("Generating dynamic schemas in {:?}", out_base);

    // Generate core schema first (flags/IDs from core data only).
    eprintln!("  core ({:?}) ...", core);
    match cdda_data::schema_gen::generate_schemas_for_mod("core", &[core.clone()], &out_base) {
        Ok(()) => eprintln!("    -> schemas/core/"),
        Err(errors) => {
            for e in &errors {
                eprintln!("    ERROR: {e}");
            }
        }
    }

    // Generate one schema per mod (core + mod data so modders get full autocomplete).
    for entry in mods {
        eprintln!("  {} ({:?}) ...", entry.name, entry.path);
        match cdda_data::schema_gen::generate_schemas_for_mod(
            &entry.name,
            &[core.clone(), entry.path.clone()],
            &out_base,
        ) {
            Ok(()) => eprintln!("    -> schemas/{}/", entry.name),
            Err(errors) => {
                for e in &errors {
                    eprintln!("    ERROR: {e}");
                }
            }
        }
    }

    eprintln!("Done.");
}

// ── Static schema generation ─────────────────────────────────────────

fn cmd_schema() {
    let out_dir = get_default_schema_dir();
    eprintln!("Generating schemas in {:?}", out_dir);
    std::fs::create_dir_all(&out_dir).expect("Failed to create schema output directory");

    match cdda_data::schema::write_all_schemas(&out_dir) {
        Ok(()) => {
            let count = out_dir.read_dir().map(|d| d.count()).unwrap_or(0);
            eprintln!("Done. {} schema files written.", count);
        }
        Err(errors) => {
            for e in &errors {
                eprintln!("  Schema error: {}", e);
            }
            std::process::exit(1);
        }
    }
}

fn get_default_schema_dir() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop(); // crates/
    p.pop(); // workspace root
    p.join("data").join("schemas")
}

// ── Stats ────────────────────────────────────────────────────────────

fn cmd_stats(path: &PathBuf) {
    if !path.exists() {
        eprintln!("Error: directory not found: {:?}", path);
        std::process::exit(1);
    }

    let mut loader = cdda_data::loader::Loader::new(vec![path.clone()]);
    let raw_map = loader.ingest_all();

    eprintln!("Raw definitions by type:");
    let mut types: Vec<_> = raw_map.iter().map(|(k, v)| (k.clone(), v.len())).collect();
    types.sort_by(|a, b| b.1.cmp(&a.1));

    let total: usize = types.iter().map(|(_, c)| c).sum();
    for (type_name, count) in &types {
        eprintln!("  {:20} {}", type_name, count);
    }
    eprintln!("  {:20} {}", "TOTAL", total);
    eprintln!();

    match loader.load() {
        Ok(registry) => {
            eprintln!("Resolved definitions: {}", registry.total_count());
            eprintln!("Categories with data: {}", registry.category_count());
        }
        Err(errors) => {
            eprintln!("Load completed with {} errors:", errors.len());
            for (i, err) in errors.iter().enumerate().take(20) {
                eprintln!("  {:>3}. {:?}", i + 1, err);
            }
        }
    }
}

// ── Validate ─────────────────────────────────────────────────────────

fn cmd_validate(path: &PathBuf) {
    if !path.exists() {
        eprintln!("Error: directory not found: {:?}", path);
        std::process::exit(1);
    }

    let mut loader = cdda_data::loader::Loader::new(vec![path.clone()]);
    let raw_map = loader.ingest_all();

    let mut total_errors = 0;
    let mut total_defs = 0;

    for (_type_name, defs) in &raw_map {
        total_defs += defs.len();
    }

    eprintln!(
        "Validating {} raw definitions across {} types...",
        total_defs,
        raw_map.len()
    );

    match loader.load() {
        Ok(_registry) => {
            eprintln!("  All definitions loaded without errors.");
            eprintln!("  Schema validation: PASSED");
        }
        Err(errors) => {
            total_errors = errors.len();
            for (i, err) in errors.iter().enumerate() {
                eprintln!("  {:>3}. {:?}", i + 1, err);
            }
            eprintln!("  Schema validation: {} error(s)", total_errors);
        }
    }

    if total_errors > 0 {
        std::process::exit(1);
    }
}

// ── Check ────────────────────────────────────────────────────────────

fn cmd_check(path: &PathBuf) {
    if !path.exists() {
        eprintln!("Error: directory not found: {:?}", path);
        std::process::exit(1);
    }

    eprint!("Loading data from {:?} ... ", path);
    let mut loader = cdda_data::loader::Loader::new(vec![path.clone()]);

    match loader.load() {
        Ok(registry) => {
            let total = registry.total_count();
            let categories = registry.category_count();
            eprintln!("{} definitions across {} categories", total, categories);
        }
        Err(errors) => {
            eprintln!();
            eprintln!("LOAD FAILED with {} errors:", errors.len());
            for (i, err) in errors.iter().enumerate() {
                eprintln!("  {:>3}. {:?}", i + 1, err);
            }
            std::process::exit(1);
        }
    }
}

// ── Ablation testing ─────────────────────────────────────────────────

fn cmd_ablation(baseline: &PathBuf, mod_dirs: &[PathBuf]) {
    if !baseline.exists() {
        eprintln!("Error: baseline directory not found: {:?}", baseline);
        std::process::exit(1);
    }

    eprintln!("=== Ablation Test ===");
    eprintln!("Baseline: {:?}", baseline);
    eprintln!();
    eprint!("Loading baseline... ");

    let mut dirs: Vec<PathBuf> = vec![baseline.clone()];
    let mut errors: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    for mod_dir in mod_dirs {
        if !mod_dir.exists() {
            eprintln!("Warning: mod directory not found: {:?}", mod_dir);
            continue;
        }
        dirs.push(mod_dir.clone());
    }

    let mut loader = cdda_data::loader::Loader::new(dirs.clone());
    match loader.load() {
        Ok(registry) => {
            eprintln!("{} definitions", registry.total_count());
        }
        Err(load_errors) => {
            eprintln!("{} errors:", load_errors.len());
            for e in &load_errors {
                eprintln!("  {:?}", e);
            }
        }
    }

    eprintln!();

    if mod_dirs.len() > 1 {
        eprintln!("=== Ablation: removing one mod at a time ===");
        for mod_dir in mod_dirs {
            let without: Vec<PathBuf> = dirs.iter().filter(|d| *d != mod_dir).cloned().collect();
            eprint!(
                "  Without {:?} ... ",
                mod_dir.file_name().unwrap_or_default()
            );
            let mut l = cdda_data::loader::Loader::new(without);
            match l.load() {
                Ok(registry) => {
                    eprintln!("{} definitions", registry.total_count());
                }
                Err(e) => {
                    eprintln!("{} error(s)", e.len());
                    errors.insert(
                        format!(
                            "missing_{}",
                            mod_dir.file_name().unwrap_or_default().to_string_lossy()
                        ),
                        e.iter().map(|e| format!("{:?}", e)).collect(),
                    );
                }
            }
        }
    }

    if !mod_dirs.is_empty() {
        eprintln!();
        eprintln!("=== Isolated mods (with baseline) ===");
        for mod_dir in mod_dirs {
            eprint!("  {:?} ... ", mod_dir.file_name().unwrap_or_default());
            let mut l = cdda_data::loader::Loader::new(vec![baseline.clone(), mod_dir.clone()]);
            match l.load() {
                Ok(registry) => {
                    eprintln!("{} definitions", registry.total_count());
                }
                Err(e) => {
                    eprintln!("{} error(s)", e.len());
                    errors.insert(
                        format!(
                            "isolated_{}",
                            mod_dir.file_name().unwrap_or_default().to_string_lossy()
                        ),
                        e.iter().map(|e| format!("{:?}", e)).collect(),
                    );
                }
            }
        }
    }

    if !errors.is_empty() {
        eprintln!();
        eprintln!("=== Error Summary ===");
        for (key, errs) in &errors {
            eprintln!("  {}: {} error(s)", key, errs.len());
            for err in errs {
                eprintln!("    {}", err);
            }
        }
        std::process::exit(1);
    }
}
