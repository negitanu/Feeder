# Reader API

The service binds `127.0.0.1:8080` by default. Configure it with `FEEDER_BIND` and set `FEEDER_DB` to a Feeder-owned SQLite file (default `data/feeder.sqlite3`). Set `FEEDER_DEMO=1` to opt in to generated sample articles. It serves the built `frontend/dist` directory at `/`. All requests require a valid local Host; mutations reject unrelated Origins.

- `GET /healthz`: health response.
- `GET /api/articles?limit=20&offset=0&unread=true&starred=true&q=term&feed_id=1&folder_id=1`: returns `{items, total}`. Filters are optional and combine; limit is bounded. Articles have `id`, `title`, `content`, `url`, `published`, `is_read`, and `is_starred`.
- `GET /api/articles/{id}` returns an article without changing its state.
- `POST /api/articles/{id}/state`: `{read?, starred?}` returns the updated article.
- `GET /api/library`: `{folders, feeds}`. Folders have `id`, `name`; feeds have `id`, `url`, `title`, `folder_id` (nullable), `paused`, `last_checked`, and `last_error` (both nullable).
- `POST /api/folders`: `{name}`; `PATCH /api/folders/{id}`: `{name}`. Both return the folder. `DELETE /api/folders/{id}` returns 204 and moves its feeds to unfiled.
- `POST /api/feeds`: `{url, folder_id?}` validates and fetches a direct RSS/Atom URL before atomically creating the feed and articles. Duplicate URLs return 409.
- `PATCH /api/feeds/{id}`: `{title?, folder_id?, paused?}` updates a feed; pass null for `folder_id` to move it to unfiled. Returns the feed.
- `POST /api/feeds/{id}/refresh` conditionally fetches a feed and returns it; paused feeds return 409. A failed refresh retains articles and records an error. `DELETE /api/feeds/{id}` deletes the feed and its articles and returns 204.

Feed collection rejects non-public destinations, credentials and unsafe redirects, validates DNS on every hop and pins validated addresses, bypasses proxy settings, and enforces a total 10-second timeout, five redirects, 2 MiB response and 500 entries. Article bodies are converted to plain text. Discovery and previews use the same fetch protections.

Recognized version 1 databases are copied using SQLite's backup API to a sibling with extension `.v1-backup.sqlite3` before migration; an existing backup is not overwritten. Unknown schemas are refused. `reqwest` 0.12 and `html2text` 0.16 use the compatible APIs chosen for this implementation; the lockfile records exact resolved versions.


- `POST /api/discover`: `{url}` returns `{candidates:[{url,title}]}` with up to eight unique RSS/Atom alternate links resolved against the final site URL. A direct feed is itself a candidate.
- `POST /api/preview`: `{url}` returns `{token,url,title,entries:[{title,url}],expires_in}` with up to five entries. It does not write subscriptions. The response sets an HttpOnly SameSite cookie when needed; keep that cookie for confirmation.
- `POST /api/preview/confirm`: `{token,folder_id?}` consumes the single-use, session-bound plan and refetches before creating the subscription. Plans expire after 900 seconds, are held in bounded server memory and disappear on restart. Invalid, expired, reused or cross-session plans fail.
- `GET /api/collection/settings` and `PATCH /api/collection/settings` read or update `{enabled,interval_seconds}`. Collection defaults to disabled, with a 900-second interval; accepted intervals are 60–86400 seconds.

Run `feeder worker` separately with the same `FEEDER_DB` to schedule due, unpaused feeds. `feeder worker --once` processes one pass and exits. Disabling collection prevents new scheduled refreshes; an in-flight fetch can finish. Per-feed database leases prevent simultaneous manual and worker refreshes and expire after 60 seconds. Pausing a feed while a fetch is in flight prevents applying its result.

Recognized version 2 databases are backed up to `.v2-backup.sqlite3` before migration to version 3. Existing backups are not overwritten; unknown schemas are refused.

