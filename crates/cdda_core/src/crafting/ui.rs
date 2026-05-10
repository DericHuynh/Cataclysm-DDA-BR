//! Crafting menu — recipe browser with tabbed category navigation,
//! craftability indicator, filter, and craft execution.
//!
//! Uses the same visual design as `render/dev_spawn.rs` (debug spawn panel):
//! scrollable list with focus highlight, position counter, name + ID layout,
//! and a matching colour palette.
//!
//! Layout (top to bottom):
//!   1. Header bar — "CRAFTING" title, recipe count / position
//!   2. Top-level category tabs (LEFT/RIGHT to switch)
//!   3. Subcategory tabs (for selected top category)
//!   4. Body — recipe list (left) + detail panel (right)
//!   5. Filter bar (at bottom)
//!   6. Footer with key hints
//!
//! Systems:
//! - `build_craft_state` (exclusive OnEnter) — populates `CraftState` + `CategoryIndex`
//! - `spawn_crafting_ui` (regular OnEnter) — spawns the root wrapper node
//! - `update_crafting_ui` (regular Update) — rebuilds content when state changes
//! - `crafting_menu_input` (regular, per-frame) — keyboard navigation
//! - `process_pending_craft` (exclusive, per-frame) — executes craft

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_ecs::message::MessageReader;
use bevy_input::keyboard::{Key, KeyboardInput};
use bevy_input::ButtonState;
use bevy_state::prelude::NextState;
use bevy_state::state_scoped::DespawnOnExit;

use crate::context::ctx::Ctx;
use crate::context::nav::{pop_ctx, FocusedCommandIndex};
use crate::context::ContextStack;
use crate::core::components::def::ItemName;
use crate::core::components::def::{
    AmmoData, ArmourData, BookData, ContainerData, FoodData, GunData, ItemCategory, ItemColor,
    ItemDescription, ItemMaterials, ItemPhase, ItemSymbol, ItemVolume, ItemWeight, MagazineData,
    Phase, RecipeCategory, RecipeComponents, RecipeQualities, RecipeResult, RecipeResultCount,
    RecipeSubcategory, RecipeTime, ToolData, WeaponData,
};
use crate::core::components::item::ItemQualities;
use crate::data::def_world::DefinitionWorld;
use crate::crafting::systems::{
    check_can_craft, collect_available_items, display_category, display_subcategory,
    find_dev_player, start_craft, CategoryIndex, RecipeIndex,
};
use crate::input::context::{InputContextId, InputContextStack};
use crate::input::{GameAction, InputAction};

// ---------------------------------------------------------------------------
// Colours — adapted from render/dev_spawn.rs palette
// ---------------------------------------------------------------------------

const BG: Color = Color::srgb(0.04, 0.04, 0.06);
const HEADER_BG: Color = Color::srgb(0.10, 0.10, 0.14);
const ITEM_BG: Color = Color::srgb(0.07, 0.07, 0.10);
const ITEM_FOCUS_BG: Color = Color::srgb(0.12, 0.35, 0.55);
const ACCENT: Color = Color::srgb(0.30, 0.70, 1.00);
const ACCENT_CRAFTABLE: Color = Color::srgb(0.40, 0.85, 0.40);
const TEXT_BRIGHT: Color = Color::srgb(0.95, 0.95, 0.95);
const TEXT_DIM: Color = Color::srgb(0.55, 0.55, 0.55);
const TEXT_RED: Color = Color::srgb(0.85, 0.30, 0.30);
const TEXT_ID: Color = Color::srgb(0.50, 0.65, 0.50);
const DIVIDER: Color = Color::srgb(0.20, 0.20, 0.25);
const TAB_BG: Color = Color::srgb(0.08, 0.08, 0.14);
const TAB_ACTIVE_BG: Color = Color::srgb(0.15, 0.25, 0.40);
const PANEL_BG: Color = Color::srgb(0.06, 0.06, 0.10);
const LABEL: Color = Color::srgb(0.50, 0.75, 0.90);

/// Number of recipe rows visible at once in the scroll window.
const VISIBLE_ROWS: usize = 22;

// ---------------------------------------------------------------------------
// CraftEntry / CraftState
// ---------------------------------------------------------------------------

/// One row in the crafting menu recipe list.
#[derive(Clone)]
pub struct CraftEntry {
    pub recipe_entity: Entity,
    pub result_id: String,
    pub result_name: String,
    pub result_count: u32,
    pub craftable: bool,
    /// First blocking reason when not craftable.
    pub reason: String,
    pub time_turns: u32,
    pub components_text: Vec<String>,
    pub qualities_text: Vec<String>,
}

/// UI state for the crafting menu, rebuilt each time the menu is opened.
#[derive(Resource)]
pub struct CraftState {
    pub focus: usize,
    /// When `true`, shows all recipes; when `false`, shows only craftable ones.
    pub show_all: bool,
    pub entries: Vec<CraftEntry>,
    /// Message shown after a craft attempt (success or failure).
    pub last_message: Option<String>,
    /// Current substring filter (case-insensitive match on result name/ID).
    pub filter: String,
    /// True while the TextInput context is active for filter editing.
    pub filtering: bool,
}

impl Default for CraftState {
    fn default() -> Self {
        Self {
            focus: 0,
            show_all: true,
            entries: Vec::new(),
            last_message: None,
            filter: String::new(),
            filtering: false,
        }
    }
}

impl CraftState {
    /// Entries matching the current filter (and show_all/craftable toggle).
    pub fn visible(&self) -> impl Iterator<Item = &CraftEntry> {
        let filter = self.filter.to_lowercase();
        self.entries.iter().filter(move |e| {
            (self.show_all || e.craftable)
                && (filter.is_empty()
                    || e.result_name.to_lowercase().contains(&filter)
                    || e.result_id.to_lowercase().contains(&filter))
        })
    }

    pub fn visible_count(&self) -> usize {
        self.visible().count()
    }

    pub fn focused_entry(&self) -> Option<&CraftEntry> {
        self.visible().nth(self.focus)
    }
}

// ---------------------------------------------------------------------------
// PendingCraft — bridges input to the exclusive craft system
// ---------------------------------------------------------------------------

/// Set by `crafting_menu_input` when the player confirms a craft.
/// Drained each frame by `process_pending_craft`.
#[derive(Resource, Default)]
pub struct PendingCraft(pub Option<Entity>);

