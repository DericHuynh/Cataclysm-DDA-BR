# cdda_noise DOX

## Purpose
- Stateless 3D simplex noise primitives; bit-identical port of CDDA master's `simplexnoise.cpp` (Ken Perlin reference).
- Feeds terrain generation in `cdda_overmap_gen` (forest, lake, ocean, floodplain steps).

## Ownership
- All public noise functions live in `src/lib.rs` (the crate's only source file).
- Overmap generation systems (Bevy ECS) live in `cdda_overmap_gen`; this crate is intentionally Bevy-free.
- `static PERM` in `src/lib.rs` is the 256-entry reference permutation duplicated to 512 entries — it is the determinism anchor.

## Local Contracts
- **Zero dependencies.** `Cargo.toml` has an empty `[dependencies]` block — no Bevy, no `std::collections`, only `f32` math over a const table.
- **Bit-identical to `simplexnoise.cpp`.** `F3 = 1/3`, `G3 = 1/6`, the 12-entry `GRAD3`, and `PERM` must not change without updating the C++ reference. Any change here is a terrain-visible regression.
- **Public API** (in `src/lib.rs`):
  - `raw_noise_3d(x, y, z) -> f32` — base simplex sample.
  - `octave_noise_3d(octaves, persistence, scale, x, y, z) -> f32` — fBm; doubles frequency, scales amplitude by `persistence` per octave.
  - `scaled_octave_noise_3d(oct, per, sc, lo, hi, x, y, z) -> f32` — octave result remapped to `[lo, hi]`.
  - `forest_noise_at(x: i32, y: i32, seed: u32) -> f32` — output in `[0, 1]`.
  - `lake_noise_at(x: i32, y: i32, seed: u32) -> f32` — output in `[0, 1]`.
  - `ocean_noise_at(x, y, seed)` — delegates to `lake_noise_at`; alias kept for C++ naming parity.
  - `floodplain_noise_at(x: i32, y: i32, seed: u32) -> f32` — output in `[0, 1]`.
- **Seed handling.** The private `seed_z` LCG-hashes any `u32` into `[0, 10000)` to keep the z-coordinate in safe `f32` range (prevents `fastfloor` overflow on large seeds). Treat this hash as part of the determinism contract.
- **Range invariants.** `forest_*`, `lake_*`, `floodplain_*` are non-negative and bounded by `1.0` by construction (squared/cubed/quartic outputs of `scaled_octave_noise_3d` in `[0, 1]`); the test module asserts this.
- **Crate-private items** (`fastfloor`, `dot3`, `perm`, `grad_idx`, `seed_z`, `F3`, `G3`, `GRAD3`, `PERM`) must not be re-exported.

## Work Guidance
- Add a new terrain helper by composing `scaled_octave_noise_3d` with algebraic transforms — do not fork the simplex core.
- Keep function names and signatures aligned with CDDA's C++ helpers; callers in `cdda_overmap_gen/src/steps/{forests,lakes,oceans,mongroups,swamps}.rs` import the named functions directly.
- Do not introduce Bevy types. If state or scheduling is needed, add it in `cdda_overmap_gen`.
- Coordinate any signature change with `cdda_overmap_gen` first — it is the sole consumer.

## Verification
- `cargo check -p cdda_noise` for compile sanity (no Bevy, no extra features).
- `cargo nextest run -p cdda_noise` (fall back to `cargo test -p cdda_noise`) — runs the six tests in the `tests` module of `src/lib.rs`: determinism of `raw_noise_3d`, range of forest/lake/floodplain, `ocean == lake`, and seed sensitivity.
- After any change to `PERM`, the simplex core, or `seed_z`, regenerate a known overmap in `cdda_overmap_gen` and diff against a baseline.

## Child DOX Index
(none — single-file crate)
