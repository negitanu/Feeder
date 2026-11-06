## Design

- Accept `url` as an optional non-null string on browser `/api/feeds/{id}` and public `/api/v1/feeds/{id}` PATCH. The browser dialog submits the URL alongside the current title and folder. Existing clients may omit it.
- Normalize and validate with the existing OPML URL policy: public HTTP(S), no user information, no localhost/local domains or non-public IP literals; strip fragments. No DNS lookup or request is performed at edit time. The existing collector must validate and pin public DNS on every later request and redirect.
- Within one SQLite `BEGIN IMMEDIATE` transaction, verify the feed exists, reject an unexpired refresh lease with 409, check the normalized URL against other subscriptions including fragment-bearing old URLs, and validate remaining inputs. A conflict/invalid value rolls back all changes, including creation of any requested folder. Expired leases do not block edits. Apply the same immediate lease check to browser and public deletion.
- Claim refresh leases in an immediate transaction and check ownership/expiry again inside the immediate transaction that stores fetched articles. An old response must not be applied after a lease expires and another process changes the URL.
- When the normalized URL actually changes, clear `etag`, `modified`, `last_checked` and `last_error` so a future refresh cannot use old validators or status; unchanged normalized URLs preserve them. No new schema or writer migration is required. Preserve existing feed ID, article IDs/content/read/star/history and ordering.
- The browser reports an explicit conflict/invalid-URL message and keeps the edit dialog open. Prevent stale response overwrites from concurrent use by relying on SQLite transaction serialization; last completed edits from separate clients remain last-writer-wins (unchanged existing behavior).

## Acceptance

Synthetic tests exercise both production-protected API paths, unchanged and changed URLs, invalid/private/credential URLs, duplicates, expired/active leases, rollback of title/folder/flags, unaffected articles and history, and safe no-network updates. Browser tests cover an edit from the contextual menu, preservation of displayed content and responsive layout. Verify strict OpenSpec and Linux regression. No live feed or existing database is modified.
