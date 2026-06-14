# .cargo DOX

## Purpose
Owns Cargo configuration that affects the workspace build environment.

## Ownership
- `.cargo/config.toml` is the only file in this subtree.
- Tooling that needs to read the build env (lint runners, CI scripts) may inspect this folder but must not modify it casually.

## Local Contracts
- Dev builds use dynamic linking via `clang` + `lld` on Linux and `zld` on macOS to speed up incremental builds. Never enable dynamic linking on Windows or in release profiles.
- The current config pins the linker only; it does not set codegen or feature flags. Keep it minimal.
- Any new target-specific block must be commented with the rationale and the platform it applies to.

## Work Guidance
- Add a new target block only when a new platform needs linker tweaks; prefer upstream Cargo environment vars otherwise.
- Profile-level flags live in the root `Cargo.toml`, not here.

## Verification
- `cargo build` on each supported target confirms the linker is wired correctly. No automated CI matrix is configured for this folder yet.

## Child DOX Index
No durable sub-boundaries.
