# Design

Add v10-only runtime state with cascading feed ownership, independent of the existing v9 migration. Each accepted lease atomically commits articles, checkpoints, diagnostics and retry state. Retry-After on 429/503 and bounded exponential failure backoff constrain manual and scheduled refresh equally. Serialize network requests per host while retaining redirect validation and pinned DNS. Social pagination has a bounded page budget; persist continuation and a completed high-water mark, never advertising initial historical backfill as current. URL changes invalidate associated state. Health reports waiting, pending and possible gaps. Existing conditional RSS and public-only social filtering remain in place.

Mastodon account statuses: https://docs.joinmastodon.org/methods/accounts/#statuses; Bluesky author feed: https://docs.bsky.app/docs/api/app-bsky-feed-get-author-feed and https://github.com/bluesky-social/atproto/blob/main/lexicons/app/bsky/feed/getAuthorFeed.json. Mastodon supports max_id/since_id; Bluesky uses an opaque cursor, with no guaranteed snapshot across requests, so gap uncertainty must remain visible.

## Final integration review

Use the runtime load/save/backoff helpers in the production collection path, with one shared checkpoint representation. Store an optional JSON cursor guard in the v10 runtime row: a Brent cycle detector keeps an anchor, power and distance, bounding storage without retaining every visited cursor. It survives refresh/restart boundaries and detects longer cycles. Source changes clear the row, including this guard. A cycle guard is reset only on completed continuation; errors retain the prior committed checkpoint.

Mastodon pagination uses the last numeric id even for a short non-empty page; an empty page or the previous high-water boundary terminates collection. A missing boundary is visible as a possible gap and marks the response as recovery before article persistence. Recovery remains on the returned checkpoint through storage of the final page, then clears only in the persisted completed state. Mastodon `edited_at` maps to Atom updated while `created_at` maps to published; absent/null edited_at falls back to created_at. Reference: https://docs.joinmastodon.org/entities/Status/#edited_at.

Acceptance includes a real `apply_refresh` regression using synthetic SQLite and parsed RSS across initial, due normal, failure and recovery collection, asserting stored ingestion_kind and read/star preservation. Full strict OpenSpec validation and owned-code clippy checks are required.
