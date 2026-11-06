## Context

`backend/src/reader.rs::list` applies LIMIT/OFFSET. `frontend/src/App.tsx::loadMore` uses rendered count as offset and deduplicates IDs. Deduplication cannot recover articles skipped by shifts. `loadMore` reports errors through a first-page retry, and no IntersectionObserver means no alternative continuation. Existing public v1 uses independent ID cursors and must keep its contract.

## Goals / Non-Goals

**Goals:** No offset-shift skips for insertion/deletion, scoped tokens, explicit continuation recovery and stale-timezone mutation rejection.

**Non-Goals:** A historical snapshot of mutable content/sort keys; no-script HTML rendering; floating details; legacy DB conversion; research/SNS features; public deployment.

## Decisions

- Opt into cursor mode with `pagination=cursor` for the first request; continuation uses the same parameter and a non-empty `cursor`. Return `next_cursor` (string or null) and `has_more` in that mode only. Reject `cursor` without the mode, any `offset` in cursor mode, malformed/mixed parameters or an unsupported version with 400. Return 409 for a structurally valid token with an invalid signature (including a process restart), an expired token or changed scope. Preserve the legacy response and offset-only calls; public `/api/v1` remains ID based. A 409 requires explicit fresh-list reload; transient/network errors can retry the same page.
- Reuse normalized validated filter parsing and SQL scope. Bind token to a digest of all filters, order, selected shared timezone and relevant source membership. Include format version, expiry, initial max article ID and last ordering tuple. Sign bounded payloads with a maintained HMAC implementation and compare authentication tags safely; no handwritten crypto. Enforce a small token-size bound and a30-minute expiry, reject unknown versions and malformed values before querying data.
- Initially use a process-local random signing key, separated from API keys, sessions and export. Restart invalidates cursors; UI preserves already loaded cards and offers an explicit fresh-list action. Document single-Web-process semantics. Persistent multi-replica signing is separate work.
- Use SQL ordering tuples equivalent to existing normalized Julian publication date/ID and visit time/ID, including NULL-last grouping. Add `id <= initial_max_id` so newly collected articles with higher IDs wait for explicit refresh. Read high-water, timezone/membership and first batch within one consistent transaction. Later batches reject changed timezone/membership; never silently use a broader scope. The existing SQLite schema can reuse a deleted maximum ID, so a new article reusing such an ID is not necessarily excluded by the high-water mark; fixing that requires a separately specified schema migration.
- Keep source order changes that do not change article scope valid. Source membership changes affecting a selected folder require refresh. Do not log token payloads/search phrases or treat the signature as public API authorization.
- Mutable publication dates, content eligibility, visit timestamps and read/star flags are not historical snapshots. Reflect current state and document that changed sort/eligibility may require refresh; do not claim snapshot isolation. Tests for insertion/deletion use unchanged sort keys and matching surviving articles.
- Add an optional expected display timezone to shared scoped browser mutations, validate it against current DB state inside the mutation transaction and return a conflict before writes on mismatch. Existing API clients omitting it retain their documented current-timezone interpretation. UI refreshes settings and asks for an explicit retry, never replays a bulk write.
- Separate initial and continuation error states. An explicit keyboard-accessible load/retry button handles missing IntersectionObserver and network failures. Do not automatically retry a failed page. Keep loaded cards/selection/filter/detail position. Expired/scope-conflicting cursors offer a clearly named fresh-list reload, distinct from retrying the same page.

## Risks / Trade-offs

- Float Julian ordering and equal timestamps → bind SQL-generated ordering values and test equal instants with different offsets, NULL dates and IDs in both directions.
- Process restart invalidates cursors → bounded failure with explicit fresh reload; no reliance on browser sessions for signature validity.
- State/filter changes during reading → retained list reconciliation plus documented live-view semantics, not an invented full snapshot guarantee.
- Added normalization paths → one canonical scope/fingerprint implementation shared by verification, pagination and mutation expectations.

## Migration Plan

No schema migration is assumed for process-local cursor signing. If implementation requires persistent schema, update this design and add recognized migration/backup tests first. Preserve all existing v7 state and offset/public API contracts. Roll back only the application change if no schema changed.
