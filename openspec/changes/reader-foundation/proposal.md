## Why
Feeder needs a testable first Web implementation before broader migration.
## What Changes
- Add Rust HTTP/SQLite backend and React/TypeScript reader with generated sample data.
- Support article browsing, explicit read/star mutations, persistent state and health checks.
- Refuse existing FeedAstrolabe databases rather than modifying them without migration proof.
## Capabilities
### New Capabilities
- `reader-foundation`: Local development reader vertical slice and security boundaries.
### Modified Capabilities
None.
## Impact
New backend, frontend, tests and development instructions. Production cutover remains gated by the migration plan.
