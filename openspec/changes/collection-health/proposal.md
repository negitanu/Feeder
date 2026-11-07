## Why

Operators need a read-only view of feed collection outcomes and schedule state so they can find failed or overdue feeds without causing collection work.

## Scope

Add a compact backend attempt-history table, a bounded read-only diagnostics endpoint, and a reusable read-only React panel. The existing `feeds.last_checked` / `feeds.last_error`, `collection_settings`, and `feed_poll_intervals` remain authoritative for existing collection and scheduling behavior. No changes to the collection worker, authentication migration, app navigation, or retry endpoint are part of this change.

## Existing Web correspondence

- `openspec/specs/subscription-collection/spec.md` defines direct subscriptions, durable refresh, pause behavior and safe collection.
- `backend/src/collection.rs` currently stores `last_checked` and `last_error`; `collection_settings` holds global enabled/interval and `feed_poll_intervals` holds an optional per-feed interval. Its lease-owned refresh paths are where the owning collection implementation must call `record_attempt` only after an accepted completion.
- The backend database migration is orchestrated in `backend/src/main.rs` through the version-9 extensions migration; the diagnostics route is merged from `collection_health::routes()`.
- `frontend/src/CollectionHealthPanel.tsx` is mounted in the Settings health section of `App.tsx`; the app supplies the authorized feed-refresh callback and the configured display timezone.

## Guarantees

- Diagnostics GET performs no writes and is capped at 1000 feeds with explicit truncation metadata.
- A completed success records its UTC SQLite timestamp and resets the consecutive failure count. A completed failure increments the count and leaves the last successful timestamp unchanged.
- Historical rows without a recorded success may infer `last_success` from a successful legacy `last_checked` only when `last_error IS NULL`; legacy errors do not imply an invented failure count.
- Paused feeds and globally disabled collection have no `next_due`.
- UI timestamps are absolute and use the requested UTC or Asia/Tokyo display timezone; filtering and retry are read-only except for the caller-provided retry callback.

## Non-goals

No legacy FeedAstrolabe migration, collection retry implementation, scheduler changes, destructive repair, route registration, App integration, deployment, or commits.
