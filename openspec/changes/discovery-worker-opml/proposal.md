## Why
Direct subscription collection is available; users need safe feed discovery, deliberate preview confirmation, scheduled updates and portable subscriptions.
## What Changes
- Discover at most eight deduplicated RSS/Atom candidates from a site; preview up to five entries without creating data.
- Confirm an expiring, session-bound preview by revalidating and refetching.
- Add persisted collection settings and a separate worker with bounded refresh and overlapping-job exclusion.
- Add OPML preview, explicit import confirmation and escaped export.
## Capabilities
### New Capabilities
- `discovery-worker-opml`: Preview-based subscription onboarding, periodic collection and OPML portability.
### Modified Capabilities
None; existing direct registration remains compatible.
## Impact
Rust fetch/module/API/worker and database v3, React collection/settings UI, tests and operating instructions. No production deployment or complete legacy-data migration.