// ---------------------------------------------------------------------------
// CraftMenuRoot — marker for the persistent UI shell
// ---------------------------------------------------------------------------

/// Marks the root node of the crafting menu. Content is rebuilt each frame
/// by `update_crafting_ui` when `CraftState` changes.
#[derive(Component)]
pub struct CraftMenuRoot;

// ── Sub-components for targeted despawn ─────────────────────────────────────

#[derive(Component)]
pub struct HeaderContainer;

#[derive(Component)]
pub struct CategoryTabsContainer;

#[derive(Component)]
pub struct SubcategoryTabsContainer;

#[derive(Component)]
pub struct RecipeListContainer;

#[derive(Component)]
pub struct DetailPanelContainer;

#[derive(Component)]
pub struct ItemDetailPanelContainer;

#[derive(Component)]
pub struct FilterBarContainer;

#[derive(Component)]
pub struct FooterContainer;

// ---------------------------------------------------------------------------
// build_craft_state — exclusive OnEnter system
// ---------------------------------------------------------------------------

/// Rebuild `CraftState` from the current world state.
/// Runs first on `OnEnter(Ctx::CraftingMenu)`.
pub fn build_craft_state(world: &mut World) {
    let Some(player) = find_dev_player(world) else {
        return;
    };

    let available = collect_available_items(world, player);

    let recipe_entities: Vec<Entity> = world
        .get_resource::<RecipeIndex>()
        .map(|ri| ri.0.clone())
        .unwrap_or_default();

    // ── Build category index ──────────────────────────────────────────────
    let mut cat_index = CategoryIndex::default();
    let mut seen_top: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut seen_sub: std::collections::BTreeSet<(String, String)> =
        std::collections::BTreeSet::new();

    for &re in &recipe_entities {
        if world.get::<RecipeResult>(re).is_none() {
            continue;
        }
        let raw_cat = world
            .get::<RecipeCategory>(re)
            .map(|c| c.0.clone())
            .unwrap_or_else(|| "CC_MISC".to_string());
        let raw_sub = world
            .get::<RecipeSubcategory>(re)
            .map(|s| s.0.clone())
            .unwrap_or_else(|| "CSC_MISC_NONE".to_string());

        let cat_display = display_category(&raw_cat);
        let sub_display = display_subcategory(&raw_cat, &raw_sub);

        seen_top.insert(cat_display.clone());
        seen_sub.insert((cat_display.clone(), sub_display.clone()));

        cat_index
            .sub_recipes
            .entry((cat_display, sub_display))
            .or_default()
            .push(re);
    }

    cat_index.top_categories = seen_top.into_iter().collect();
    // Ensure all (top,sub) keys exist even if no recipes are in a subcategory yet
    // (they were populated above)
    cat_index.selected_top = 0;
    cat_index.selected_sub = 0;

    world.insert_resource(cat_index.clone());

    // ── Build craft entries ───────────────────────────────────────────────
    let def_world: Option<&DefinitionWorld> = world.get_resource::<DefinitionWorld>();

    let mut entries: Vec<CraftEntry> = recipe_entities
        .iter()
        .filter(|&&re| world.get::<RecipeResult>(re).is_some())
        .filter_map(|&re| {
            let result_id = world
                .get::<RecipeResult>(re)
                .map(|r| r.0.clone())
                .unwrap_or_default();

            // Look up display name from the item def entity
            let result_name = def_world
                .and_then(|dw| dw.entity_by_str(&result_id))
                .and_then(|def_e| world.get::<ItemName>(def_e).map(|n| n.0.clone()))
                .unwrap_or_else(|| result_id.clone());

            let result_count = world.get::<RecipeResultCount>(re).map(|c| c.0).unwrap_or(1);
            let time_turns = world.get::<RecipeTime>(re).map(|t| t.0).unwrap_or(0);

            let (craftable, reason) = match check_can_craft(world, re, &available) {
                Ok(()) => (true, String::new()),
                Err(e) => (false, e),
            };

            let components_text = world
                .get::<RecipeComponents>(re)
                .map(|comps| {
                    comps
                        .0
                        .iter()
                        .filter_map(|slot| slot.first())
                        .map(|entry| {
                            if slot_has_alternatives(world, re, &entry.item_id) {
                                format!("  {} x{}  (or alternatives)", entry.item_id, entry.count)
                            } else {
                                format!("  {} x{}", entry.item_id, entry.count)
                            }
                        })
                        .collect()
                })
                .unwrap_or_default();

            let qualities_text = world
                .get::<RecipeQualities>(re)
                .map(|quals| {
                    quals
                        .0
                        .iter()
                        .map(|(id, lvl)| format!("  {} (level {})", id, lvl))
                        .collect()
                })
                .unwrap_or_default();

            Some(CraftEntry {
                recipe_entity: re,
                result_id,
                result_name,
                result_count,
                craftable,
                reason,
                time_turns,
                components_text,
                qualities_text,
            })
        })
        .collect();

    // Craftable first, then alphabetical by display name
    entries.sort_by(|a, b| {
        b.craftable
            .cmp(&a.craftable)
            .then(a.result_name.cmp(&b.result_name))
    });

    let (show_all, last_message, filter, filtering) = world
        .get_resource::<CraftState>()
        .map(|s| {
            (
                s.show_all,
                s.last_message.clone(),
                s.filter.clone(),
                s.filtering,
            )
        })
        .unwrap_or((true, None, String::new(), false));

    world.insert_resource(CraftState {
        focus: 0,
        show_all,
        entries,
        last_message,
        filter,
        filtering,
    });
}

