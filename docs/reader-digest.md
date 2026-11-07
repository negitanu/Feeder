# Reader digest

Feeder provides a deterministic, read-only digest of articles already stored in its Web database. It does not use AI, investigate article content, or send notifications.

## API

`GET /api/digest?hours=24|168[&feed_id=N|&folder_id=N]`

`hours` defaults to 24. `feed_id` and `folder_id` are optional and mutually exclusive; IDs must identify existing records. Unknown, duplicate, non-positive, or unsupported query parameters return HTTP 400.

Response shape:

```json
{"hours":24,"total":12,"shown_count":10,"source_count":1,"truncated":true,"sources":[{"feed_id":1,"source_title":"Example","count":12,"shown_count":10,"items":[{"id":10,"title":"New article","published":"2026-11-07T00:00:00Z","url":"https://example.org/article"}]}]}
```

`count` and `total` are actual qualifying article row counts, independent of the display cap. Dates are evaluated with SQLite `datetime()` against the requested rolling UTC duration; null, unparseable, and future dates are excluded. Enabled mute rules are applied to counts and items. The newest ten per source are candidates for the globally newest 100, with article ID descending as the tie-break. Sources are ordered by their newest displayed item. `source_count` counts all qualifying sources, `shown_count` counts displayed items, and `truncated` indicates any qualifying items were omitted. The panel supports refresh and renders times in its configured timezone. Source labels come from `feeds.title`. The endpoint returns no article body and does not alter read or starred state.

## Existing Web mapping

The implementation uses `articles.published` and `articles.feed_id` with `feeds.title` and `feeds.folder_id`. These are existing Feeder Web schema fields; no new table or migration is needed. The React panel uses the existing collection `Feed` and `Folder` shapes and opens an article through the Reader callback.
