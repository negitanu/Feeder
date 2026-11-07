## Why
The requested reader features need shared durable storage and a consistent article representation.
## What Changes
Introduce a recognized v9 additive schema with automatic v8 backup, shared extension routes, publisher metadata, read-later and mute filtering, and source-aware absolute-time rows. Integrate independently specified smart folders, duplicates, SNS, read later, mute rules, health, history and digest.
## Capabilities
### New Capabilities
- `reader-expansion-foundation`: Shared reader feature integration and data preservation.
### Modified Capabilities
## Impact
Rust SQLite recognition/migration, query filtering, React integration and synthetic acceptance. No old FeedAstrolabe import. Existing articles and reading states are preserved; external services are exercised through bounded adapters and synthetic fixtures before live acceptance.