- `POST /api/opml/preview`: `{xml}` returns `{items,valid,new,existing,duplicates,invalid}`. Items contain `url`, `title`, `folder`, and `status`; invalid entries are counted and shown with status `invalid`.
- `POST /api/opml/import`: `{xml}` reparses and validates the document and returns `{added,existing,duplicates,invalid}`. Newly imported feeds are paused and no outbound fetch occurs. Nested folders are flattened into slash-separated names.
- `GET /api/opml/export` returns an escaped OPML attachment. The `feeder:paused` outline attribute preserves pause status in the export; imports deliberately pause all newly added feeds.

OPML input is limited to 1 MiB, 1,000 outlines, and 16 levels of XML nesting. DTDs and entity declarations are rejected. Neither OPML operation imports article content or credentials.

## Reader productivity

Feeder schema v4 adds `article_visits`. Known v3 databases are backed up to `.v3-backup.sqlite3` before migration; existing backups are never overwritten. Article JSON includes nullable `last_visited`.

- `POST /api/articles/{id}/visit`: marks read and records the latest detail visit atomically, preserving stars. Returns Article. GET/state/bulk do not create visits.
- List filters additionally accept `history=true`, `order=new|old|recent`, `scope=both|title|body`, `any` and `exclude` CSV phrases, `published_from` / `published_to` UTC calendar dates, and `feed_ids` as up to 50 comma-separated IDs. All filters combine. Recent order is the default for history, and date order uses actual UTC instants with ID tie-breaking.
- `POST /api/articles/read-all`: `{filters:{...}}` returns `{updated:number}` for all matching articles, independent of list pagination.
- `POST /api/articles/bulk-state`: `{ids:[...],read?,starred?,filters:{...}}` validates up to 100 distinct existing in-scope IDs and updates atomically. It returns `{updated:number}`. Mutation filters reject pagination keys, unknown fields and invalid conditions. Empty selection, duplicate/unknown/out-of-scope IDs or absent actions are rejected without partial changes.

Browser `PATCH /api/feeds/{id}` and write-scoped `PATCH /api/v1/feeds/{id}` accept a non-null `url` for RSS subscriptions. It is normalized and checked without network access; private IP literals, local names, credentials, invalid schemes and duplicates are refused. A changed URL clears old HTTP validators and collection status but retains the feed, articles and reader state; equivalent normalized URLs leave the metadata untouched. Both endpoints and their DELETE counterparts return 409 while an unexpired refresh lease exists. Expired leases do not block management; worker results are applied only if the worker still owns an unexpired lease in the same write transaction. DNS and redirect safety are checked again during the next fetch.

Both scoped mutation endpoints accept an optional `expected_timezone` (`UTC` or `Asia/Tokyo`). It is checked against the shared setting inside the mutation transaction; a mismatch returns HTTP 409 without changing articles. An invalid value returns 400. Omitting it preserves the existing behavior of using the current setting. The browser submits its displayed timezone and refreshes settings after a conflict; it does not replay the mutation.

`q` is one literal phrase up to 256 bytes. Any/exclude fields allow up to 20 CSV phrases, each 256 bytes and at most 4096 bytes of encoded text per field. List query input is bounded to 8192 bytes and 32 distinct parameters; duplicate/unknown fields and invalid values are rejected. Each internal batch is 1–100 articles. Malformed or reversed dates, unavailable sources and unsupported scope/order are rejected before mutation. Historical newline search URLs remain future compatibility work.

Internal cursor requests use `pagination=cursor`, optionally with `cursor` on later requests, and cannot include `offset`. Cursor-mode responses add `next_cursor` and `has_more`; existing offset requests and public v1 are unchanged. Malformed or unsupported tokens produce 400; invalid signatures, expired tokens or changed scope produce 409 and require a fresh list. Tokens expire 30 minutes after the first page and use a process-local signing key: restarting the server invalidates them. This assumes a single web process. Network/server errors allow retry of the same page. High-water IDs exclude newly inserted higher IDs, but SQLite can reuse the deleted highest ID; mutable ordering keys and filter eligibility are live rather than a historical snapshot.

