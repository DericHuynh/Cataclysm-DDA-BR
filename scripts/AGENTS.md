# Scripts DOX

## Purpose
Owns development, dependency-boundary validation, content-baseline and data-extraction scripts that operate on the JSON data and Rust struct surface.

## Ownership
- Scripts here are run by hand (or in CI) outside of `cargo`. They read from `data/` and `crates/cdda_defs_raw/src/raw_defs/`.
- `scripts/README.md` is the user-facing quick reference for this subtree.

## Local Contracts
- Shell scripts (`*.sh`) and Python scripts (`*.py`) live side by side. Each script is self-contained and takes its arguments positionally — see `scripts/README.md` for the canonical usage.
- Scripts are read-only against the repo: they print diagnostics, never mutate data or Rust files in place.
- Python scripts assume Python 3.10+ and the standard library only (no third-party deps).

## Work Guidance
- Add new scripts under this folder and document them in `scripts/README.md` in the same change.
- Prefer shelling out to `cargo run -p cdda_cli` over writing Rust helpers here — CLI subcommands are the supported Rust entry points.
- Keep scripts fast; they are part of the inner dev loop.

## Verification
- No automated verification of the scripts themselves is configured. Smoke-test any new script by hand and document the example invocation in `scripts/README.md`.

## Child DOX Index
No durable sub-boundaries; scripts are flat, peer-level utilities.
