//! Retained, keyed text rows. The pane owns all row/spacer state and lifetimes.
#[cfg(test)]
#[path = "rows_tests.rs"]
mod tests;
use crate::VirtualList;
use bevy_color::Color;
use bevy_ecs::prelude::*;
use bevy_text::{TextColor, TextFont};
use bevy_ui::{widget::Text, BackgroundColor, BorderColor, Node, Val};
use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

#[derive(Clone, PartialEq)]
pub struct RowCell {
    pub text: String,
    pub font: TextFont,
    pub color: Color,
    pub grow: f32,
}
impl RowCell {
    pub fn new(text: impl Into<String>, size: f32, color: Color) -> Self {
        Self {
            text: text.into(),
            font: TextFont {
                font_size: size,
                ..Default::default()
            },
            color,
            grow: 0.0,
        }
    }
}
#[derive(Clone, PartialEq)]
pub struct TextRow {
    pub node: Node,
    pub background: Color,
    pub border: Color,
    pub cells: Vec<RowCell>,
}
struct RetainedRow {
    entity: Entity,
    cells: Vec<Entity>,
    value: Option<TextRow>,
}
/// Current model identity of a pooled row. Resolve clicks through this component;
/// an entity may represent a different key after scrolling.
#[derive(Component)]
pub struct RowKey<K: Send + Sync + 'static>(pub K);
/// Keys identify model rows, not interaction targets. Recycled entities must not
/// carry key-specific observers: route interactions through current model keys.
#[derive(Component)]
pub struct RetainedRows<K: Send + Sync + 'static> {
    rows: HashMap<K, RetainedRow>,
    spacers: Option<(Entity, Entity, f32, f32)>,
    order: Vec<Entity>,
}
impl<K: Send + Sync + 'static> Default for RetainedRows<K> {
    fn default() -> Self {
        Self {
            rows: HashMap::new(),
            spacers: None,
            order: Vec::new(),
        }
    }
}
impl<K: Eq + Hash + Clone + Send + Sync + 'static> RetainedRows<K> {
    /// Synchronize only the visible window, with O(viewport) state and work.
    /// Unchanged rows/cells receive no component writes. Overlapping keys keep
    /// their entities; leaving rows are recycled before allocating new ones.
    pub fn sync(
        &mut self,
        commands: &mut Commands,
        pane: Entity,
        list: &VirtualList,
        values: impl IntoIterator<Item = (K, TextRow)>,
    ) -> Vec<Entity> {
        let values: Vec<_> = values.into_iter().collect();
        let keys: HashSet<_> = values.iter().map(|(k, _)| k.clone()).collect();
        assert_eq!(keys.len(), values.len(), "visible row keys must be unique");
        let mut reusable = Vec::new();
        self.rows.retain(|key, row| {
            if keys.contains(key) {
                true
            } else {
                reusable.push(RetainedRow {
                    entity: row.entity,
                    cells: std::mem::take(&mut row.cells),
                    value: row.value.take(),
                });
                false
            }
        });
        // Deterministic recycling independent of hash iteration order.
        reusable.sort_by_key(|row| row.entity.to_bits());
        let top = crate::virtual_top_spacer_px(list);
        let bottom = crate::virtual_bottom_spacer_px(list);
        let (top_entity, bottom_entity, old_top, old_bottom) =
            *self.spacers.get_or_insert_with(|| {
                let a = commands.spawn(spacer(top)).id();
                let b = commands.spawn(spacer(bottom)).id();
                (a, b, top, bottom)
            });
        if old_top != top {
            commands.entity(top_entity).insert(spacer(top));
        }
        if old_bottom != bottom {
            commands.entity(bottom_entity).insert(spacer(bottom));
        }
        self.spacers = Some((top_entity, bottom_entity, top, bottom));
        let mut order = vec![top_entity];
        let mut created = Vec::new();
        for (key, value) in values {
            let row = self.rows.entry(key.clone()).or_insert_with(|| {
                let row = reusable.pop().unwrap_or_else(|| {
                    let entity = commands.spawn_empty().id();
                    created.push(entity);
                    RetainedRow {
                        entity,
                        cells: Vec::new(),
                        value: None,
                    }
                });
                commands.entity(row.entity).insert(RowKey(key));
                row
            });
            sync_row(commands, row, value);
            order.push(row.entity);
        }
        order.push(bottom_entity);
        // Reorder only when membership/order changes, never for focus/style updates.
        if self.order != order {
            commands.entity(pane).replace_children(&order);
            self.order = order;
        }
        for row in reusable {
            commands.entity(row.entity).despawn();
        }
        created
    }
    pub fn entity(&self, key: &K) -> Option<Entity> {
        self.rows.get(key).map(|r| r.entity)
    }
}
fn spacer(height: f32) -> Node {
    Node {
        height: Val::Px(height),
        min_height: Val::Px(height),
        flex_shrink: 0.0,
        ..Default::default()
    }
}
fn sync_row(commands: &mut Commands, row: &mut RetainedRow, value: TextRow) {
    let old = row.value.as_ref();
    if old.is_some_and(|old| old == &value) {
        return;
    }
    if old.is_none_or(|old| old.node != value.node) {
        commands.entity(row.entity).insert(value.node.clone());
    }
    if old.is_none_or(|old| old.background != value.background) {
        commands
            .entity(row.entity)
            .insert(BackgroundColor(value.background));
    }
    if old.is_none_or(|old| old.border != value.border) {
        commands
            .entity(row.entity)
            .insert(BorderColor::all(value.border));
    }
    for (i, cell) in value.cells.iter().enumerate() {
        let entity = if let Some(&entity) = row.cells.get(i) {
            entity
        } else {
            let entity = commands.spawn_empty().id();
            commands.entity(row.entity).add_child(entity);
            row.cells.push(entity);
            entity
        };
        let previous = old.and_then(|old| old.cells.get(i));
        if previous.is_none_or(|c| c.text != cell.text) {
            commands.entity(entity).insert(Text::new(cell.text.clone()));
        }
        if previous.is_none_or(|c| c.font != cell.font) {
            commands.entity(entity).insert(cell.font.clone());
        }
        if previous.is_none_or(|c| c.color != cell.color) {
            commands.entity(entity).insert(TextColor(cell.color));
        }
        if previous.is_none_or(|c| c.grow != cell.grow) {
            commands.entity(entity).insert(Node {
                flex_grow: cell.grow,
                ..Default::default()
            });
        }
    }
    for entity in row.cells.drain(value.cells.len()..) {
        commands.entity(entity).despawn();
    }
    row.value = Some(value);
}
