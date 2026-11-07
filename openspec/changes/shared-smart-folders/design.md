## Existing behavior
SavedSearches stores up to 50 validated snapshots in localStorage; App applies snapshots and checks missing sources. Reader filters remain the source of truth.
## Design
Migration creates smart_folders(id, name, snapshot, revision). The shared API accepts the existing snapshot shape, bounds names, strings and record count, and rejects duplicate names. PATCH and DELETE require the last revision; compare and mutate atomically. No request creates schema. A missing source never becomes an unscoped search: the UI delegates application to App's existing onApply callback. Local records are not altered by copying; duplicate shared names are rejected. All mutations use apiFetch and existing authentication/CSRF middleware.