fn slot_has_alternatives(world: &World, re: Entity, first_id: &str) -> bool {
    world
        .get::<RecipeComponents>(re)
        .map(|comps| {
            comps
                .0
                .iter()
                .any(|slot| slot.iter().any(|e| e.item_id == first_id) && slot.len() > 1)
        })
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// spawn_crafting_ui — regular OnEnter system (root shell only)
// ---------------------------------------------------------------------------

/// Spawn the persistent root wrapper for the crafting menu.
/// Content is built by `update_crafting_ui` which runs immediately in Update.
pub fn spawn_crafting_ui(mut commands: Commands) {
    commands
        .spawn((
            DespawnOnExit(Ctx::CraftingMenu),
            CraftMenuRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(BG),
        ))
        .with_children(|root| {
            // ── 1. Header ─────────────────────────────────────────────────
            root.spawn((
                HeaderContainer,
                Node {
                    width: Val::Percent(100.0),
                    padding: UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
                    ..default()
                },
                BackgroundColor(HEADER_BG),
            ));

            // ── 2. Top-level category tabs ────────────────────────────────
            root.spawn((
                CategoryTabsContainer,
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    padding: UiRect::axes(Val::Px(4.0), Val::Px(4.0)),
                    column_gap: Val::Px(4.0),
                    overflow: Overflow::clip_x(),
                    ..default()
                },
                BackgroundColor(TAB_BG),
            ));

            // ── 3. Subcategory tabs ───────────────────────────────────────
            root.spawn((
                SubcategoryTabsContainer,
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    padding: UiRect::axes(Val::Px(4.0), Val::Px(3.0)),
                    column_gap: Val::Px(3.0),
                    overflow: Overflow::clip_x(),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.06, 0.06, 0.12)),
            ));

            // ── 4. Body: recipe list + detail panel ───────────────────────
            root.spawn((Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                flex_direction: FlexDirection::Row,
                min_height: Val::Px(300.0),
                ..default()
            },))
                .with_children(|body| {
                    // Left: recipe list
                    body.spawn((
                        RecipeListContainer,
                        Node {
                            width: Val::Percent(45.0),
                            min_width: Val::Percent(45.0),
                            max_width: Val::Percent(45.0),
                            flex_shrink: 0.0,
                            flex_grow: 0.0,
                            flex_direction: FlexDirection::Column,
                            overflow: Overflow::clip(),
                            ..default()
                        },
                        BackgroundColor(PANEL_BG),
                    ));

                    // Middle: crafting detail panel
                    body.spawn((
                        DetailPanelContainer,
                        Node {
                            flex_grow: 1.0,
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::all(Val::Px(14.0)),
                            row_gap: Val::Px(5.0),
                            overflow: Overflow::clip(),
                            ..default()
                        },
                        BackgroundColor(PANEL_BG),
                    ));

                    // Right: item detail panel
                    body.spawn((
                        ItemDetailPanelContainer,
                        Node {
                            flex_grow: 1.0,
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::all(Val::Px(14.0)),
                            row_gap: Val::Px(4.0),
                            overflow: Overflow::clip(),
                            ..default()
                        },
                        BackgroundColor(PANEL_BG),
                    ));
                });

            // ── 5. Filter bar (at bottom) ─────────────────────────────────
            root.spawn((
                FilterBarContainer,
                Node {
                    width: Val::Percent(100.0),
                    padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                    ..default()
                },
                BackgroundColor(HEADER_BG),
            ));

            // ── 6. Footer ─────────────────────────────────────────────────
            root.spawn((
                FooterContainer,
                Node {
                    width: Val::Percent(100.0),
                    padding: UiRect::axes(Val::Px(12.0), Val::Px(5.0)),
                    border: UiRect::top(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(HEADER_BG),
                BorderColor::all(DIVIDER),
            ));
        });
}

// ---------------------------------------------------------------------------
// Bundled def queries for the item detail panel
// ---------------------------------------------------------------------------

