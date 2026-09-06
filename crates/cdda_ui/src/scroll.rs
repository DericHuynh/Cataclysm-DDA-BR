//! Reusable, idiomatic scroll primitives for Bevy UI.
//!
//! Everything here is a thin wrapper over what Bevy 0.18 already provides —
//! `Overflow::scroll_y()` + a `ScrollPosition(Vec2)` that the layout system
//! clamps against `ComputedNode::content_size()` — so panes get native
//! clipping, clamped scroll, and scroll-wheel support instead of the old
//! "hand-window content and copy a scalar" pattern.
//!
//! Usage: spawn a scroll container with
//! `(KeyboardScroll, ScrollPosition::default(), Node { overflow: Overflow::scroll_y(), .. })`
//! and register the systems here. Arrow keys and the mouse wheel both drive it.

use bevy_ecs::prelude::*;
use bevy_picking::events::Pointer;
use bevy_picking::events::Scroll;
use bevy_ui::{AlignItems, ComputedNode, Node, Overflow, ScrollPosition, Val};

/// Marker: this scroll container is navigable by the arrow/page keys and the
/// mouse wheel.
#[derive(Component, Default)]
pub struct KeyboardScroll;

/// Disables keyboard scrolling while retaining mouse-wheel access.
#[derive(Component)]
pub struct InactiveScrollPane;

/// Per-pane virtualized-list config for item-heavy scroll panes.
///
/// Bevy's `ScrollPosition` scroll **clips but still lays out every child**, so a
/// 40k-row pane is O(n) nodes every frame. This enables **spacer-based
/// virtualization**: the layout owns no row-windowing per child — instead, each
/// pane spawns the rows in `window` plus a tall top spacer (so native scroll
/// position maps to the item index) and a bottom spacer to fill the remaining
/// content. Only ~viewport+overscan rows exist at a time.
///
/// Attach this (plus `KeyboardScroll` and a `ScrollPosition`) to the pane, and
/// tell the pane the current `window` before it builds each frame. Call
/// [`update_virtual_windows`] after focus scrolling to keep `window` synced to the
/// native `ScrollPosition`.
#[derive(Component, Debug, Clone, Copy)]
pub struct VirtualList {
    /// Height of one row in logical px.
    pub row_height: f32,
    /// Total number of rows in the pane's data.
    pub total_rows: usize,
    /// Extra rows kept rendered above/below the viewport (smooth edge scroll).
    pub overscan_rows: usize,
    /// Last computed visible row window `[start, end)` — set by
    /// [`update_virtual_windows`], read while building the frame.
    pub window: (usize, usize),
}

impl Default for VirtualList {
    fn default() -> Self {
        Self {
            row_height: ROW_PX,
            total_rows: 0,
            overscan_rows: 4,
            window: (0, 0),
        }
    }
}

/// Updates every [`VirtualList`]'s `window` from its `ScrollPosition` + measured
/// viewport, so a pane can build only the visible rows that frame. The following presentation update consumes the new window.
pub fn update_virtual_windows(
    mut q: Query<(&mut VirtualList, &ScrollPosition, Option<&ComputedNode>)>,
) {
    for (mut list, pos, computed) in &mut q {
        let height = viewport_height(computed, list.row_height);
        let window = list.visible_window(pos.0.y, height);
        if list.window != window {
            list.window = window;
        }
    }
}

fn viewport_height(computed: Option<&ComputedNode>, row_height: f32) -> f32 {
    computed
        .map(|c| c.size().y * c.inverse_scale_factor())
        .filter(|h| *h > 0.0)
        .unwrap_or(row_height * VIEWPORT_DEFAULT_ROWS)
}

impl VirtualList {
    pub fn visible_window(&self, offset: f32, viewport: f32) -> (usize, usize) {
        if self.total_rows == 0 {
            return (0, 0);
        }
        let row = self.row_height.max(1.0);
        let viewport = viewport.max(row);
        let offset = offset.clamp(0.0, (self.total_rows as f32 * row - viewport).max(0.0));
        let start = (offset / row) as usize;
        (
            start.saturating_sub(self.overscan_rows),
            (((offset + viewport) / row).ceil() as usize)
                .saturating_add(self.overscan_rows)
                .min(self.total_rows),
        )
    }
}

/// Height (in px) that the visible window's `start` offset occupies, for the top
/// spacer. Keeps even a fully-virtualized pane's scroll position aligned to the
/// data index.
pub fn virtual_top_spacer_px(list: &VirtualList) -> f32 {
    list.window.0 as f32 * list.row_height
}

/// Height (in px) that the not-yet-rendered rows below the window occupy, for
/// the bottom spacer (so scroll range = full data height).
pub fn virtual_bottom_spacer_px(list: &VirtualList) -> f32 {
    (list.total_rows.saturating_sub(list.window.1)) as f32 * list.row_height
}

/// The number of rows currently expected to be rendered (window length).
pub fn virtual_window_len(list: &VirtualList) -> usize {
    list.window.1.saturating_sub(list.window.0)
}

/// Per-viewport factor used when no `ComputedNode` is available yet.
const VIEWPORT_DEFAULT_ROWS: f32 = 20.0;

