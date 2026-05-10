//! Shared item detail widget — renders a def entity's full properties into
//! a Bevy UI panel.
//!
//! Used by:
//! - `render/dev_spawn.rs`  (debug spawn panel's right-hand pane)
//! - `render/crafting.rs`   (crafting menu's right-hand item detail panel)
//! - `render/examine.rs`    (item examine overlay)
//!
//! Provides a `SystemParam` bundle and a function to spawn the detail tree.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::core::components::def::{
    AmmoData, ArmourData, BookData, ContainerData, FoodData, GunData, ItemCategory, ItemColor,
    ItemDescription, ItemMaterials, ItemPhase, ItemSymbol, ItemVolume, ItemWeight, MagazineData,
    Phase, ToolData, WeaponData,
};
use crate::core::components::item::ItemQualities;

// ---------------------------------------------------------------------------
// Bundled queries — a SystemParam to avoid Bevy's 16-query limit
// ---------------------------------------------------------------------------

#[derive(SystemParam)]
pub struct ItemDetailQueries<'w, 's> {
    pub item_descs: Query<'w, 's, &'static ItemDescription>,
    pub item_weights: Query<'w, 's, &'static ItemWeight>,
    pub item_volumes: Query<'w, 's, &'static ItemVolume>,
    pub item_symbols: Query<'w, 's, &'static ItemSymbol>,
    pub item_colors: Query<'w, 's, &'static ItemColor>,
    pub item_materials: Query<'w, 's, &'static ItemMaterials>,
    pub item_categories: Query<'w, 's, &'static ItemCategory>,
    pub item_phases: Query<'w, 's, &'static ItemPhase>,
    pub item_qualities: Query<'w, 's, &'static ItemQualities>,
    pub weapon_data: Query<'w, 's, &'static WeaponData>,
    pub gun_data: Query<'w, 's, &'static GunData>,
    pub ammo_data: Query<'w, 's, &'static AmmoData>,
    pub armour_data: Query<'w, 's, &'static ArmourData>,
    pub food_data: Query<'w, 's, &'static FoodData>,
    pub tool_data: Query<'w, 's, &'static ToolData>,
    pub container_data: Query<'w, 's, &'static ContainerData>,
    pub book_data: Query<'w, 's, &'static BookData>,
    pub magazine_data: Query<'w, 's, &'static MagazineData>,
}

// ---------------------------------------------------------------------------
// The widget function
// ---------------------------------------------------------------------------

