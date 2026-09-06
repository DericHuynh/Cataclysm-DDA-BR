# Docs DOX

## Purpose
Owns project documentation, architecture notes, and durable design records.

## Ownership
- Root-level architecture documents (`CURRENT_ARCHITECTURE.md`, `TARGET_ARCHITECTURE.md`) live at the repository root and are indexed from the root `AGENTS.md`.
- Crate-specific documentation lives with its owning crate under `crates/<crate>/AGENTS.md`.
- User-requested durable design records that are project-wide live here.

## Local Contracts
- Keep architecture docs consistent with the current crate layout and DOX hierarchy.
- Prefer concise operational guidance over historical narrative.
- Markdown only. Filenames are kebab-case where multi-word.

## Work Guidance
- Update this subtree when adding or revising durable design records.
- Link from root or crate docs when a document establishes an important contract.
- Promote recurring design decisions into the relevant crate's `AGENTS.md` so they live next to the code that implements them.

## Verification
- No automated documentation verification is configured yet.

## Child DOX Index
No durable sub-boundaries.

- `ecs-compatibility-baseline.md` — Source/content hashes, ownership/reference audit and native-family capability report.
- `ecs-modularity-and-json-compatibility-plan.md` — Staged ECS ownership, catalog/import separation, reusable UI, and original-CDDA content compatibility plan; includes source evidence and acceptance gates.

- `master-semantics-audit.md` — Player-visible comparison with the pinned sibling master source; confirmed mismatches, limited aligned cases and continuation gate.
