## Why

Feeder's detail footer already navigates the captured loaded article order. The reader backlog still identifies direct keyboard navigation as a gap. This extends the existing Web reader without changing storage or loading additional pages.

## What Changes

Left and right arrow keys navigate to the previous and next captured article while focus is inside the detail panel. Preserve existing visit, filtering, failure, focus, and stale-response behavior. Text editing, modified keys, repeated keydown and unavailable endpoints must not initiate visits.

## Capabilities

### New Capabilities
- `reader-detail-arrow-keys`: Scoped keyboard navigation in article detail.

### Modified Capabilities

## Impact

React reader and generated-data browser acceptance; no API, schema, or network boundary changes. The legacy reader interaction and panel specifications are tracked in the source inventory; this is an additional Feeder keyboard interaction alongside their existing detail controls.
