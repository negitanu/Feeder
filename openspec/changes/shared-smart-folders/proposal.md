## Why
Saved search snapshots are browser-local and cannot be reused across devices.
## What Changes
Add shared smart-folder CRUD with revision checks and explicit copying of existing local searches. Preserve local records and current save UI. No legacy migration or research features.
## Impact
A v9-owned SQLite table and authenticated Web API; a standalone React list and local copy action. Existing reader filter validation and scope handling remain authoritative.
