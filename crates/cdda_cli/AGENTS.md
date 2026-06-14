# cdda_cli DOX

## Purpose
Owns CLI tools for schema generation, validation, and ablation testing.

## Ownership
- CLI commands and command-specific implementation live in this crate.
- Shared schema and data-loading behavior remains in `cdda_data`.

## Local Contracts
- CLI commands should be deterministic and suitable for developer validation workflows.
- CLI output should be stable enough for scripts and tests.

## Work Guidance
- Keep CLI code focused on command orchestration.
- Reuse data and overmap crates instead of duplicating loading logic.

## Verification
- Run `cargo check -p cdda_cli`.
- Run the relevant CLI command after changing command behavior.

## Child DOX Index
