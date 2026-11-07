## API
`GET /api/digest?hours=24|168[&feed_id=N|&folder_id=N]` returns `{hours,total,shown_count,source_count,truncated,sources:[{feed_id,source_title,count,shown_count,items:[{id,title,published,url}]}]}`. Query parsing rejects duplicate, unknown, oversized, non-positive, or mutually exclusive source parameters. SQL values are bound parameters. `datetime(published)` is used for SQLite-recognized date strings; NULL/unparseable dates are excluded.

Enabled mute rules use the shared article predicate for counts and items. Counts cover all visible valid non-future articles in the interval. Rank each source by publication time descending and ID descending, retain ten per source, then select the newest 100 across sources with the same tie-break. Sources follow the order of their newest selected item. The response reports total qualifying sources, displayed counts, and whether any qualifying item was omitted.

## UI
`DigestPanel` accepts `{feeds, folders, onOpen, timezone?}` using the existing `Feed` and `Folder` shapes. It loads the API on filter changes, renders real counts and a bounded list of links, supports refresh and loading/empty/error states; formats timestamps in the configured timezone, and opens via `onOpen(id)`. Existing article state is not written.

## Migration and Security
No new table or schema migration is needed. Register the module and mount its GET router in the application router. Keep the endpoint read-only, use bounded query parsing and parameterized SQL, and return generic bounded HTTP errors. No article contents are exposed.

## Verification
Use synthetic in-memory SQLite fixtures with valid, old, future, and malformed publication times, multiple feeds/folders, and more than ten entries per feed. Verify counts, caps, filtering, stable title, and invalid query rejection.
