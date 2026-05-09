//! Item examine overlay — shown on top of inventory.
//!
//! Spawned on `OnEnter(Ctx::ItemExamine)`, auto-despawned via `DespawnOnExit`.
//! Shows full item details including name, type, stack count, volume, and
//! qualities.  Reads the examined entity from the `ExaminedItem` resource.

use crate::context::ctx::Ctx;
use crate::core::components::def::ItemSymbol;
use crate::core::components::def::ItemVolume;
use crate::core::components::item::{ItemQualities, ItemTypeId, StackCount};
use crate::inventory::examine_resource::ExaminedItem;
use crate::worldgen::dev::DevGroundItemName;
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
const LABEL: Color = Color::srgb(0.50, 0.75, 0.90);
const DIVIDER: Color = Color::srgb(0.20, 0.20, 0.25);

// ---------------------------------------------------------------------------
// Spawn (OnEnter)
// ---------------------------------------------------------------------------

pub fn spawn_examine_overlay(
    mut commands: Commands,
    examined: Res<ExaminedItem>,
    item_names: Query<&DevGroundItemName>,
    item_type_ids: Query<&ItemTypeId>,
    item_counts: Query<&StackCount>,
    item_volumes: Query<&ItemVolume>,
    item_symbols: Query<&ItemSymbol>,
    item_qualities: Query<&ItemQualities>,
) {
    let Some(item_entity) = examined.0 else {
        return;
    };

    let name = item_names
        .get(item_entity)
        .map(|n| n.0.as_str())
        .unwrap_or("?");
    let type_id = item_type_ids
        .get(item_entity)
        .map(|t| t.0.as_str())
        .unwrap_or("");
    let qty = item_counts.get(item_entity).map(|s| s.get()).unwrap_or(1);
    let vol_ml = item_volumes.get(item_entity).map(|v| v.0).unwrap_or(0);
    let sym = item_symbols.get(item_entity).map(|s| s.0).unwrap_or('?');
    let qualities: Vec<String> = item_qualities
        .get(item_entity)
        .map(|q| {
            q.0.iter()
                .map(|(id, level)| format!("  {}  (x{})", id, level))
                .collect()
        })
        .unwrap_or_default();

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
                Text::new(format!("{} — DETAILS", name)),
                TextFont {
                    font_size: 28.0,
                    ..default()
                },
                TextColor(ACCENT),
            ));

            // ── Detail fields ─────────────────────────────────────────────
            let details = vec![
                (
                    "Info",
                    vec![
                        format!("Symbol:      {}", sym),
                        format!("Type ID:     {}", type_id),
                        format!("Stack:       {}", qty),
                        format!("Volume:      {} mL", vol_ml),
                    ],
                ),
                (
                    "Quality",
                    if qualities.is_empty() {
                        vec!["(none)".to_string()]
                    } else {
                        qualities.clone()
                    },
                ),
            ];

            for (section_title, lines) in &details {
                // Section header
                root.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                        border: UiRect::bottom(Val::Px(1.0)),
                        ..default()
                    },
                    BorderColor::all(DIVIDER),
                ))
                .with_child((
                    Text::new(*section_title),
                    TextFont {
                        font_size: 18.0,
                        ..default()
                    },
                    TextColor(LABEL),
                ));

                for line in lines {
                    root.spawn((Node {
                        width: Val::Percent(100.0),
                        padding: UiRect::axes(Val::Px(16.0), Val::Px(3.0)),
                        ..default()
                    },))
                        .with_child((
                            Text::new(line.clone()),
                            TextFont {
                                font_size: 16.0,
                                ..default()
                            },
                            TextColor(TEXT_BRIGHT),
                        ));
                }
            }

            // ── Footer hints ──────────────────────────────────────────────
            root.spawn((Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
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