#[derive(SystemParam)]
pub struct ItemDefQueries<'w, 's> {
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
// update_crafting_ui — regular Update system
// ---------------------------------------------------------------------------

/// Rebuild crafting menu content whenever `CraftState` or `CategoryIndex` changes.
pub fn update_crafting_ui(
    mut commands: Commands,
    state: Res<CraftState>,
    cat_index: Res<CategoryIndex>,
    def_world: Res<DefinitionWorld>,
    root_q: Query<Entity, With<CraftMenuRoot>>,
    header_q: Query<Entity, With<HeaderContainer>>,
    cat_tabs_q: Query<Entity, With<CategoryTabsContainer>>,
    sub_tabs_q: Query<Entity, With<SubcategoryTabsContainer>>,
    list_q: Query<Entity, With<RecipeListContainer>>,
    detail_q: Query<Entity, With<DetailPanelContainer>>,
    item_detail_q: Query<Entity, With<ItemDetailPanelContainer>>,
    filter_q: Query<Entity, With<FilterBarContainer>>,
    footer_q: Query<Entity, With<FooterContainer>>,
    defs: ItemDefQueries,
) {
    let Ok(_root) = root_q.single() else {
        return;
    };
    let Ok(header) = header_q.single() else {
        return;
    };
    let Ok(cat_tabs) = cat_tabs_q.single() else {
        return;
    };
    let Ok(sub_tabs) = sub_tabs_q.single() else {
        return;
    };
    let Ok(list) = list_q.single() else {
        return;
    };
    let Ok(detail) = detail_q.single() else {
        return;
    };
    let Ok(item_detail) = item_detail_q.single() else {
        return;
    };
    let Ok(filter_bar) = filter_q.single() else {
        return;
    };
    let Ok(footer) = footer_q.single() else {
        return;
    };

    let focus = state.focus;
    let show_all = state.show_all;
    let filtering = state.filtering;
    let filter = state.filter.clone();
    let _total_visible = state.visible_count();
    let focus_zone = cat_index.focus_zone;
    let last_message = state.last_message.clone();

    // Determine current category/subcategory
    let sel_top = cat_index
        .selected_top
        .min(cat_index.top_categories.len().saturating_sub(1));
    let current_top = cat_index
        .top_categories
        .get(sel_top)
        .cloned()
        .unwrap_or_default();

    // Collect subcategories for the selected top category
    let subcats_for_top: Vec<String> = cat_index
        .sub_recipes
        .keys()
        .filter(|(top, _)| top == &current_top)
        .map(|(_, sub)| sub.clone())
        .collect();

    let sel_sub = cat_index
        .selected_sub
        .min(subcats_for_top.len().saturating_sub(1));
    let current_sub = subcats_for_top.get(sel_sub).cloned().unwrap_or_default();

    // Get recipes in the current (top, sub) pair
    let current_key = (current_top.clone(), current_sub.clone());
    let category_recipes: Vec<Entity> = cat_index
        .sub_recipes
        .get(&current_key)
        .cloned()
        .unwrap_or_default();

    // Filter visible entries to the current category/subcategory
    let category_filtered: Vec<&CraftEntry> = state
        .visible()
        .filter(|e| category_recipes.contains(&e.recipe_entity))
        .collect();

    let total_in_cat = category_filtered.len();
    let focused_entry = category_filtered
        .get(focus.min(total_in_cat.saturating_sub(1)))
        .cloned();

    // ── Header ────────────────────────────────────────────────────────────
    commands
        .entity(header)
        .despawn_children()
        .with_children(|h| {
            h.spawn((
                Text::new("CRAFTING"),
                TextFont {
                    font_size: 22.0,
                    ..default()
                },
                TextColor(ACCENT),
            ));
            let status = if total_in_cat > 0 {
                let sub_focus = if focus < total_in_cat {
                    focus + 1
                } else {
                    total_in_cat
                };
                format!(
                    "Recipe {} of {}  [{}]",
                    sub_focus,
                    total_in_cat,
                    if show_all { "ALL" } else { "CRAFTABLE" }
                )
            } else if !filter.is_empty() {
                format!(
                    "No matching recipes  [{}]",
                    if show_all { "ALL" } else { "CRAFTABLE" }
                )
            } else {
                format!(
                    "No recipes  [{}]",
                    if show_all { "ALL" } else { "CRAFTABLE" }
                )
            };
            h.spawn((
                Text::new(status),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(TEXT_DIM),
            ));
        });

    // ── Top-level category tabs ────────────────────────────────────────────
    commands
        .entity(cat_tabs)
        .despawn_children()
        .with_children(|tabs| {
            for (i, cat_name) in cat_index.top_categories.iter().enumerate() {
                let is_active = i == sel_top;
                let zone_highlight = focus_zone == 1 && is_active;
                let tab_bg = if zone_highlight {
                    Color::srgb(0.15, 0.30, 0.45)
                } else if is_active {
                    TAB_ACTIVE_BG
                } else {
                    Color::NONE
                };
                let text_color = if is_active { ACCENT } else { TEXT_DIM };
                tabs.spawn((
                    Node {
                        padding: UiRect::axes(Val::Px(10.0), Val::Px(4.0)),
                        border: UiRect::bottom(if is_active {
                            Val::Px(2.0)
                        } else {
                            Val::Px(0.0)
                        }),
                        ..default()
                    },
                    BackgroundColor(tab_bg),
                    BorderColor::all(if is_active { ACCENT } else { Color::NONE }),
                ))
                .with_child((
                    Text::new(cat_name.clone()),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(text_color),
                ));
            }
        });

    // ── Subcategory tabs ───────────────────────────────────────────────────
    commands
        .entity(sub_tabs)
        .despawn_children()
        .with_children(|tabs| {
            for (i, sub_name) in subcats_for_top.iter().enumerate() {
                let is_active = i == sel_sub;
                let zone_highlight = focus_zone == 2 && is_active;
                let tab_bg = if zone_highlight {
                    Color::srgb(0.15, 0.30, 0.45)
                } else if is_active {
                    Color::srgb(0.10, 0.18, 0.28)
                } else {
                    Color::NONE
                };
                let text_color = if is_active { TEXT_BRIGHT } else { TEXT_DIM };
                tabs.spawn((
                    Node {
                        padding: UiRect::axes(Val::Px(8.0), Val::Px(3.0)),
                        ..default()
                    },
                    BackgroundColor(tab_bg),
                ))
                .with_child((
                    Text::new(sub_name.clone()),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(text_color),
                ));
            }
        });

    // ── Recipe list ────────────────────────────────────────────────────────
    commands
        .entity(list)
        .despawn_children()
        .with_children(|list_node| {
            if category_filtered.is_empty() {
                list_node
                    .spawn((Node {
                        padding: UiRect::all(Val::Px(14.0)),
                        ..default()
                    },))
                    .with_child((
                        Text::new("No recipes in this category."),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(TEXT_DIM),
                    ));
                return;
            }

            // Compute scroll window so focused row is always visible
            let focus_clamped = focus.min(total_in_cat.saturating_sub(1));

            let scroll_start = if focus_clamped >= VISIBLE_ROWS {
                focus_clamped + 1 - VISIBLE_ROWS
            } else {
                0
            };
            let scroll_end = (scroll_start + VISIBLE_ROWS).min(total_in_cat);

            // Position counter
            list_node
                .spawn((Node {
                    padding: UiRect::new(Val::Px(12.0), Val::Px(12.0), Val::Px(4.0), Val::Px(4.0)),
                    ..default()
                },))
                .with_child((
                    Text::new(format!("Recipe {} of {}", focus_clamped + 1, total_in_cat)),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(TEXT_DIM),
                ));

            // Recipe rows
            for (i, entry) in category_filtered[scroll_start..scroll_end]
                .iter()
                .enumerate()
            {
                let abs_index = scroll_start + i;
                let is_focused = abs_index == focus_clamped;
                let row_bg = if is_focused { ITEM_FOCUS_BG } else { ITEM_BG };

                let mark = if entry.craftable { "+" } else { "-" };
                let row_label = if entry.result_count > 1 {
                    format!("[{}] {}  x{}", mark, entry.result_name, entry.result_count)
                } else {
                    format!("[{}] {}", mark, entry.result_name)
                };
                let id_label = format!("  [{}]", entry.result_id);

                list_node
                    .spawn((
                        Node {
                            width: Val::Percent(100.0),
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                            border: UiRect::bottom(Val::Px(1.0)),
                            ..default()
                        },
                        BackgroundColor(row_bg),
                        BorderColor::all(DIVIDER),
                    ))
                    .with_children(|row| {
                        row.spawn((
                            Text::new(row_label),
                            TextFont {
                                font_size: 15.0,
                                ..default()
                            },
                            TextColor(TEXT_BRIGHT),
                            Node {
                                flex_grow: 1.0,
                                ..default()
                            },
                        ));
                        row.spawn((
                            Text::new(id_label),
                            TextFont {
                                font_size: 13.0,
                                ..default()
                            },
                            TextColor(TEXT_ID),
                        ));
                    });
            }
        });

    // ── Detail panel ───────────────────────────────────────────────────────
    commands
        .entity(detail)
        .despawn_children()
        .with_children(|det| {
            if let Some(entry) = &focused_entry {
                // Title
                det.spawn((
                    Text::new(format!(
                        "{}{}",
                        entry.result_name,
                        if entry.result_count > 1 {
                            format!("  x{}", entry.result_count)
                        } else {
                            String::new()
                        }
                    )),
                    TextFont {
                        font_size: 18.0,
                        ..default()
                    },
                    TextColor(ACCENT),
                ));

                // ID
                det.spawn((
                    Text::new(format!("[{}]", entry.result_id)),
                    TextFont {
                        font_size: 12.0,
                        ..default()
                    },
                    TextColor(TEXT_ID),
                ));

                // Craftability
                if entry.craftable {
                    det.spawn((
                        Text::new("Craftable: YES"),
                        TextFont {
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(ACCENT_CRAFTABLE),
                    ));
                }

                // Time
                if entry.time_turns > 0 {
                    det.spawn((
                        Text::new(format!("Time: {} turns", entry.time_turns)),
                        TextFont {
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(TEXT_DIM),
                    ));
                }

                // Divider
                det.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(1.0),
                        margin: UiRect::vertical(Val::Px(4.0)),
                        ..default()
                    },
                    BackgroundColor(DIVIDER),
                ));

                // Components
                det.spawn((
                    Text::new("Components:"),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(TEXT_DIM),
                ));
                if entry.components_text.is_empty() {
                    det.spawn((
                        Text::new("  (none)"),
                        TextFont {
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(TEXT_DIM),
                    ));
                } else {
                    for line in &entry.components_text {
                        det.spawn((
                            Text::new(line.clone()),
                            TextFont {
                                font_size: 12.0,
                                ..default()
                            },
                            TextColor(TEXT_BRIGHT),
                        ));
                    }
                }

                // Tool qualities
                if !entry.qualities_text.is_empty() {
                    det.spawn((
                        Text::new("Tool qualities:"),
                        TextFont {
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(TEXT_DIM),
                    ));
                    for line in &entry.qualities_text {
                        det.spawn((
                            Text::new(line.clone()),
                            TextFont {
                                font_size: 12.0,
                                ..default()
                            },
                            TextColor(TEXT_BRIGHT),
                        ));
                    }
                }

                // Cannot craft reason
                if !entry.craftable {
                    det.spawn((Node {
                        margin: UiRect::top(Val::Px(6.0)),
                        ..default()
                    },))
                        .with_child((
                            Text::new(format!("Cannot craft: {}", entry.reason)),
                            TextFont {
                                font_size: 13.0,
                                ..default()
                            },
                            TextColor(TEXT_RED),
                        ));
                }

                // Last craft message floats to bottom
                if let Some(msg) = &last_message {
                    det.spawn((Node {
                        flex_grow: 1.0,
                        align_items: AlignItems::End,
                        ..default()
                    },))
                        .with_child((
                            Text::new(msg.clone()),
                            TextFont {
                                font_size: 13.0,
                                ..default()
                            },
                            TextColor(ACCENT_CRAFTABLE),
                        ));
                }
            } else {
                det.spawn((
                    Text::new("Select a recipe"),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(TEXT_DIM),
                ));
            }
        });

    // ── Item detail panel ──────────────────────────────────────────────────
    commands
        .entity(item_detail)
        .despawn_children()
        .with_children(|d| {
            let result_id = focused_entry.as_ref().map(|e| e.result_id.as_str()).unwrap_or("");
            let def_entity = if result_id.is_empty() { None } else { def_world.entity_by_str(result_id) };

            let Some(def) = def_entity else {
                d.spawn((
                    Text::new("Item info"),
                    TextFont { font_size: 14.0, ..default() },
                    TextColor(TEXT_DIM),
                ));
                return;
            };

            // Name + ID header
            let name_str = focused_entry.as_ref().map(|e| e.result_name.as_str()).unwrap_or("");
            d.spawn((
                Text::new(name_str.to_string()),
                TextFont { font_size: 18.0, ..default() },
                TextColor(ACCENT),
            ));
            d.spawn((
                Text::new(format!("id: {}", result_id)),
                TextFont { font_size: 11.0, ..default() },
                TextColor(TEXT_ID),
            ));

            craft_divider(d);

            // Description
            if let Ok(desc) = defs.item_descs.get(def) {
                if !desc.0.is_empty() {
                    d.spawn((
                        Text::new(desc.0.clone()),
                        TextFont { font_size: 12.0, ..default() },
                        TextColor(TEXT_BRIGHT),
                    ));
                    craft_divider(d);
                }
            }

            // Basic properties
            craft_section_header(d, "Properties");

            let weight_g = defs.item_weights.get(def).map(|w| w.0).unwrap_or(0);
            let volume_ml = defs.item_volumes.get(def).map(|v| v.0).unwrap_or(0);
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
            if let Ok(sym) = defs.item_symbols.get(def) {
                craft_stat_row(d, "Symbol", &sym.0.to_string());
            }
            craft_stat_row(d, "Weight", &weight_str);
            craft_stat_row(d, "Volume", &volume_str);
            if let Ok(color) = defs.item_colors.get(def) { craft_stat_row(d, "Color", &color.0); }
            if let Ok(cat) = defs.item_categories.get(def) { craft_stat_row(d, "Category", &cat.0); }
            if let Ok(mats) = defs.item_materials.get(def) {
                if !mats.0.is_empty() { craft_stat_row(d, "Materials", &mats.0.join(", ")); }
            }
            if let Ok(phase) = defs.item_phases.get(def) {
                let phase_str = match phase.0 {
                    Phase::Solid => "Solid", Phase::Liquid => "Liquid",
                    Phase::Gas => "Gas", Phase::Plasma => "Plasma",
                };
                craft_stat_row(d, "Phase", phase_str);
            }

            // Qualities
            if let Ok(quals) = defs.item_qualities.get(def) {
                if !quals.0.is_empty() {
                    craft_divider(d);
                    craft_section_header(d, "Tool Qualities");
                    for (id, level) in &quals.0 {
                        craft_stat_row(d, id, &level.to_string());
                    }
                }
            }

            // Weapon
            if let Ok(w) = defs.weapon_data.get(def) {
                craft_divider(d);
                craft_section_header(d, "Melee");
                craft_stat_row(d, "Bash/Cut/Stab", &format!("{}/{}/{}", w.damage_bash, w.damage_cut, w.damage_stab));
                craft_stat_row(d, "To-hit", &w.to_hit.to_string());
                if !w.techniques.is_empty() { craft_stat_row(d, "Techniques", &w.techniques.join(", ")); }
            }

            // Gun
            if let Ok(g) = defs.gun_data.get(def) {
                craft_divider(d);
                craft_section_header(d, "Ranged");
                craft_stat_row(d, "Skill", &g.skill);
                craft_stat_row(d, "Ammo", &g.ammo_type);
                craft_stat_row(d, "Clip", &g.clip_size.to_string());
            }

            // Ammo
            if let Ok(a) = defs.ammo_data.get(def) {
                craft_divider(d);
                craft_section_header(d, "Ammo");
                craft_stat_row(d, "Type", &a.ammo_type);
                craft_stat_row(d, "Damage", &a.damage.to_string());
                craft_stat_row(d, "Range", &a.range.to_string());
            }

            // Magazine
            if let Ok(m) = defs.magazine_data.get(def) {
                craft_divider(d);
                craft_section_header(d, "Magazine");
                craft_stat_row(d, "Ammo type", &m.ammo_type);
                craft_stat_row(d, "Capacity", &m.capacity.to_string());
            }

            // Armour
            if let Ok(armour) = defs.armour_data.get(def) {
                craft_divider(d);
                craft_section_header(d, "Armour");
                for part in &armour.parts {
                    let covers_str = if part.body_part.is_empty() { "?".to_string() } else { part.body_part.clone() };
                    let layers_str = if part.layers.is_empty() { "NORMAL".to_string() } else { part.layers.join(", ") };
                    craft_stat_row(d, "Covers", &format!("{} [{}]", covers_str, layers_str));
                    craft_stat_row(d, "Coverage", &format!("{}%  enc {}", part.coverage, part.encumbrance));
                }
            }

            // Food
            if let Ok(food) = defs.food_data.get(def) {
                craft_divider(d);
                craft_section_header(d, "Food");
                craft_stat_row(d, "Type", &food.comestible_type);
                craft_stat_row(d, "Calories", &food.calories.to_string());
                craft_stat_row(d, "Quench", &food.quench.to_string());
            }

            // Tool
            if let Ok(tool) = defs.tool_data.get(def) {
                craft_divider(d);
                craft_section_header(d, "Tool");
                if tool.max_charges != 0 {
                    craft_stat_row(d, "Max charges", &tool.max_charges.to_string());
                }
                if let Some(at) = &tool.ammo_type { craft_stat_row(d, "Ammo type", at); }
            }

            // Container
            if let Ok(cont) = defs.container_data.get(def) {
                craft_divider(d);
                craft_section_header(d, "Pockets");
                for (idx, pocket) in cont.pockets.iter().enumerate() {
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
                    craft_stat_row(d, "Pocket", &format!("#{} {} — {} / {}", idx + 1, pocket.pocket_type, vol_str, wt_str));
                }
            }

            // Book
            if let Ok(book) = defs.book_data.get(def) {
                craft_divider(d);
                craft_section_header(d, "Book");
                craft_stat_row(d, "Skill", &book.skill);
                craft_stat_row(d, "Levels", &format!("{} → {}", book.required_level, book.max_level));
                craft_stat_row(d, "Fun", &book.fun.to_string());
            }
        });

    // ── Filter bar (bottom) ────────────────────────────────────────────────
    commands
        .entity(filter_bar)
        .despawn_children()
        .with_children(|fb| {
            let filter_bg = if filtering {
                Color::srgb(0.08, 0.18, 0.30)
            } else {
                Color::NONE
            };
            let filter_text = if filtering {
                format!("Filter: {}_", filter)
            } else if filter.is_empty() {
                "[/] to filter".to_string()
            } else {
                format!("Filter: {}  (Enter=close  Esc=clear)", filter)
            };
            fb.spawn((
                Node {
                    width: Val::Percent(100.0),
                    padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                    ..default()
                },
                BackgroundColor(filter_bg),
            ))
            .with_child((
                Text::new(filter_text),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(if filtering { TEXT_BRIGHT } else { TEXT_DIM }),
            ));
        });

    // ── Footer ────────────────────────────────────────────────────────────
    commands
        .entity(footer)
        .despawn_children()
        .with_child((
            Text::new(
                "[↑↓ / j,k] navigate   [←→] category   [Enter] craft   [a] all/craftable   [/] filter   [Esc] back",
            ),
            TextFont {
                font_size: 12.0,
                ..default()
            },
            TextColor(TEXT_DIM),
        ));
}

// ---------------------------------------------------------------------------
// crafting_menu_input — regular per-frame system
// ---------------------------------------------------------------------------

pub fn crafting_menu_input(
    mut reader: MessageReader<InputAction>,
    mut keyboard: MessageReader<KeyboardInput>,
    mut craft_state: ResMut<CraftState>,
    mut cat_index: ResMut<CategoryIndex>,
    mut stack: ResMut<ContextStack>,
    mut next_ctx: ResMut<NextState<Ctx>>,
    mut focused: ResMut<FocusedCommandIndex>,
    mut pending: ResMut<PendingCraft>,
    mut input_ctx: ResMut<InputContextStack>,
) {
    // ── Filter mode ──────────────────────────────────────────────────────
    if craft_state.filtering {
        for _ in reader.read() {}

        for ev in keyboard.read() {
            if ev.state == ButtonState::Released || ev.repeat {
                continue;
            }
            match &ev.logical_key {
                Key::Character(ch) if !ch.chars().any(|c| c.is_control()) => {
                    // Skip '/' if filter just opened (it was the toggle key)
                    if ch == "/" && craft_state.filter.is_empty() {
                        continue;
                    }
                    craft_state.filter.push_str(ch.as_str());
                    craft_state.focus = 0;
                }
                Key::Space => {
                    craft_state.filter.push(' ');
                    craft_state.focus = 0;
                }
                Key::Backspace => {
                    craft_state.filter.pop();
                    craft_state.focus = 0;
                }
                Key::Enter => {
                    craft_state.filtering = false;
                    input_ctx.pop();
                }
                Key::Escape => {
                    craft_state.filtering = false;
                    craft_state.filter.clear();
                    craft_state.focus = 0;
                    input_ctx.pop();
                }
                _ => {}
            }
        }
        return;
    }

    let actions: Vec<GameAction> = reader.read().map(|e| e.action.clone()).collect();
    if actions.is_empty() {
        return;
    }

    for action in actions {
        match action {
            // ── Filter ───────────────────────────────────────────────────
            GameAction::Filter => {
                if !craft_state.filtering {
                    craft_state.filtering = true;
                    input_ctx.push(InputContextId::TextInput);
                }
            }

            // ── Tab cycle focus zones ────────────────────────────────────
            GameAction::NavigateNextTab => {
                cat_index.focus_zone = (cat_index.focus_zone + 1) % 3;
                craft_state.focus = 0;
            }
            GameAction::NavigatePrevTab => {
                cat_index.focus_zone = if cat_index.focus_zone == 0 {
                    2
                } else {
                    cat_index.focus_zone - 1
                };
                craft_state.focus = 0;
            }

            // ── Zone-aware navigation ────────────────────────────────────
            GameAction::NavigateLeft => {
                if cat_index.focus_zone == 1 {
                    // Switch category left
                    let n = cat_index.top_categories.len();
                    if n > 0 {
                        cat_index.selected_top = if cat_index.selected_top == 0 {
                            n - 1
                        } else {
                            cat_index.selected_top - 1
                        };
                        cat_index.selected_sub = 0;
                        craft_state.focus = 0;
                    }
                } else if cat_index.focus_zone == 2 {
                    // Previous subcategory
                    let current_top = cat_index
                        .top_categories
                        .get(cat_index.selected_top)
                        .cloned()
                        .unwrap_or_default();
                    let subcats: Vec<String> = cat_index
                        .sub_recipes
                        .keys()
                        .filter(|(top, _)| top == &current_top)
                        .map(|(_, sub)| sub.clone())
                        .collect();
                    if !subcats.is_empty() && cat_index.selected_sub > 0 {
                        cat_index.selected_sub -= 1;
                        craft_state.focus = 0;
                    }
                }
            }
            GameAction::NavigateRight => {
                if cat_index.focus_zone == 1 {
                    // Switch category right
                    let n = cat_index.top_categories.len();
                    if n > 0 {
                        cat_index.selected_top =
                            (cat_index.selected_top + 1).min(n.saturating_sub(1));
                        cat_index.selected_sub = 0;
                        craft_state.focus = 0;
                    }
                } else if cat_index.focus_zone == 2 {
                    // Next subcategory
                    let current_top = cat_index
                        .top_categories
                        .get(cat_index.selected_top)
                        .cloned()
                        .unwrap_or_default();
                    let subcats: Vec<String> = cat_index
                        .sub_recipes
                        .keys()
                        .filter(|(top, _)| top == &current_top)
                        .map(|(_, sub)| sub.clone())
                        .collect();
                    if !subcats.is_empty()
                        && cat_index.selected_sub < subcats.len().saturating_sub(1)
                    {
                        cat_index.selected_sub += 1;
                        craft_state.focus = 0;
                    }
                }
            }

            // ── UP/DOWN: zone-specific ───────────────────────────────────
            GameAction::NavigateUp => {
                if cat_index.focus_zone == 0 {
                    // Recipe list: move focus up, clamped to category-filtered max
                    let current_top = cat_index
                        .top_categories
                        .get(cat_index.selected_top)
                        .cloned()
                        .unwrap_or_default();
                    let subcats: Vec<String> = cat_index
                        .sub_recipes
                        .keys()
                        .filter(|(top, _)| top == &current_top)
                        .map(|(_, sub)| sub.clone())
                        .collect();
                    let sel_sub = cat_index.selected_sub.min(subcats.len().saturating_sub(1));
                    let current_sub = subcats.get(sel_sub).cloned().unwrap_or_default();
                    let key = (current_top, current_sub);
                    let cat_recipes: Vec<Entity> =
                        cat_index.sub_recipes.get(&key).cloned().unwrap_or_default();
                    let cat_visible: Vec<&CraftEntry> = craft_state
                        .visible()
                        .filter(|e| cat_recipes.contains(&e.recipe_entity))
                        .collect();
                    let max = cat_visible.len().saturating_sub(1);
                    craft_state.focus = if craft_state.focus > max {
                        max
                    } else {
                        craft_state.focus.saturating_sub(1)
                    };
                } else if cat_index.focus_zone == 1 {
                    // On category tabs: move up to subcategory tabs
                    cat_index.focus_zone = 2;
                } else {
                    // On subcategory tabs: move up to category tabs
                    cat_index.focus_zone = 1;
                }
            }
            GameAction::NavigateDown => {
                if cat_index.focus_zone == 0 {
                    // Recipe list: move focus down
                    let current_top = cat_index
                        .top_categories
                        .get(cat_index.selected_top)
                        .cloned()
                        .unwrap_or_default();
                    let subcats: Vec<String> = cat_index
                        .sub_recipes
                        .keys()
                        .filter(|(top, _)| top == &current_top)
                        .map(|(_, sub)| sub.clone())
                        .collect();
                    let sel_sub = cat_index.selected_sub.min(subcats.len().saturating_sub(1));
                    let current_sub = subcats.get(sel_sub).cloned().unwrap_or_default();
                    let key = (current_top, current_sub);
                    let cat_recipes: Vec<Entity> =
                        cat_index.sub_recipes.get(&key).cloned().unwrap_or_default();
                    let cat_visible: Vec<&CraftEntry> = craft_state
                        .visible()
                        .filter(|e| cat_recipes.contains(&e.recipe_entity))
                        .collect();
                    let max = cat_visible.len().saturating_sub(1);
                    craft_state.focus = (craft_state.focus + 1).min(max);
                } else if cat_index.focus_zone == 1 {
                    // On category tabs: move down to subcategory tabs
                    cat_index.focus_zone = 2;
                } else {
                    // On subcategory tabs: move down to recipe list
                    cat_index.focus_zone = 0;
                }
            }

            GameAction::NavigateHome => {
                craft_state.focus = 0;
            }
            GameAction::NavigateEnd => {
                let current_top = cat_index
                    .top_categories
                    .get(cat_index.selected_top)
                    .cloned()
                    .unwrap_or_default();
                let subcats: Vec<String> = cat_index
                    .sub_recipes
                    .keys()
                    .filter(|(top, _)| top == &current_top)
                    .map(|(_, sub)| sub.clone())
                    .collect();
                let sel_sub = cat_index.selected_sub.min(subcats.len().saturating_sub(1));
                let current_sub = subcats.get(sel_sub).cloned().unwrap_or_default();
                let key = (current_top, current_sub);
                let cat_recipes: Vec<Entity> =
                    cat_index.sub_recipes.get(&key).cloned().unwrap_or_default();
                let cat_visible: Vec<&CraftEntry> = craft_state
                    .visible()
                    .filter(|e| cat_recipes.contains(&e.recipe_entity))
                    .collect();
                craft_state.focus = cat_visible.len().saturating_sub(1);
            }
            GameAction::NavigatePageUp => {
                let current_top = cat_index
                    .top_categories
                    .get(cat_index.selected_top)
                    .cloned()
                    .unwrap_or_default();
                let subcats: Vec<String> = cat_index
                    .sub_recipes
                    .keys()
                    .filter(|(top, _)| top == &current_top)
                    .map(|(_, sub)| sub.clone())
                    .collect();
                let sel_sub = cat_index.selected_sub.min(subcats.len().saturating_sub(1));
                let current_sub = subcats.get(sel_sub).cloned().unwrap_or_default();
                let key = (current_top, current_sub);
                let cat_recipes: Vec<Entity> =
                    cat_index.sub_recipes.get(&key).cloned().unwrap_or_default();
                let cat_visible: Vec<&CraftEntry> = craft_state
                    .visible()
                    .filter(|e| cat_recipes.contains(&e.recipe_entity))
                    .collect();
                let max = cat_visible.len().saturating_sub(1);
                craft_state.focus = if craft_state.focus > 10 {
                    craft_state.focus.saturating_sub(10)
                } else {
                    0
                };
                if craft_state.focus > max {
                    craft_state.focus = max;
                }
            }
            GameAction::NavigatePageDown => {
                let current_top = cat_index
                    .top_categories
                    .get(cat_index.selected_top)
                    .cloned()
                    .unwrap_or_default();
                let subcats: Vec<String> = cat_index
                    .sub_recipes
                    .keys()
                    .filter(|(top, _)| top == &current_top)
                    .map(|(_, sub)| sub.clone())
                    .collect();
                let sel_sub = cat_index.selected_sub.min(subcats.len().saturating_sub(1));
                let current_sub = subcats.get(sel_sub).cloned().unwrap_or_default();
                let key = (current_top, current_sub);
                let cat_recipes: Vec<Entity> =
                    cat_index.sub_recipes.get(&key).cloned().unwrap_or_default();
                let cat_visible: Vec<&CraftEntry> = craft_state
                    .visible()
                    .filter(|e| cat_recipes.contains(&e.recipe_entity))
                    .collect();
                let max = cat_visible.len().saturating_sub(1);
                craft_state.focus = (craft_state.focus + 10).min(max);
            }

            // ── Craft ───────────────────────────────────────────────────
            GameAction::Confirm => {
                // Find the focused entry within the current category/subcategory
                let current_top = cat_index
                    .top_categories
                    .get(cat_index.selected_top)
                    .cloned()
                    .unwrap_or_default();
                let subcats: Vec<String> = cat_index
                    .sub_recipes
                    .keys()
                    .filter(|(top, _)| top == &current_top)
                    .map(|(_, sub)| sub.clone())
                    .collect();
                let sel_sub = cat_index.selected_sub.min(subcats.len().saturating_sub(1));
                let current_sub = subcats.get(sel_sub).cloned().unwrap_or_default();
                let key = (current_top, current_sub);
                let cat_recipes: Vec<Entity> =
                    cat_index.sub_recipes.get(&key).cloned().unwrap_or_default();
                let cat_visible: Vec<&CraftEntry> = craft_state
                    .visible()
                    .filter(|e| cat_recipes.contains(&e.recipe_entity))
                    .collect();
                if let Some(entry) = cat_visible.get(craft_state.focus) {
                    if entry.craftable {
                        pending.0 = Some(entry.recipe_entity);
                    }
                }
            }

            // ── Toggle all/craftable ─────────────────────────────────────
            GameAction::HotkeyPress('a') => {
                craft_state.show_all = !craft_state.show_all;
                let max = craft_state.visible_count().saturating_sub(1);
                if craft_state.focus > max {
                    craft_state.focus = max;
                }
            }

            // ── Back ────────────────────────────────────────────────────
            GameAction::Cancel => {
                craft_state.last_message = None;
                pop_ctx(&mut stack, &mut next_ctx, &mut focused);
            }

            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// process_pending_craft — exclusive per-frame system
// ---------------------------------------------------------------------------

/// Drain `PendingCraft`, execute the craft, and rebuild `CraftState` so the
/// recipe list reflects the updated inventory.
pub fn process_pending_craft(world: &mut World) {
    let recipe_entity = {
        let mut pending = world.resource_mut::<PendingCraft>();
        pending.0.take()
    };
    let Some(recipe_entity) = recipe_entity else {
        return;
    };

    let Some(player) = find_dev_player(world) else {
        return;
    };

    match start_craft(world, player, recipe_entity) {
        Ok(craft_e) => {
            let result_name = world
                .get::<crate::core::components::item::InProgressCraft>(craft_e)
                .map(|c| c.result_name.clone())
                .unwrap_or_else(|| "item".to_string());
            tracing::info!("Started crafting: {}", result_name);
            if let Some(mut state) = world.get_resource_mut::<CraftState>() {
                state.last_message = Some(format!("Crafting: {}", result_name));
            }
        }
        Err(e) => {
            tracing::warn!("Craft failed: {}", e);
            if let Some(mut state) = world.get_resource_mut::<CraftState>() {
                state.last_message = Some(format!("Failed: {}", e));
            }
        }
    }

    // Rebuild craft state so craftability reflects the updated inventory.
    build_craft_state(world);
}

// ---------------------------------------------------------------------------
// UI helpers for crafting item detail pane
// ---------------------------------------------------------------------------

fn craft_divider(parent: &mut ChildSpawnerCommands) {
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

fn craft_section_header(parent: &mut ChildSpawnerCommands, title: &str) {
    parent.spawn((
        Text::new(title.to_uppercase()),
        TextFont { font_size: 11.0, ..default() },
        TextColor(LABEL),
    ));
}

fn craft_stat_row(parent: &mut ChildSpawnerCommands, label: &str, value: &str) {
    parent
        .spawn((Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(4.0),
            ..default()
        },))
        .with_children(|row| {
            row.spawn((
                Text::new(format!("{}: ", label)),
                TextFont { font_size: 12.0, ..default() },
                TextColor(TEXT_DIM),
                Node {
                    min_width: Val::Px(90.0),
                    ..default()
                },
            ));
            row.spawn((
                Text::new(value.to_string()),
                TextFont { font_size: 12.0, ..default() },
                TextColor(TEXT_BRIGHT),
            ));
        });
}