/// Spawn the full item detail tree (properties, weapon, gun, ammo, armour,
/// food, tool, container, book, qualities) as children of `parent`.
///
/// * `parent` — the `UiBuilder` (from `with_children(|parent| …)`)
/// * `name` — display name for the title header (e.g. "M4A1")
/// * `id` — CDDA string ID (e.g. "m4a1")
/// * `def` — the item definition entity to read components from
/// * `q` — the `ItemDetailQueries` SystemParam
pub fn spawn_item_detail(
    parent: &mut ChildSpawnerCommands,
    name: &str,
    id: &str,
    def: Entity,
    q: &ItemDetailQueries,
) {
    // Name + ID header
    parent.spawn((
        Text::new(name.to_string()),
        TextFont {
            font_size: 22.0,
            ..default()
        },
        TextColor(Color::srgb(0.85, 0.60, 0.15)),
    ));
    parent.spawn((
        Text::new(format!("id: {}", id)),
        TextFont {
            font_size: 12.0,
            ..default()
        },
        TextColor(Color::srgb(0.50, 0.65, 0.50)),
    ));

    divider(parent);

    // Description
    if let Ok(desc) = q.item_descs.get(def) {
        if !desc.0.is_empty() {
            parent.spawn((
                Text::new(desc.0.clone()),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(Color::srgb(0.95, 0.95, 0.95)),
            ));
            divider(parent);
        }
    }

    // Basic properties
    section_header(parent, "Properties");

    let weight_g = q.item_weights.get(def).map(|w| w.0).unwrap_or(0);
    let volume_ml = q.item_volumes.get(def).map(|v| v.0).unwrap_or(0);

    let weight_str = if weight_g >= 1000 {
        format!("{:.2} kg", weight_g as f32 / 1000.0)
    } else {
        format!("{} g", weight_g)
    };
    let volume_str = if volume_ml >= 1000 {
        format!("{:.2} L", volume_ml as f32 / 1000.0)
    } else {
        format!("{} mL", volume_ml)
    };

    if let Ok(sym) = q.item_symbols.get(def) {
        stat_row(parent, "Symbol", &sym.0.to_string());
    }
    stat_row(parent, "Weight", &weight_str);
    stat_row(parent, "Volume", &volume_str);
    if let Ok(color) = q.item_colors.get(def) {
        stat_row(parent, "Color", &color.0);
    }
    if let Ok(cat) = q.item_categories.get(def) {
        stat_row(parent, "Category", &cat.0);
    }
    if let Ok(mats) = q.item_materials.get(def) {
        if !mats.0.is_empty() {
            stat_row(parent, "Materials", &mats.0.join(", "));
        }
    }
    if let Ok(phase) = q.item_phases.get(def) {
        let phase_str = match phase.0 {
            Phase::Solid => "Solid",
            Phase::Liquid => "Liquid",
            Phase::Gas => "Gas",
            Phase::Plasma => "Plasma",
        };
        stat_row(parent, "Phase", phase_str);
    }

    // Qualities
    if let Ok(quals) = q.item_qualities.get(def) {
        if !quals.0.is_empty() {
            divider(parent);
            section_header(parent, "Tool Qualities");
            for (quality_id, level) in &quals.0 {
                stat_row(parent, quality_id, &level.to_string());
            }
        }
    }

    // Weapon
    if let Ok(w) = q.weapon_data.get(def) {
        divider(parent);
        section_header(parent, "Melee");
        stat_row(
            parent,
            "Bash / Cut / Stab",
            &format!("{} / {} / {}", w.damage_bash, w.damage_cut, w.damage_stab),
        );
        stat_row(parent, "To-hit", &w.to_hit.to_string());
        stat_row(parent, "Moves/attack", &w.moves_per_attack.to_string());
        if w.reach > 1 {
            stat_row(parent, "Reach", &w.reach.to_string());
        }
        if !w.techniques.is_empty() {
            stat_row(parent, "Techniques", &w.techniques.join(", "));
        }
    }

    // Gun
    if let Ok(g) = q.gun_data.get(def) {
        divider(parent);
        section_header(parent, "Ranged");
        stat_row(parent, "Skill", &g.skill);
        stat_row(parent, "Ammo type", &g.ammo_type);
        stat_row(parent, "Clip", &g.clip_size.to_string());
        stat_row(parent, "Reload time", &g.reload_time.to_string());
        stat_row(parent, "Dispersion", &g.dispersion.to_string());
        if g.burst > 1 {
            stat_row(parent, "Burst", &g.burst.to_string());
        }
    }

    // Ammo
    if let Ok(a) = q.ammo_data.get(def) {
        divider(parent);
        section_header(parent, "Ammo");
        stat_row(parent, "Type", &a.ammo_type);
        stat_row(parent, "Damage", &a.damage.to_string());
        stat_row(parent, "Pierce", &a.pierce.to_string());
        stat_row(parent, "Range", &a.range.to_string());
        if a.count > 1 {
            stat_row(parent, "Count", &a.count.to_string());
        }
        if !a.effects.is_empty() {
            stat_row(parent, "Effects", &a.effects.join(", "));
        }
    }

    // Magazine
    if let Ok(m) = q.magazine_data.get(def) {
        divider(parent);
        section_header(parent, "Magazine");
        stat_row(parent, "Ammo type", &m.ammo_type);
        stat_row(parent, "Capacity", &m.capacity.to_string());
        stat_row(parent, "Reload time", &m.reload_time.to_string());
    }

    // Armour
    if let Ok(armour) = q.armour_data.get(def) {
        divider(parent);
        section_header(parent, "Armour");
        for (i, part) in armour.parts.iter().enumerate() {
            if i > 0 {
                parent.spawn(Node {
                    height: Val::Px(4.0),
                    ..default()
                });
            }
            let covers_str = if part.body_part.is_empty() {
                "?".to_string()
            } else {
                part.body_part.clone()
            };
            let layers_str = if part.layers.is_empty() {
                "NORMAL".to_string()
            } else {
                part.layers.join(", ")
            };
            stat_row(
                parent,
                "Covers",
                &format!("{} [{}]", covers_str, layers_str),
            );
            stat_row(
                parent,
                "Coverage",
                &format!("{}%  enc {}", part.coverage, part.encumbrance),
            );
            if !part.material.is_empty() {
                let mat_str: Vec<String> = part
                    .material
                    .iter()
                    .map(|(id, thick, cov)| {
                        if *cov < 100.0 {
                            format!("{} {:.1}mm ({}%)", id, thick, *cov as u32)
                        } else {
                            format!("{} {:.1}mm", id, thick)
                        }
                    })
                    .collect();
                stat_row(parent, "Material", &mat_str.join(" / "));
            }
            if !part.specifically_covers.is_empty() {
                stat_row(parent, "Specific", &part.specifically_covers.join(", "));
            }
        }
    }

    // Food
    if let Ok(food) = q.food_data.get(def) {
        divider(parent);
        section_header(parent, "Food");
        stat_row(parent, "Type", &food.comestible_type);
        stat_row(parent, "Calories", &food.calories.to_string());
        stat_row(parent, "Quench", &food.quench.to_string());
        stat_row(parent, "Fun", &food.fun.to_string());
        stat_row(parent, "Healthy", &food.healthy.to_string());
        if food.stim != 0 {
            stat_row(parent, "Stim", &food.stim.to_string());
        }
        if food.spoils_in > 0 {
            stat_row(parent, "Spoils in", &format!("{} turns", food.spoils_in));
        }
    }

    // Tool — only shown when at least one field is non-default.
    if let Ok(tool) = q.tool_data.get(def) {
        let has_tool_info =
            tool.max_charges != 0 || tool.ammo_type.is_some() || tool.revert_to.is_some();
        if has_tool_info {
            divider(parent);
            section_header(parent, "Tool");
            if tool.max_charges != 0 {
                stat_row(parent, "Max charges", &tool.max_charges.to_string());
                stat_row(parent, "Charges/use", &tool.charges_per_use.to_string());
            }
            if let Some(at) = &tool.ammo_type {
                stat_row(parent, "Ammo type", at);
            }
            if let Some(r) = &tool.revert_to {
                stat_row(parent, "Reverts to", r);
            }
        }
    }

    // Container
    if let Ok(cont) = q.container_data.get(def) {
        divider(parent);
        section_header(parent, "Pockets");
        for (idx, pocket) in cont.pockets.iter().enumerate() {
            if idx > 0 {
                parent.spawn(Node {
                    height: Val::Px(3.0),
                    ..default()
                });
            }
            let type_str = &pocket.pocket_type;
            let vol_str = if pocket.max_volume >= 1000 {
                format!("{:.2} L", pocket.max_volume as f32 / 1000.0)
            } else {
                format!("{} mL", pocket.max_volume)
            };
            let wt_str = if pocket.max_weight >= 1000 {
                format!("{:.2} kg", pocket.max_weight as f32 / 1000.0)
            } else {
                format!("{} g", pocket.max_weight)
            };
            let mut flags: Vec<&str> = Vec::new();
            if pocket.holster {
                flags.push("holster");
            }
            if pocket.ablative {
                flags.push("ablative");
            }
            if pocket.sealed {
                flags.push("sealed");
            }
            let header_str = if flags.is_empty() {
                format!("#{} {} — {} / {}", idx + 1, type_str, vol_str, wt_str)
            } else {
                format!(
                    "#{} {} — {} / {}  [{}]",
                    idx + 1,
                    type_str,
                    vol_str,
                    wt_str,
                    flags.join(", ")
                )
            };
            stat_row(parent, "Pocket", &header_str);
            if !pocket.description.is_empty() {
                stat_row(parent, "Desc", &pocket.description);
            }
            if !pocket.flag_restriction.is_empty() {
                stat_row(parent, "Flags", &pocket.flag_restriction.join(", "));
            }
        }
    }

    // Book
    if let Ok(book) = q.book_data.get(def) {
        divider(parent);
        section_header(parent, "Book");
        stat_row(parent, "Skill", &book.skill);
        stat_row(
            parent,
            "Levels",
            &format!("{} → {}", book.required_level, book.max_level),
        );
        stat_row(parent, "Fun", &book.fun.to_string());
        stat_row(parent, "Int req.", &book.intelligence.to_string());
        stat_row(parent, "Read time", &format!("{} turns", book.time));
        if !book.martial_art.is_empty() {
            stat_row(parent, "Martial art", &book.martial_art);
        }
        if book.chapters > 0 {
            stat_row(parent, "Chapters", &book.chapters.to_string());
        }
    }
}

