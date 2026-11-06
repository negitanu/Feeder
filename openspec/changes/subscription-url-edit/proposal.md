## Why

FeedAstrolabe lets readers repair an RSS subscription URL without losing articles or reading state. Feeder currently rejects `url` on the public v1 PATCH and does not offer it in the subscription edit dialog. The missing operation blocks recovery when a feed moves.

## Scope and correspondence

Implement RSS URL edits in the browser and `PATCH /api/v1/feeds/{id}`, matching the applicable behavior of FeedAstrolabe `feedastrolabe/subscriptions.py::update_subscription` and `docs/api.md` subscription management. Keep identity, entries, flags, ordering, folder and paused status unless explicitly changed. No page observer, SNS, individual interval, data migration, or new network request is added.

Use the existing Feeder SQLite refresh lease to reject concurrent subscription edits/deletes while a refresh is active. This additionally closes a race between collection and management across the independently running Web and worker processes. Legacy uses a refresh lock instead of a DB lease.
