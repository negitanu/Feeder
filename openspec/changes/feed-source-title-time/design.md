## Design
`ArticleRowContent` accepts `{ article: { title, content, url, published, feed_title? }, timezone: 'UTC' | 'Asia/Tokyo' }`. It renders a source label, title, absolute publication time, and an excerpt for non-line layouts. Source is the non-empty `feed_title`, otherwise a sanitized URL hostname, otherwise a readable generic label. Invalid or absent publication dates render `公開日不明`.

The line layout uses a scoped grid with an approximately 160px source column, a flexible title column, and an approximately 180px time column. The title remains the dominant column and truncates without forcing horizontal overflow. Standard and tile layouts keep the same source/title/time order and include the excerpt. On narrow screens, metadata and title may wrap, with readable text no smaller than 11px.

The component preserves `.card-copy strong`, `.card-meta`, `.source`, and `.excerpt` hooks where practical. All selectors are rooted under the component class to avoid changing existing screens.

## Existing Web correspondence
- Current `frontend/src/App.tsx` article rows render date then hostname in `.card-meta`, followed by `<strong>` title and `.excerpt`.
- `App.tsx` already loads the reader display timezone; the integrating change passes it through as the component's `timezone` prop.
- The backend list response supplies `feed_title: Option<String>` from the subscribed `Feed.title` and `feed_id: Option<i64>` on internal Article responses; the component uses `feed_title` for display and does not need `feed_id`.
- The current UI uses `Intl.DateTimeFormat` with the configured timezone but only a medium date. This component adds hour/minute/second precision and never computes relative time.
