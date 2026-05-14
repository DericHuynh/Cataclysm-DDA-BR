//! Item examine overlay — shown on top of inventory.
//!
//! Spawned on `OnEnter(Ctx::ItemExamine)`, auto-despawned via `DespawnOnExit`.
//! Shows full item details using the shared `spawn_item_detail` widget,
//! looking up the def entity from the runtime item's type ID.

use crate::context::ctx::Ctx;
use crate::context::screen::CddaScreen;
use crate::context::ContextActions;
use crate::data::def_world::DefinitionWorld;
use crate::data::interner::ItemTypeRegistry;
use crate::data::interner::QualityRegistry;
use crate::input::{ActiveKeybindings, BindableAction};
use crate::inventory::examine_resource::ExaminedItem;
use crate::render::item_detail::ItemDetailSnapshot;
use crate::render::theme::{self, UiTheme};
use bevy::prelude::*;
use bevy_state::state_scoped::DespawnOnExit;
use cdda_components::item::{ItemTypeId, StackCount};

// ---------------------------------------------------------------------------
// CddaScreen trait impl
// ---------------------------------------------------------------------------

pub struct ExamineScreen;

impl CddaScreen for ExamineScreen {
    const CTX: Ctx = Ctx::ItemExamine;
    const ACTIONS: &'static [(&'static str, BindableAction)] = &[
        ("drop", BindableAction::Drop),
        ("wield", BindableAction::UseItem),
    ];

    fn spawn(world: &mut World) {
        spawn_examine_from_world(world);
    }
}

fn spawn_examine_from_world(world: &mut World) {
    // ── Phase 1: extract data from world ────────────────────────────────
    let theme = world.resource::<UiTheme>().clone();
    let examined_opt = world.resource::<ExaminedItem>().0;
    let Some(item_entity) = examined_opt else {
        return;
    };

    let type_id: String = {
        let mut q = world.query::<&ItemTypeId>();
        q.get(world, item_entity)
            .map(|t| {
                world
                    .resource::<ItemTypeRegistry>()
                    .resolve(t.0)
                    .unwrap_or("?")
                    .to_string()
            })
            .unwrap_or_default()
    };
    let qty: u32 = {
        let mut q = world.query::<&StackCount>();
        q.get(world, item_entity).map(|s| s.get()).unwrap_or(1)
    };

    let def_entity: Option<Entity> = if type_id.is_empty() {
        None
    } else {
        world.resource::<DefinitionWorld>().entity_by_str(&type_id)
    };

    let ctx_actions_actions = world.resource::<ContextActions>().actions.clone();
    let active_keys = world.resource::<ActiveKeybindings>().clone();
    let font_handle = world.resource::<super::UiFontHandle>().0.clone();

    // Pre-extract item detail data from the def entity
    let detail_data: Option<ItemDetailSnapshot> =
        def_entity.map(|def| ItemDetailSnapshot::extract(world, def));
    let quality_registry = world.resource::<QualityRegistry>().clone();

    // ── Phase 2: build UI ───────────────────────────────────────────────
    let mut cmds = world.commands();
    let type_id_ref = type_id.clone();

    cmds.spawn((
        DespawnOnExit(Ctx::ItemExamine),
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(24.0)),
            ..default()
        },
        BackgroundColor(theme::BG),
    ))
    .with_children(|root| {
        // ── Title ─────────────────────────────────────────────────────
        root.spawn((
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::axes(Val::Px(16.0), Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(theme::TAB_BG),
        ))
        .with_child((
            Text::new(format!("{} — DETAILS", type_id_ref)),
            TextFont {
                font_size: 28.0,
                ..default()
            },
            TextColor(theme.accent2()),
        ));

        // ── Runtime info ──────────────────────────────────────────────
        if qty > 1 {
            root.spawn((Node {
                width: Val::Percent(100.0),
                padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                ..default()
            },))
                .with_child((
                    Text::new(format!("Stack:  {}", qty)),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(theme::TEXT_BRIGHT),
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
            BackgroundColor(theme::DIVIDER),
        ));

        // ── Item details from def entity ──────────────────────────────
        if let Some(ref snapshot) = detail_data {
            root.spawn((Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                overflow: Overflow::clip(),
                flex_grow: 1.0,
                ..default()
            },))
                .with_children(|d| {
                    snapshot.spawn_into(d, &quality_registry);
                });
        } else {
            root.spawn((
                Text::new("(no definition data)"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(theme::TEXT_DIM),
            ));
        }

        // ── Footer hints ──────────────────────────────────────────────
        let mut hints = String::from("[Esc] close");
        for entry in &ctx_actions_actions {
            let key = active_keys.key_for(entry.action);
            hints.push_str(&format!("  [{}] {}", key, entry.label));
        }

        root.spawn((Node {
            width: Val::Percent(100.0),
            flex_grow: 0.0,
            align_items: AlignItems::End,
            ..default()
        },))
            .with_child((
                Text::new(hints),
                super::ui_font(&font_handle, 15.0),
                TextColor(theme::TEXT_DIM),
            ));
    });
}
