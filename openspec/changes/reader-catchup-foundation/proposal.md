## Why
Reader catch-up needs additive storage for collection progress, article versions and explicit reading preferences without losing Feeder's existing library.

## What Changes
- Upgrade known Feeder databases to schema v10 with a consistent pre-upgrade backup.
- Keep the v9 schema constructor immutable and independently verify the v10 object set before any writes.
- Integrate new Reader APIs behind the existing authentication, authorization, Origin and CSRF boundary.
- Preserve existing article, reading, subscription, order and settings data.

## Impact
Rust schema/route integration, migration fixtures and synthetic backup/restore acceptance. Product requirements are owned by reader-catchup and collection-reliability. Existing Web behavior remains the baseline; historical FeedAstrolabe data conversion is excluded.
