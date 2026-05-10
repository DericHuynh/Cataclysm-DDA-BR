//! Debug spawn panel renderer.
//!
//! Left panel: scrollable, filtered item list.
//! Right panel: full item detail (all def components for the selected entry).
//!
//! Spawned on `OnEnter(Ctx::DevSpawnPanel)`, auto-despawned via `DespawnOnExit`.
//! The layout skeleton (title / body-row / filter / footer) is built once and
//! persists; only the *content* inside each container is rebuilt each frame.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_state::state_scoped::DespawnOnExit;

use crate::context::ctx::Ctx;
use crate::core::components::def::{
    AmmoData, ArmourData, BookData, ContainerData, FoodData, GunData, ItemCategory, ItemColor,
    ItemDescription, ItemMaterials, ItemPhase, ItemSymbol, ItemVolume, ItemWeight, MagazineData,
    Phase, ToolData, WeaponData,
};
use crate::core::components::item::ItemQualities;
use crate::worldgen::dev_spawn::DevSpawnFocus;

// ---------------------------------------------------------------------------
// Colours
// ---------------------------------------------------------------------------

const BG: Color = Color::srgb(0.04, 0.04, 0.06);
const PANEL_BG: Color = Color::srgb(0.07, 0.07, 0.10);
const HEADER_BG: Color = Color::srgb(0.10, 0.10, 0.14);
const ITEM_BG: Color = Color::srgb(0.06, 0.06, 0.09);
const ITEM_FOCUS_BG: Color = Color::srgb(0.12, 0.35, 0.55);
const ACCENT: Color = Color::srgb(0.30, 0.70, 1.00);
const ACCENT2: Color = Color::srgb(0.85, 0.60, 0.15);
const TEXT_BRIGHT: Color = Color::srgb(0.95, 0.95, 0.95);
const TEXT_DIM: Color = Color::srgb(0.55, 0.55, 0.55);
const TEXT_ID: Color = Color::srgb(0.50, 0.65, 0.50);
const LABEL: Color = Color::srgb(0.50, 0.75, 0.90);
const DIVIDER: Color = Color::srgb(0.20, 0.20, 0.25);
const FILTER_ACTIVE_BG: Color = Color::srgb(0.08, 0.18, 0.30);

// Rows visible at once in the list (controls centered-scroll window).
const VISIBLE_ROWS: usize = 22;

// ---------------------------------------------------------------------------
// Persistent-skeleton markers
// ---------------------------------------------------------------------------

#[derive(Component)]
pub(crate) struct SpawnListContainer;

#[derive(Component)]
pub(crate) struct SpawnTitleBar;

#[derive(Component)]
pub(crate) struct SpawnListPanel;

#[derive(Component)]
pub(crate) struct SpawnDetailPanel;

#[derive(Component)]
pub(crate) struct SpawnFilterBar;

// ---------------------------------------------------------------------------
// Bundled queries (avoids hitting Bevy's 16-param system limit)
// ---------------------------------------------------------------------------

#[derive(SystemParam)]
pub(crate) struct DefDetailQueries<'w, 's> {
    item_descs: Query<'w, 's, &'static ItemDescription>,
    item_weights: Query<'w, 's, &'static ItemWeight>,
    item_volumes: Query<'w, 's, &'static ItemVolume>,
    item_symbols: Query<'w, 's, &'static ItemSymbol>,
    item_colors: Query<'w, 's, &'static ItemColor>,
    item_materials: Query<'w, 's, &'static ItemMaterials>,
    item_categories: Query<'w, 's, &'static ItemCategory>,
    item_phases: Query<'w, 's, &'static ItemPhase>,
    item_qualities: Query<'w, 's, &'static ItemQualities>,
    weapon_data: Query<'w, 's, &'static WeaponData>,
    gun_data: Query<'w, 's, &'static GunData>,
    ammo_data: Query<'w, 's, &'static AmmoData>,
    armour_data: Query<'w, 's, &'static ArmourData>,
    food_data: Query<'w, 's, &'static FoodData>,
    tool_data: Query<'w, 's, &'static ToolData>,
    container_data: Query<'w, 's, &'static ContainerData>,
    book_data: Query<'w, 's, &'static BookData>,
    magazine_data: Query<'w, 's, &'static MagazineData>,
}

// ---------------------------------------------------------------------------
// Spawn (OnEnter) — full layout skeleton, built once
// ---------------------------------------------------------------------------

