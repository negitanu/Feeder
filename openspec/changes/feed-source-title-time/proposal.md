## Why
Reader rows currently prioritize a relative publication date and a URL hostname, which makes it harder to scan the publication source and exact article title. The row should consistently present the feed, title, and an absolute timestamp.

## What Changes
- Add a reusable React article-row content component for source → title → publication time.
- Show a feed title when supplied, falling back to the article URL hostname.
- Format a full timestamp in the selected UTC or Asia/Tokyo timezone and show an explicit unknown value for invalid dates.
- Keep excerpts available in standard and tile layouts while allowing the compact line layout to prioritize the title.

## Capabilities
### New Capabilities
- `feed-source-title-time`: Reader rows consistently expose source, title, and absolute publication time.

## Impact
Frontend component and scoped CSS only. The article list API contract gains optional `feed_title`; no database migration is required for this presentation change.