// ---------------------------------------------------------------------------
// UI helper functions (private)
// ---------------------------------------------------------------------------

/// A thin horizontal line used as a visual separator between sections.
fn divider(parent: &mut ChildSpawnerCommands) {
    parent.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(1.0),
            margin: UiRect::vertical(Val::Px(4.0)),
            ..default()
        },
        BackgroundColor(Color::srgb(0.20, 0.20, 0.25)),
    ));
}

/// A section header label (e.g. "Properties", "Melee", "Book").
fn section_header(parent: &mut ChildSpawnerCommands, title: &str) {
    parent.spawn((
        Text::new(title.to_uppercase()),
        TextFont {
            font_size: 11.0,
            ..default()
        },
        TextColor(Color::srgb(0.50, 0.75, 0.90)),
    ));
}

/// A single stat row with a dim label and a bright value.
/// Avoids layout allocation per call.
fn stat_row(parent: &mut ChildSpawnerCommands, label: &str, value: &str) {
    parent
        .spawn((Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(6.0),
            ..default()
        },))
        .with_children(|row| {
            row.spawn((
                Text::new(format!("{}: ", label)),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(Color::srgb(0.55, 0.55, 0.55)),
                Node {
                    min_width: Val::Px(110.0),
                    ..default()
                },
            ));
            row.spawn((
                Text::new(value.to_string()),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(Color::srgb(0.95, 0.95, 0.95)),
            ));
        });
}
