## Design

- [x] Define data/API/UI contract and map it to current Web implementation.

## Implementation

- [x] Add collection attempt history migration, recording helper, and read-only health handler.
- [x] Add compact read-only diagnostics panel and responsive styling.
- [x] Document historical fallback and integration contract for main/auth and collection owners.

## Verification

- [x] Exercise schema, upsert, fallback, schedule, stale and bounded-result behavior with synthetic fixtures.
- [x] Check owned-file diff and build/typecheck where shared state permits.
