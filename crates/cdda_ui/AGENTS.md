# cdda_ui

## Purpose
Reusable ECS UI primitives independent of gameplay, screen contexts, and keybindings.

## Ownership
`src/scroll.rs`: virtual row geometry, spacer construction, selection reveal,
mouse-wheel ancestor routing, native scroll positions and window synchronization.
`src/rows.rs`: keyed retained text rows, per-cell updates and bounded row recycling.
`src/rows_tests.rs`: headless retention, change-tick, recycling and lifetime regressions.

## Local Contracts
- No cdda gameplay/import/presenter dependencies and no GameAction or Ctx matching.
- Presenters own models and map input into focus/scroll components.
- Fixed headers are siblings of the scroll pane. Virtual rows/spacers never shrink.
- Use logical pixels; manual scrolling does not reselect or reveal unchanged focus.
- Hosts register focus reveal followed by window synchronization before UI layout.
- RetainedRows owns every child of its pane, including two permanent spacers. Sync only the visible window with unique model keys; do not despawn its children externally.
- RowKey holds the current model identity. Pooled entities may change keys: attach static markers once and resolve interaction through RowKey, never captured key-specific observers.
- Unchanged row/cell values receive no writes; overlapping keys retain entities, outgoing rows are recycled, and shrinking windows despawn surplus children.

## Work Guidance
Keep widget state on pane entities. Change components only when values differ.

## Verification
`cargo nextest run -p cdda_ui -p cdda_render` runs widget unit tests and production headless screen/layout regressions.

## Child DOX Index
None.
