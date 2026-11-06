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

`q` is one literal phrase up to 256 bytes. Any/exclude fields allow up to 20 CSV phrases, each 256 bytes and at most 4096 bytes of encoded text per field. List query input is bounded to 8192 bytes and 32 distinct parameters; duplicate/unknown fields and invalid values are rejected. Each internal batch is 1–100 articles. Malformed or reversed dates, unavailable sources and unsupported scope/order are rejected before mutation. Historical newline search URLs, signed continuations and timezone display settings remain future compatibility work.

## Library ordering (schema v5)

`GET /api/library` returns folders in stored order and feeds grouped by stored folder order, then unfiled, with stored order inside each group.

`POST /api/library/order` accepts `{expected, order}`. Both are complete snapshots `{folders: [id, ...], feeds: [{id, folder_id}, ...]}` in canonical group order; unfiled is `folder_id: null`. Folder and feed identities must each occur exactly once. `expected` must match current arrangement. Success204; stale409; invalid membership/duplicates/missing identities400; hostile Origin403. Unknown JSON fields are rejected. SQLite applies membership and positions in one immediate transaction. New sources and reassigned feeds append; deleting a folder appends its feeds to unfiled in their prior order. Renaming sources preserves positions.

Known v4 databases are backed up before the v5 migration. Migration preserves visible name/title/id ordering and adds order tables/triggers. Unknown schemas and an existing migration-backup path are refused. Article IDs, read/star flags, visits and collection settings are unchanged.
