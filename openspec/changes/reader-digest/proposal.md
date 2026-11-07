## Why
Readers need a compact, in-app view of newly published articles across their subscriptions.

## What Changes
- Add a deterministic digest API that counts recent publications by source and returns bounded article links.
- Add a Reader panel to choose the 24-hour or 7-day window and an optional feed or folder scope.
- This is a local presentation of stored Feeder articles. It does not use AI or send notifications.

## Existing Web Mapping
- Stored article publication time and feed ownership: `articles.published`, `articles.feed_id` in the Web SQLite schema.
- Source names and folder membership: `feeds.title`, `feeds.folder_id`.
- Existing collection API and UI types: `backend/src/collection.rs` and `frontend/src/LibrarySidebar.tsx`.
- Article states remain owned by the existing Reader; digest responses are read-only.

## Acceptance
- `GET /api/digest` accepts only `hours=24|168` and at most one positive existing `feed_id` or `folder_id`.
- Counts reflect qualifying article rows; invalid, unknown, future, and muted publication rows do not count.
- Each source has a consistent source title, actual count, and at most ten newest articles. Total items are at most 100, selected globally newest first after the per-source cap; response identifies truncation and displayed/source counts.
- The panel lets the user choose the time window and source, supports refresh and configured timezone, opens an item through its callback, and does not mutate read/star state.
- Synthetic tests cover date filtering, source/folder scopes, bounded items, and invalid parameters.
