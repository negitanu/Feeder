## Why
Reader locations should be shareable and browser navigation should return to the reading context.

## What Changes
Encode reader view, search, source, detailed filters and article in a validated canonical URL; restore bounded in-memory list snapshots on history traversal. No API or schema changes.

## Impact
Reader frontend, synthetic browser smoke test and documentation. Existing cursor and visit contracts remain authoritative.
