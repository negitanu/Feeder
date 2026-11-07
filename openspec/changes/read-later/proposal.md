## Why
Readers need a persistent, separate queue for articles to revisit without changing read or starred state.

## Existing Web Mapping
The Web SQLite `articles` table owns article identity, read state and stars; the existing authenticated `/api` router and `apiFetch` provide access and CSRF protection. Reader navigation owns actual visits. This feature adds independent queue and settings tables; it does not import legacy data.

## Acceptance
- Saving and removing are idempotent, independent of read and starred state, and reject nonexistent article IDs.
- A bounded newest-saved-first list survives restart; deleting an article cascades to its queue entry.
- Automatic removal defaults off and is applied only when Reader explicitly invokes the visit hook, never when merely marking read.
- A keyboard-accessible button, queue panel and setting expose loading and failure states.
- Synthetic fixtures verify independence, unknown IDs, cascade and visit settings.
