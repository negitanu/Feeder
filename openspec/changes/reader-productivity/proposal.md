## Why
Feeder needs shared reading history, precise filtering and scoped bulk actions to match daily Web reading workflows.
## What Changes
- Record actual detail visits separately from read/star state.
- Add deterministic ordering, bounded literal title/body/any/exclusion/date/source filters.
- Add prominent scoped 全て既読 and atomic selected-article updates.
- Preserve Linux container operation and known schema migration backups.
## Capabilities
### New Capabilities
- `reader-productivity`: History, detailed filters and scope-safe bulk state changes.
### Modified Capabilities
None. Legacy signed continuation, timezone settings and no-JavaScript UI compatibility remain separate migration work.
## Impact
Rust schema v4/API, React reader and Linux container tests; no production data conversion.