pub fn spawn_dev_spawn_panel(mut commands: Commands) {
    commands
        .spawn((
            DespawnOnExit(Ctx::DevSpawnPanel),
            SpawnListContainer,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(BG),
        ))
        .with_children(|root| {
            // Title bar — children rebuilt each frame
            root.spawn((
                SpawnTitleBar,
                Node {
                    width: Val::Percent(100.0),
                    padding: UiRect::axes(Val::Px(20.0), Val::Px(10.0)),
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(HEADER_BG),
            ));

            // Body row: grows to fill all space between title and footer
            root.spawn((Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                min_height: Val::Px(0.0),
                flex_direction: FlexDirection::Row,
                overflow: Overflow::clip(),
                ..default()
            },))
            .with_children(|body| {
                // Left: list panel — children rebuilt each frame
                body.spawn((
                    SpawnListPanel,
                    Node {
                        width: Val::Percent(38.0),
                        min_width: Val::Percent(38.0),
                        max_width: Val::Percent(38.0),
                        min_height: Val::Px(0.0),
                        flex_shrink: 0.0,
                        flex_grow: 0.0,
                        flex_direction: FlexDirection::Column,
                        overflow: Overflow::clip(),
                        border: UiRect::right(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(PANEL_BG),
                    BorderColor::all(DIVIDER),
                ));

                // Right: detail panel — children rebuilt each frame
                body.spawn((
                    SpawnDetailPanel,
                    Node {
                        flex_grow: 1.0,
                        min_height: Val::Px(0.0),
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(Val::Px(16.0)),
                        row_gap: Val::Px(4.0),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    BackgroundColor(PANEL_BG),
                ));
            });

            // Filter bar — background + child rebuilt each frame
            root.spawn((
                SpawnFilterBar,
                Node {
                    width: Val::Percent(100.0),
                    padding: UiRect::axes(Val::Px(20.0), Val::Px(6.0)),
                    border: UiRect::top(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                BorderColor::all(DIVIDER),
            ));

            // Footer — static, built once
            root.spawn((
                Node {
                    width: Val::Percent(100.0),
                    padding: UiRect::axes(Val::Px(20.0), Val::Px(8.0)),
                    border: UiRect::top(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(HEADER_BG),
                BorderColor::all(DIVIDER),
            ))
            .with_child((
                Text::new(
                    "[j/k / ↑↓] navigate    [PgUp/PgDn] page    [Home/End] first/last    [/] filter    [Enter] spawn    [Esc] close",
                ),
                TextFont { font_size: 12.0, ..default() },
                TextColor(TEXT_DIM),
            ));
        });
}

// ---------------------------------------------------------------------------
// Update — rebuild only the *content* inside each persistent container
// ---------------------------------------------------------------------------

pub(crate) fn update_dev_spawn_panel(
    mut commands: Commands,
    focus: Res<DevSpawnFocus>,
    title_bar: Query<Entity, With<SpawnTitleBar>>,
    list_panel: Query<Entity, With<SpawnListPanel>>,
    detail_panel: Query<Entity, With<SpawnDetailPanel>>,
    filter_bar: Query<Entity, With<SpawnFilterBar>>,
    detail: DefDetailQueries,
) {
    let filtered = focus.filtered_entries();
    let total = filtered.len();

    // ── Title bar ─────────────────────────────────────────────────────────
    if let Ok(title_e) = title_bar.single() {
        let status = if focus.filter.is_empty() {
            format!("{} items", focus.catalog.len())
        } else {
            format!("{} / {} items", total, focus.catalog.len())
        };
        commands
            .entity(title_e)
            .despawn_children()
            .with_children(|h| {
                h.spawn((
                    Text::new("DEBUG: SPAWN ITEM"),
                    TextFont { font_size: 22.0, ..default() },
                    TextColor(ACCENT),
                ));
                h.spawn((
                    Text::new(status),
                    TextFont { font_size: 13.0, ..default() },
                    TextColor(TEXT_DIM),
                ));
            });
    }

    // ── List panel ────────────────────────────────────────────────────────
    if let Ok(list_e) = list_panel.single() {
        // Centered scroll: selected row stays near the middle.
        let scroll_start = focus.index.saturating_sub(VISIBLE_ROWS / 2);
        let scroll_end = (scroll_start + VISIBLE_ROWS).min(total);

        commands
            .entity(list_e)
            .despawn_children()
            .with_children(|list| {
                if filtered.is_empty() {
                    list.spawn((Node {
                        padding: UiRect::axes(Val::Px(16.0), Val::Px(14.0)),
                        ..default()
                    },))
                    .with_child((
                        Text::new(if focus.catalog.is_empty() {
                            "Loading..."
                        } else {
                            "No matches"
                        }),
                        TextFont { font_size: 16.0, ..default() },
                        TextColor(TEXT_DIM),
                    ));
                    return;
                }

                // Position indicator
                list.spawn((
                    Node {
                        padding: UiRect::new(
                            Val::Px(14.0),
                            Val::Px(14.0),
                            Val::Px(3.0),
                            Val::Px(3.0),
                        ),
                        border: UiRect::bottom(Val::Px(1.0)),
                        ..default()
                    },
                    BorderColor::all(DIVIDER),
                ))
                .with_child((
                    Text::new(format!("{} / {}", focus.index + 1, total)),
                    TextFont { font_size: 11.0, ..default() },
                    TextColor(TEXT_DIM),
                ));

                for i in scroll_start..scroll_end {
                    let entry = &filtered[i];
                    let is_focused = i == focus.index;
                    let row_bg = if is_focused { ITEM_FOCUS_BG } else { ITEM_BG };

                    list.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::axes(Val::Px(14.0), Val::Px(5.0)),
                            border: UiRect::bottom(Val::Px(1.0)),
                            ..default()
                        },
                        BackgroundColor(row_bg),
                        BorderColor::all(DIVIDER),
                    ))
                    .with_children(|row| {
                        row.spawn((
                            Text::new(entry.name.clone()),
                            TextFont { font_size: 15.0, ..default() },
                            TextColor(TEXT_BRIGHT),
                        ));
                        row.spawn((
                            Text::new(entry.def_id.clone()),
                            TextFont { font_size: 11.0, ..default() },
                            TextColor(TEXT_ID),
                        ));
                    });
                }
            });
    }

    // ── Detail panel ──────────────────────────────────────────────────────
    if let Ok(det_e) = detail_panel.single() {
        let selected_entity = filtered.get(focus.index).map(|e| e.def_entity);
        let selected_name =
            filtered.get(focus.index).map(|e| e.name.as_str()).unwrap_or("");
        let selected_id =
            filtered.get(focus.index).map(|e| e.def_id.as_str()).unwrap_or("");

        commands
            .entity(det_e)
            .despawn_children()
            .with_children(|d| {
                let Some(def) = selected_entity else {
                    d.spawn((
                        Text::new("Select an item"),
                        TextFont { font_size: 16.0, ..default() },
                        TextColor(TEXT_DIM),
                    ));
                    return;
                };

                // Name + ID header
                d.spawn((
                    Text::new(selected_name.to_string()),
                    TextFont { font_size: 22.0, ..default() },
                    TextColor(ACCENT2),
                ));
                d.spawn((
                    Text::new(format!("id: {}", selected_id)),
                    TextFont { font_size: 12.0, ..default() },
                    TextColor(TEXT_ID),
                ));

                divider(d);

                // Description
                if let Ok(desc) = detail.item_descs.get(def) {
                    if !desc.0.is_empty() {
                        d.spawn((
                            Text::new(desc.0.clone()),
                            TextFont { font_size: 13.0, ..default() },
                            TextColor(TEXT_BRIGHT),
                        ));
                        divider(d);
                    }
                }

                // Basic properties
                section_header(d, "Properties");

                let sym = detail.item_symbols.get(def).map(|s| s.0).unwrap_or('?');
                let weight_g = detail.item_weights.get(def).map(|w| w.0).unwrap_or(0);
                let volume_ml = detail.item_volumes.get(def).map(|v| v.0).unwrap_or(0);

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

                stat_row(d, "Symbol", &sym.to_string());
                stat_row(d, "Weight", &weight_str);
                stat_row(d, "Volume", &volume_str);

                if let Ok(color) = detail.item_colors.get(def) {
                    stat_row(d, "Color", &color.0);
                }
                if let Ok(cat) = detail.item_categories.get(def) {
                    stat_row(d, "Category", &cat.0);
                }
                if let Ok(mats) = detail.item_materials.get(def) {
                    if !mats.0.is_empty() {
                        stat_row(d, "Materials", &mats.0.join(", "));
                    }
                }
                if let Ok(phase) = detail.item_phases.get(def) {
                    let phase_str = match phase.0 {
                        Phase::Solid => "Solid",
                        Phase::Liquid => "Liquid",
                        Phase::Gas => "Gas",
                        Phase::Plasma => "Plasma",
                    };
                    stat_row(d, "Phase", phase_str);
                }

                // Qualities
                if let Ok(quals) = detail.item_qualities.get(def) {
                    if !quals.0.is_empty() {
                        divider(d);
                        section_header(d, "Tool Qualities");
                        for (id, level) in &quals.0 {
                            stat_row(d, id, &level.to_string());
                        }
                    }
                }

                // Weapon
                if let Ok(w) = detail.weapon_data.get(def) {
                    divider(d);
                    section_header(d, "Melee");
                    stat_row(
                        d,
                        "Bash / Cut / Stab",
                        &format!("{} / {} / {}", w.damage_bash, w.damage_cut, w.damage_stab),
                    );
                    stat_row(d, "To-hit", &w.to_hit.to_string());
                    stat_row(d, "Moves/attack", &w.moves_per_attack.to_string());
                    if w.reach > 1 {
                        stat_row(d, "Reach", &w.reach.to_string());
                    }
                    if !w.techniques.is_empty() {
                        stat_row(d, "Techniques", &w.techniques.join(", "));
                    }
                }

                // Gun
                if let Ok(g) = detail.gun_data.get(def) {
                    divider(d);
                    section_header(d, "Ranged");
                    stat_row(d, "Skill", &g.skill);
                    stat_row(d, "Ammo type", &g.ammo_type);
                    stat_row(d, "Clip", &g.clip_size.to_string());
                    stat_row(d, "Reload time", &g.reload_time.to_string());
                    stat_row(d, "Dispersion", &g.dispersion.to_string());
                    if g.burst > 1 {
                        stat_row(d, "Burst", &g.burst.to_string());
                    }
                }

                // Ammo
                if let Ok(a) = detail.ammo_data.get(def) {
                    divider(d);
                    section_header(d, "Ammo");
                    stat_row(d, "Type", &a.ammo_type);
                    stat_row(d, "Damage", &a.damage.to_string());
                    stat_row(d, "Pierce", &a.pierce.to_string());
                    stat_row(d, "Range", &a.range.to_string());
                    if a.count > 1 {
                        stat_row(d, "Count", &a.count.to_string());
                    }
                    if !a.effects.is_empty() {
                        stat_row(d, "Effects", &a.effects.join(", "));
                    }
                }

                // Magazine
                if let Ok(m) = detail.magazine_data.get(def) {
                    divider(d);
                    section_header(d, "Magazine");
                    stat_row(d, "Ammo type", &m.ammo_type);
                    stat_row(d, "Capacity", &m.capacity.to_string());
                    stat_row(d, "Reload time", &m.reload_time.to_string());
                }

                // Armour
                if let Ok(armour) = detail.armour_data.get(def) {
                    divider(d);
                    section_header(d, "Armour");
                    for (i, part) in armour.parts.iter().enumerate() {
                        if i > 0 {
                            // small gap between parts
                            d.spawn(Node { height: Val::Px(4.0), ..default() });
                        }
                        let covers_str = if part.body_part.is_empty() { "?".to_string() } else { part.body_part.clone() };
                        let layers_str = if part.layers.is_empty() { "NORMAL".to_string() } else { part.layers.join(", ") };
                        stat_row(d, "Covers", &format!("{} [{}]", covers_str, layers_str));
                        stat_row(d, "Coverage", &format!("{}%  enc {}", part.coverage, part.encumbrance));
                        if !part.material.is_empty() {
                            let mat_str: Vec<String> = part.material.iter().map(|(id, thick, cov)| {
                                if *cov < 100.0 {
                                    format!("{} {:.1}mm ({}%)", id, thick, *cov as u32)
                                } else {
                                    format!("{} {:.1}mm", id, thick)
                                }
                            }).collect();
                            stat_row(d, "Material", &mat_str.join(" / "));
                        }
                        if !part.specifically_covers.is_empty() {
                            stat_row(d, "Specific", &part.specifically_covers.join(", "));
                        }
                    }
                }

                // Food
                if let Ok(food) = detail.food_data.get(def) {
                    divider(d);
                    section_header(d, "Food");
                    stat_row(d, "Type", &food.comestible_type);
                    stat_row(d, "Calories", &food.calories.to_string());
                    stat_row(d, "Quench", &food.quench.to_string());
                    stat_row(d, "Fun", &food.fun.to_string());
                    stat_row(d, "Healthy", &food.healthy.to_string());
                    if food.stim != 0 {
                        stat_row(d, "Stim", &food.stim.to_string());
                    }
                    if food.spoils_in > 0 {
                        stat_row(d, "Spoils in", &format!("{} turns", food.spoils_in));
                    }
                }

                // Tool
                if let Ok(tool) = detail.tool_data.get(def) {
                    divider(d);
                    section_header(d, "Tool");
                    if tool.max_charges != 0 {
                        stat_row(d, "Max charges", &tool.max_charges.to_string());
                        stat_row(d, "Charges/use", &tool.charges_per_use.to_string());
                    }
                    if let Some(at) = &tool.ammo_type {
                        stat_row(d, "Ammo type", at);
                    }
                    if let Some(r) = &tool.revert_to {
                        stat_row(d, "Reverts to", r);
                    }
                }

                // Container
                if let Ok(cont) = detail.container_data.get(def) {
                    divider(d);
                    section_header(d, "Pockets");
                    for (idx, pocket) in cont.pockets.iter().enumerate() {
                        if idx > 0 {
                            d.spawn(Node { height: Val::Px(3.0), ..default() });
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
                        if pocket.holster { flags.push("holster"); }
                        if pocket.ablative { flags.push("ablative"); }
                        if pocket.sealed { flags.push("sealed"); }
                        let header_str = if flags.is_empty() {
                            format!("#{} {} — {} / {}", idx + 1, type_str, vol_str, wt_str)
                        } else {
                            format!("#{} {} — {} / {}  [{}]", idx + 1, type_str, vol_str, wt_str, flags.join(", "))
                        };
                        stat_row(d, "Pocket", &header_str);
                        if !pocket.description.is_empty() {
                            stat_row(d, "Desc", &pocket.description);
                        }
                        if !pocket.flag_restriction.is_empty() {
                            stat_row(d, "Flags", &pocket.flag_restriction.join(", "));
                        }
                    }
                }

                // Book
                if let Ok(book) = detail.book_data.get(def) {
                    divider(d);
                    section_header(d, "Book");
                    stat_row(d, "Skill", &book.skill);
                    stat_row(
                        d,
                        "Levels",
                        &format!("{} → {}", book.required_level, book.max_level),
                    );
                    stat_row(d, "Fun", &book.fun.to_string());
                    stat_row(d, "Int req.", &book.intelligence.to_string());
                    stat_row(d, "Read time", &format!("{} turns", book.time));
                }
            });
    }

    // ── Filter bar ────────────────────────────────────────────────────────
    if let Ok(filt_e) = filter_bar.single() {
        let filter_bg = if focus.filtering { FILTER_ACTIVE_BG } else { Color::NONE };
        let filter_text = if focus.filtering {
            format!("Filter: {}_", focus.filter)
        } else if focus.filter.is_empty() {
            "[/] to filter".to_string()
        } else {
            format!("Filter: {}  (Enter=close  Esc=clear)", focus.filter)
        };
        commands.entity(filt_e).insert(BackgroundColor(filter_bg));
        commands
            .entity(filt_e)
            .despawn_children()
            .with_child((
                Text::new(filter_text),
                TextFont { font_size: 13.0, ..default() },
                TextColor(if focus.filtering { TEXT_BRIGHT } else { TEXT_DIM }),
            ));
    }
}

// ---------------------------------------------------------------------------
// UI helpers
// ---------------------------------------------------------------------------

fn divider(parent: &mut ChildSpawnerCommands) {
    parent.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(1.0),
            margin: UiRect::vertical(Val::Px(3.0)),
            ..default()
        },
        BackgroundColor(DIVIDER),
    ));
}

fn section_header(parent: &mut ChildSpawnerCommands, title: &str) {
    parent.spawn((
        Text::new(title.to_uppercase()),
        TextFont { font_size: 11.0, ..default() },
        TextColor(LABEL),
    ));
}

fn stat_row(parent: &mut ChildSpawnerCommands, label: &str, value: &str) {
    parent
        .spawn((Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(4.0),
            ..default()
        },))
        .with_children(|row| {
            row.spawn((
                Text::new(format!("{}: ", label)),
                TextFont { font_size: 13.0, ..default() },
                TextColor(TEXT_DIM),
                Node {
                    min_width: Val::Px(110.0),
                    ..default()
                },
            ));
            row.spawn((
                Text::new(value.to_string()),
                TextFont { font_size: 13.0, ..default() },
                TextColor(TEXT_BRIGHT),
            ));
        });
}
