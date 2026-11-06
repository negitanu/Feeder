## Why
Settings need a discoverable entry and OPML operations must remain independent of feed discovery.
## What Changes
Add a settings navigation entry while retaining existing collection settings access. Isolate OPML busy/error state, prevent changes during import, support preview cancellation, and label preview statuses in Japanese.
## Capabilities
### New Capabilities
- `settings-opml-workflow`: Discoverable settings and recoverable OPML operations.
### Modified Capabilities
## Impact
React only. Existing OPML APIs, paused imports, limits and duplicate handling remain the source of truth. Corresponds to legacy configuration-export and feed-collection through existing Feeder settings and OPML APIs.