/// Holds the currently-focused row index for a pane so [`scroll_to_focused_row`]
/// can keep it visible. Insert one (per pane entity) alongside `KeyboardScroll`.
#[derive(Component, Default, Debug, Clone, Copy)]
pub struct FocusedRow(pub usize);

/// Scroll step for a single arrow press, in logical pixels.
const ROW_PX: f32 = 34.0;

/// Maximum vertical scroll offset from layout data (content taller than view).
fn max_scroll_y(content: f32, view: f32, scale: f32) -> f32 {
    (content - view).max(0.0) * scale
}

/// Keep the focused row visible: when the row's index position leaves the
/// visible window, scroll just enough to bring it back. Reads a [`FocusedRow`]
/// component stored on the pane entity.
pub fn scroll_to_focused_row(
    mut q: Query<
        (
            &mut ScrollPosition,
            Ref<FocusedRow>,
            Option<&VirtualList>,
            Option<&ComputedNode>,
        ),
        With<KeyboardScroll>,
    >,
) {
    for (mut pos, focused, list, computed) in &mut q {
        if !focused.is_changed() {
            continue;
        }
        let row = list.map_or(ROW_PX, |l| l.row_height);
        let view = viewport_height(computed, row);
        let top = focused.0 as f32 * row;
        let next = if top < pos.0.y {
            top
        } else if top + row > pos.0.y + view {
            top + row - view
        } else {
            pos.0.y
        };
        if next != pos.0.y {
            pos.0.y = next.max(0.0);
        }
    }
}

/// Global mouse-wheel handler: reads `Pointer<Scroll>` picking events (enabled
/// by the `ui_picking` feature) and scrolls the hovered `KeyboardScroll` node,
/// clamped to its content size. Mirrors the reference `on_scroll_handler`.
pub fn scroll_with_wheel(
    mut scroll_events: MessageReader<Pointer<Scroll>>,
    parents: Query<&ChildOf>,
    mut q: Query<(&mut ScrollPosition, &ComputedNode), With<KeyboardScroll>>,
) {
    for ev in scroll_events.read() {
        // `ev.entity` is the event target (the node under the cursor).
        let mut target = ev.entity;
        while !q.contains(target) {
            let Ok(parent) = parents.get(target) else {
                break;
            };
            target = parent.parent();
        }
        let Ok((mut pos, computed)) = q.get_mut(target) else {
            continue;
        };
        let max_y = max_scroll_y(
            computed.content_size().y,
            computed.size().y,
            computed.inverse_scale_factor(),
        );
        let delta = -ev.event.y
            * match ev.event.unit {
                bevy_input::mouse::MouseScrollUnit::Line => ROW_PX,
                bevy_input::mouse::MouseScrollUnit::Pixel => 1.0,
            };
        let new_y = (pos.0.y + delta).clamp(0.0, max_y);
        if (new_y - pos.0.y).abs() > f32::EPSILON {
            pos.0.y = new_y;
        }
    }
}

/// Synchronize a retained list before building rows. Headers must be siblings of
/// this pane, since every child here contributes to the fixed-height row grid.
pub fn sync_virtual_pane(
    list: &mut Mut<VirtualList>,
    focus: &mut Mut<FocusedRow>,
    position: &mut Mut<ScrollPosition>,
    computed: &ComputedNode,
    total: usize,
    selected: usize,
    reset: bool,
) {
    let selected = selected.min(total.saturating_sub(1));
    let reveal = reset || focus.0 != selected || list.total_rows != total;
    if list.total_rows != total {
        list.total_rows = total;
    }
    if focus.0 != selected {
        focus.0 = selected;
    }
    let view = viewport_height(Some(computed), list.row_height);
    let max = (total as f32 * list.row_height - view).max(0.0);
    let mut offset = if reset {
        0.0
    } else {
        position.y.clamp(0.0, max)
    };
    if reveal {
        let top = selected as f32 * list.row_height;
        if top < offset {
            offset = top;
        } else if top + list.row_height > offset + view {
            offset = top + list.row_height - view;
        }
    }
    offset = offset.clamp(0.0, max);
    if position.y != offset {
        position.y = offset;
    }
    let window = list.visible_window(offset, view);
    if list.window != window {
        list.window = window;
    }
}

impl VirtualList {
    /// Exact row geometry shared by all virtualized menus.
    pub fn row_node(&self) -> Node {
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(self.row_height),
            flex_shrink: 0.0,
            overflow: Overflow::clip(),
            align_items: AlignItems::Center,
            ..Default::default()
        }
    }

    /// Build only the visible rows between non-shrinking spacers.
    pub fn spawn_window(
        &self,
        parent: &mut ChildSpawnerCommands,
        mut row: impl FnMut(&mut ChildSpawnerCommands, usize),
    ) {
        let spacer = |height| Node {
            height: Val::Px(height),
            flex_shrink: 0.0,
            ..Default::default()
        };
        if self.window.0 > 0 {
            parent.spawn(spacer(virtual_top_spacer_px(self)));
        }
        for index in self.window.0..self.window.1 {
            row(parent, index);
        }
        if self.window.1 < self.total_rows {
            parent.spawn(spacer(virtual_bottom_spacer_px(self)));
        }
    }
}
