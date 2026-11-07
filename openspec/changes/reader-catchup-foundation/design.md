## Design
The parent integrator owns main.rs and extensions.rs. Existing v9 module migrations retain their original SQL. A separate v10 migration invokes the new modules within the same upgrade transaction. Before opening a populated database for writing, reconstruct the expected schema in memory and compare every non-SQLite object. Back up a known prior version through SQLite's backup API before modifying it, refusing to overwrite an existing backup.

New modules expose migrate(tx) and routes(); existing collection code calls article version ingestion within the accepted refresh transaction. UI-only recommendations cannot change existing read/star state. New API routes use the same browser authentication and CSRF wrapper as existing feature endpoints. Tests use synthetic databases and independent restored copies.

The existing Reader gains an explicit mode switch. `mode=catchup` is part of its URL history; no mode parameter retains the article-list default. Selecting a source, library view or submitting a search explicitly returns to the article list. Catch-up owns its bounded query controls and does not claim to apply hidden list search filters. Existing detail open/close handling is reused, with related/history panels inside the detail. The selection and history caches remain bounded.

## Validation
Check v9-to-v10 upgrades, old-version regression fixtures, unknown v10 no-write rejection, preservation of all old typed cell values, restart, foreign keys, full-volume/SQLite restoration, authenticated API access and malformed requests. No tests use production subscriptions, secrets or backups.
