# CDDA-RS Data Definitions — Stage 1 Checklist

> **Status:** All 15 core def files + supporting types are created as Rust structs.
> See verification results below for fields still missing from each struct.
> Last verified: $(date +%Y-%m-%d)

---

## Overview

| Crate area | Files | Status |
|------------|-------|--------|
| `types/` | `localized.rs`, `id.rs`, `copy_from.rs`, `mod.rs` | ✅ Complete |
| `defs/` | 19 def modules (15 core + 4 cross-ref stubs) | ✅ Created |
| `registry.rs` | `DefRegistry` with all 27 registries | ✅ Created |
| `loader.rs` | Two-pass JSON loader skeleton | ✅ Created |
| `resolve.rs` | `copy-from` resolver with extend/delete/relative/proportional | ✅ Created |
| `mod_layer.rs` | Mod layering | ❌ Not yet |
| `lib.rs` | Crate root | ❌ Not yet |

---

## Stage 1: Core Defs — Verification Results

Each row shows the Rust struct, how many JSON samples were checked, the number of
Rust fields currently defined, and the **top missing fields by frequency**.

### `ItemDef` (10359 samples, 23 fields)
| Missing field | Frequency |
|---|---|
| `copy-from` | 48% |
| `looks_like` | 30% |
| `use_action` | 15% |
| `armor` | 13% |
| `to_hit` | 10% |
| `material_thickness` | 10% |
| `fun` / `calories` / `vitamins` / `comestible_type` | ~9% each |
| `warmth` | 9% |
| `spoils_in` | 6% |
| `ammo_type` | 6% |
| `quench` | 5% |
| `container` | 5% |
| `charges` / `initial_charges` / `max_charges` | common |
| `melee_damage` | (field exists but needs sub-fields) |
| `pocket_data` | (field exists) |

### `MonsterDef` (1197 samples, 35 fields)
| Missing field | Frequency |
|---|---|
| `copy-from` | 68% |
| `dodge` | 29% |
| `dissect` | 19% |
| `bleed_rate` | 19% |
| `reproduction` | 18% |
| `death_function` | 18% |
| `diff` | 17% |
| `zombify_into` | 16% |
| `anger_triggers` | 12% |
| `fear_triggers` | 12% |
| `looks_like` | 11% |
| `proportional` | 10% |

### `TerrainDef` (1235 samples, 20 fields)
| Missing field | Frequency |
|---|---|
| `coverage` | 29% |
| `copy-from` | 28% |
| `examine_action` | 24% |
| `rotates_to` | 12% |
| `transforms_into` | 8% |
| `open` / `close` | ~8% |
| `shoot` | 7% |

### `FurnitureDef` (686 samples, 18 fields)
| Missing field | Frequency |
|---|---|
| `max_volume` | 16% |
| `item` (spawned item) | 15% |
| `bgcolor` | 11% |
| `connect_groups` | 6% |
| `comfort` | 6% |
| `copy-from` | 6% |

### `RecipeDef` (5694 samples, 22 fields)
| Missing field | Frequency |
|---|---|
| `proficiencies` | 48% |
| `batch_time_factors` | 21% |
| `copy-from` | 17% |
| `id_suffix` | 14% |
| `byproducts` | 11% |
| `never_learn` | 10% |
| `construction_blueprint` | 9% |

### `ItemGroupDef` (5366 samples, 11 fields)
| Missing field | Frequency |
|---|---|
| `container-item` | 25% |
| `on_overflow` | 3% |

### `MapgenPaletteDef` (505 samples, 7 fields)
| Missing field | Frequency |
|---|---|
| `toilets` | 20% |
| `nested` | 13% |
| `vendingmachines` | 9% |
| `vehicles` | 5% |
| `item` | 5% |
| `monster` | 5% |
| `liquids` | 3% |

### Other defs
See per-script output for `effect_type`, `bionic`, `mutation`, `field_type`,
`vehicle_part`, `overmap_terrain`, `scenario`, `material`, `skill`, `trap`.

---

## Critical Next Actions

1. ❌ **Add `copy-from` handling** → All defs need `copy-from`, `extend`, `delete`,
   `relative`, `proportional`, `abstract` fields for the resolver to work with.
2. ❌ **Fill high-frequency missing fields** per the table above (especially ItemDef
   food/comestible fields, MonsterDef dodge/death_function, TerrainDef coverage).
3. ❌ **Create `lib.rs`** with module declarations and re-exports.
4. ❌ **Create `mod_layer.rs`** for mod stacking.
5. ❌ **Add `serde_json::Value` catch-all** to each def to preserve unknown fields
   during the copy-from resolution pass.
6. ❌ **Build + test** — verify compilation and run loader tests against real JSON.
