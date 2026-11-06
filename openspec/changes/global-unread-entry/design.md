## Design

- Explicit activation of 未読 clears source, search input/applied search and advanced filters, returning to default new order and title/body scope. Selecting a folder or applying search afterward retains unread filtering; in that scoped state the global unread control is not marked active.
- A separate internal unread count request uses the existing validated article list API with `unread=true&limit=1`; no state changes or visits result. Request generations reject stale count results. Fetch after successful article visit/read changes and scoped bulk updates, library changes, initial load and window focus; periodic bounded refresh accounts for independent worker/other sessions. Failure retains the last known count and does not block reading.
- Use semantic button/current state and a text badge that remains legible on desktop, mobile and existing themes. Do not add a new database schema, browser storage or public API.
- Synthetic browser tests cover a selected feed/folder and search, global unread reset, independent counts after scoped visits/bulk actions, and viewport/theme behavior. No legacy data migration.