## Library ordering (schema v5)

`GET /api/library` returns folders in stored order and feeds grouped by stored folder order, then unfiled, with stored order inside each group.

`POST /api/library/order` accepts `{expected, order}`. Both are complete snapshots `{folders: [id, ...], feeds: [{id, folder_id}, ...]}` in canonical group order; unfiled is `folder_id: null`. Folder and feed identities must each occur exactly once. `expected` must match current arrangement. Success204; stale409; invalid membership/duplicates/missing identities400; hostile Origin403. Unknown JSON fields are rejected. SQLite applies membership and positions in one immediate transaction. New sources and reassigned feeds append; deleting a folder appends its feeds to unfiled in their prior order. Renaming sources preserves positions.

Known v4 databases are backed up before the v5 migration. Migration preserves visible name/title/id ordering and adds order tables/triggers. Unknown schemas and an existing migration-backup path are refused. Article IDs, read/star flags, visits and collection settings are unchanged.

## Reader settings and safe export (schema v6)

`GET /api/reader/settings` returns `{display_timezone, default_view}`. `PATCH` accepts either or both explicit non-null fields and returns the complete object. Timezone is `UTC` or `Asia/Tokyo`; default view is `all`, `unread` or `starred`. Empty, unknown and invalid fields are rejected before writes. Same-origin validation applies. The shared timezone defines date-filter boundaries in list/read-all/bulk-state: inclusive local start, exclusive next-day end. Publication ordering continues to use actual instants.

`POST /api/configuration/export` accepts `{presentation:{appearance,width,layout,collapsed_folders}}`. Appearance is `system|light|dark`, width is an integer200..480, layout is `standard|line|tiles`, collapsed folders are at most2000 distinct positive IDs. It returns a UTF-8 JSON attachment named `feeder-settings.json`, with `Cache-Control: no-store`. Unknown properties are rejected.

The versioned format includes `format: feeder-settings`, `format_version: 1`, `app_version`, `schema_version`, `exported_at`, `settings` (collection/reader/folders/subscriptions/presentation), `omissions` and `unsupported_sections`. Folder and subscription arrays use stored ordering. Omitted values become null with value-free path/reason records. Serialization uses explicit field allowlists and a consistent read transaction, and performs no collection, settings mutation, secret-store access or credential creation. URLs with credentials/query/fragment or credential-like names/paths are omitted conservatively. Runtime/source errors, conditional fetch headers, articles, visits, logs and queues are excluded. This is export-only and is not a data backup.

Recognized v5 databases are backed up to `.v5-backup.sqlite3` before v6 migration. Existing backup paths and unknown schemas are refused.

## Browser sessions and public API (schema v7)

Production internal API writes require the `feeder_ui` cookie and `X-Feeder-CSRF`. `GET /api/session` returns `{csrf_token}` with no-store and creates/reuses an HttpOnly SameSite=Strict12-hour session. Restart invalidates sessions; clients must explicitly retry after a rejected action. Host/Origin/Fetch-Site checks precede internal requests; tunnel cookies are Secure.

Session-protected `GET /api/keys` returns `{keys:[safe metadata...]}`. `POST /api/keys` accepts `{name,scope,days}` (1..80characters, read/write,1..365days) and returns `{id,token}` only once with no-store. `DELETE /api/keys/{id}` revokes. `GET/PATCH /api/keys/settings` uses `{rate_limit}` (1..1000/minute, default120). Tokens are256-bit random values persisted as SHA256 hashes only.

`/api/v1` requires a separate Bearer key, supports the coreRSS/article endpoints and ID pagination documented in [API access](api-access-control.md), and never substitutes a browser session for authentication. Permission, expiration/revocation and rate counters are checked in an immediate transaction. Error codes are numeric. Public API operations do not create reader visits. Unmigrated source-specific settings fail explicitly.

Configuration export now reports the actual schema version and `settings.api.rate_limit`; no key metadata, hashes, counters or tokens are included. Recognized v6 DBs retain a `.v6-backup.sqlite3` before migration.
