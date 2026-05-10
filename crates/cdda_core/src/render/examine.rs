//! Item examine overlay — shown on top of inventory.
//!
//! Spawned on `OnEnter(Ctx::ItemExamine)`, auto-despawned via `DespawnOnExit`.
//! Shows full item details using the shared `spawn_item_detail` widget,
//! looking up the def entity from the runtime item's type ID.

use crate::context::ctx::Ctx;
use crate::core::components::item::{ItemTypeId, StackCount};
use crate::inventory::examine_resource::ExaminedItem;
use crate::data::def_world::DefinitionWorld;
use crate::render::item_detail::{spawn_item_detail, ItemDetailQueries};
use bevy::prelude::*;
use bevy_state::state_scoped::DespawnOnExit;

// ---------------------------------------------------------------------------
// Colours (match inventory palette)
// ---------------------------------------------------------------------------

const BG: Color = Color::srgb(0.04, 0.04, 0.06);
const OVERLAY_BG: Color = Color::srgb(0.08, 0.08, 0.14);
const ACCENT: Color = Color::srgb(0.85, 0.60, 0.15);
const TEXT_BRIGHT: Color = Color::srgb(0.95, 0.95, 0.95);
const TEXT_DIM: Color = Color::srgb(0.55, 0.55, 0.55);
const DIVIDER: Color = Color::srgb(0.20, 0.20, 0.25);

// ---------------------------------------------------------------------------
// Spawn (OnEnter)
// ---------------------------------------------------------------------------

pub fn spawn_examine_overlay(
    mut commands: Commands,
    examined: Res<ExaminedItem>,
    def_world: Res<DefinitionWorld>,
    item_type_ids: Query<&ItemTypeId>,
    item_counts: Query<&StackCount>,
    detail: ItemDetailQueries,
) {
    let Some(item_entity) = examined.0 else {
        return;
    };

    let type_id = item_type_ids
        .get(item_entity)
        .map(|t| t.0.as_str())
        .unwrap_or("");
    let qty = item_counts.get(item_entity).map(|s| s.get()).unwrap_or(1);

    let def_entity = if type_id.is_empty() {
        None
    } else {
        def_world.entity_by_str(type_id)
    };

    commands
        .spawn((
            DespawnOnExit(Ctx::ItemExamine),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(24.0)),
                ..default()
            },
            BackgroundColor(BG),
        ))
        .with_children(|root| {
            // ── Title ─────────────────────────────────────────────────────
            root.spawn((
                Node {
                    width: Val::Percent(100.0),
                    padding: UiRect::axes(Val::Px(16.0), Val::Px(10.0)),
                    ..default()
                },
                BackgroundColor(OVERLAY_BG),
            ))
            .with_child((
                Text::new(format!("{} — DETAILS", type_id)),
                TextFont {
                    font_size: 28.0,
                    ..default()
                },
                TextColor(ACCENT),
            ));

            // ── Runtime info ──────────────────────────────────────────────
            if qty > 1 {
                root.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                        ..default()
                    },
                ))
                .with_child((
                    Text::new(format!("Stack:  {}", qty)),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(TEXT_BRIGHT),
                ));
            }

            // ── Divider before detail ─────────────────────────────────────
            root.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(1.0),
                    margin: UiRect::vertical(Val::Px(4.0)),
                    ..default()
                },
                BackgroundColor(DIVIDER),
            ));

            // ── Item details from def entity ──────────────────────────────
            if let Some(def) = def_entity {
                root.spawn((Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    overflow: Overflow::clip(),
                    flex_grow: 1.0,
                    ..default()
                },))
                .with_children(|d| {
                    spawn_item_detail(d, type_id, type_id, def, &detail);
                });
            } else {
                root.spawn((
                    Text::new("(no definition data)"),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(TEXT_DIM),
                ));
            }

            // ── Footer hints ──────────────────────────────────────────────
            root.spawn((Node {
                width: Val::Percent(100.0),
                flex_grow: 0.0,
                align_items: AlignItems::End,
                ..default()
            },))
                .with_child((
                    Text::new("[Esc / q / Enter] close"),
                    TextFont {
                        font_size: 15.0,
                        ..default()
                    },
                    TextColor(TEXT_DIM),
                ));
        });
}
