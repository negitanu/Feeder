# Feed, title, and publication time in Reader rows

## Contract
`ArticleRowContent` receives an article with `title`, `content`, `url`, `published`, and optional `feed_title`, plus a timezone of `UTC` or `Asia/Tokyo`. It displays source, title, and an absolute `YYYY-MM-DD HH:mm:ss` timestamp with `UTC` or `JST`. Invalid timestamps are shown as `公開日不明`; relative time is never used. Source prefers the feed title and falls back to the URL hostname.

The API integrator supplies optional `feed_title` (the subscribed `Feed.title`) and `feed_id` on internal Article responses; the component uses `feed_title` and does not need `feed_id`. The UI integrator should pass the reader's configured timezone. This component is intentionally standalone and does not modify `App.tsx`.

## Existing implementation mapping
`frontend/src/App.tsx` currently renders a `.card-copy` with `.card-meta` (date and hostname), a `<strong>` title, and `.excerpt`. It already has the configured reader timezone and the feed collection. Integration can replace that row's inner content with `ArticleRowContent`, retaining the surrounding article button and its event handlers.

## Acceptance checks
- Feed title wins; URL hostname is used when feed title is blank or absent.
- Timestamp includes seconds and follows the supplied timezone. SQLite `YYYY-MM-DD HH:mm:ss` values are interpreted as UTC regardless of the browser timezone; explicit offsets are preserved.
- Unknown timestamps are labeled explicitly.
- Line layout truncates source/title/time within the available row width; standard and tile layouts retain excerpts.
- Mobile metadata and title may wrap without reducing text below 11px.
