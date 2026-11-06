## Why

FeedAstrolabe supports RSS-specific polling intervals of 60–86400 seconds, with null inheriting the global setting. Feeder currently rejects a non-null `poll_interval` and schedules all feeds at the same interval. Add the existing feature to the UI, public API and independent worker.

## Scope

Correspondence: legacy `feedastrolabe/subscriptions.py`, `feedastrolabe/collection.py` and `docs/api.md`. Only RSS feed scheduling and associated settings export are included. Legacy database conversion is explicitly out of scope. Existing Feeder databases and state must remain usable and recoverable.
