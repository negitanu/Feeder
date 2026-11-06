## Why
The reader needs real subscriptions and bounded safe RSS/Atom collection.
## What Changes
- Add folders, feed registration, pause/resume, deletion and manual refresh with preserved article state.
- Add sidebar filtering and contextual controls, and a collection form.
- Retain sample articles and migrate only recognized Feeder schema v1 to v2.
## Capabilities
### New Capabilities
- `subscription-collection`: Initial direct-feed registration, collection and organization.
### Modified Capabilities
None. Website discovery, signed preview confirmation, periodic worker and advanced drag ordering remain later migration tasks.
## Impact
Backend schema/API and React reader. No legacy FeedAstrolabe DB migration or deployment.
