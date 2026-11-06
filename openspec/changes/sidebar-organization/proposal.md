## Why

Feeder currently sorts sources by name and uses a fixed sidebar width. Migrating the existing Web organization controls lets readers retain their library arrangement across sessions without altering article state.

## What Changes

- Persist folder order and feed order/membership in the shared SQLite database, preserving existing visible order on migration.
- Provide atomic conflict-checked reorder/move operations and append newly created sources.
- Add pointer/touch handles, keyboard ordering and menu-based folder movement.
- Resize the desktop sidebar with accessible keyboard controls and per-browser safe width persistence.

## Capabilities

### New Capabilities

- `sidebar-organization`: Persistent source arrangement and accessible responsive sidebar sizing.

### Modified Capabilities

None.

## Impact

Rust schema migration and library API; React sidebar; Linux container acceptance and migration tests. No desktop/sync feature or legacy live-data access. Existing article IDs, visits, state and collection configuration are preserved.
