## Data and migration

Create `collection_health(feed_id INTEGER PRIMARY KEY REFERENCES feeds(id) ON DELETE CASCADE, last_success TEXT NULL, consecutive_failures INTEGER NOT NULL DEFAULT 0)` through `pub migrate(tx: &rusqlite::Transaction)`. Migration seeds known legacy successes (`last_checked` present and `last_error` null) before new attempts are written. `record_attempt(&Connection, feed_id, success)` uses one SQLite upsert; success sets `datetime('now')` and resets failures, failure increments failures while preserving `last_success`.

The collection owner must call `record_attempt` only in a completion branch that still owns the accepted refresh lease. This module does not alter `collection.rs` or lease semantics.

## Read contract

Expose `routes() -> Router<Db>` for read-only `GET /api/collection/health`; main merges these routes under normal authorization. JSON is `{global_enabled, global_interval_seconds, feeds, truncated}`. Each feed is `{id,name,url,paused,last_checked,last_success,error,next_due,consecutive_failures,stale,individual_override,poll_interval_seconds}`. `individual_override` indicates an explicit `feed_poll_intervals` row; `poll_interval_seconds` is null when inherited. `last_success` prefers the new history row; known historical success is seeded during migration, with a read fallback only when no history row exists and `last_error` is null. Legacy failure counts default to zero because their count is unknowable.

Effective interval is explicit feed interval or global interval. `next_due` is null if paused or global collection is disabled; otherwise it is now when never checked (the scheduler considers it immediately due), or last checked plus effective interval. `stale` is false while paused or globally disabled; otherwise it means no known successful attempt or the last success is at least one effective interval old. Select at most 1001 rows ordered by feed id, return the first 1000, and set `truncated=true` when the extra row exists.

The handler locks the shared connection only to execute SELECT statements; no transaction mutation, lease acquisition, or refresh is performed. Route registration and authentication middleware remain owned by main/auth.

## UI

`CollectionHealthPanel` accepts `{onRetry:(id:number)=>Promise<void>; refreshKey?:number; timezone?:'UTC'|'Asia/Tokyo'}`. It is mounted in App's Settings health section. It fetches diagnostics on mount and refreshKey change; it exposes All, Errors and Stale filters, reports truncation, renders absolute localized timestamps (never relative time), and invokes only the supplied retry callback. It has loading/error/empty states, semantic table markup, keyboard-accessible filters/retry, and responsive horizontal overflow.

## Verification

Use in-memory synthetic SQLite fixtures for migration shape, legacy-success seeding, foreign-key cascade, repeated success/failure transitions, legacy fallback rules, scheduling null cases, stale rules and 1000-row truncation. A full backend test run and frontend production build are recorded separately from browser interaction/accessibility verification; no browser run is claimed by this change.
