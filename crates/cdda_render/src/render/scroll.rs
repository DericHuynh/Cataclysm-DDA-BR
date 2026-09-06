//! Game-action adapter for reusable cdda_ui scrolling primitives.
use bevy::prelude::*;
use cdda_input::{GameAction, InputAction};
pub use cdda_ui::*;
const ROW_PX: f32 = 34.0;
const PAGE_ROWS: f32 = 10.0;
fn max_scroll_y(content: f32, view: f32, scale: f32) -> f32 {
    (content - view).max(0.0) * scale
}
/// Arrow keys convert the UI action stream into a clamped `ScrollPosition`.
///
/// Skips nodes that carry a [`FocusedRow`] — those are list panes whose
/// per-row focus is owned by [`scroll_to_focused_row`] (arrow keys move the
/// focused row, which then scrolls itself into view). This system handles
/// plain free-scrolling panes (e.g. a text/JSON dump) where there is no row
/// index.
pub fn scroll_with_keyboard(
    mut actions: MessageReader<InputAction>,
    mut q: Query<
        (&mut ScrollPosition, &ComputedNode),
        (
            With<KeyboardScroll>,
            Without<FocusedRow>,
            Without<InactiveScrollPane>,
        ),
    >,
) {
    for action in actions.read() {
        let step = match action.action {
            GameAction::NavigateUp => -ROW_PX,
            GameAction::NavigateDown => ROW_PX,
            GameAction::NavigatePageUp => -PAGE_ROWS * ROW_PX,
            GameAction::NavigatePageDown => PAGE_ROWS * ROW_PX,
            GameAction::NavigateHome => -f32::INFINITY,
            _ => continue,
        };
        for (mut pos, computed) in &mut q {
            let max_y = max_scroll_y(
                computed.content_size().y,
                computed.size().y,
                computed.inverse_scale_factor(),
            );
            let new_y = (pos.0.y + step).clamp(0.0, max_y);
            if (new_y - pos.0.y).abs() > f32::EPSILON {
                pos.0.y = new_y;
            }
        }
    }
}
