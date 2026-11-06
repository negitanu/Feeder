## Why
Users cannot comfortably replace the collection interval directly, and imported feeds start paused.
## What Changes
Keep interval input as an editable string until validated save. Use active subscriptions for new OPML entries and enable the collection setting when it is first created. Preserve existing pause/disabled choices and duplicate feeds.
## Capabilities
### New Capabilities
- `collection-defaults-input`: Editable interval and collection-ready subscription defaults.
### Modified Capabilities
## Impact
React collection form, OPML insertion and initial collection setting. No schema change. This supersedes previous paused-by-default OPML behavior. Import remains explicitly confirmed and does not fetch URLs inside the import request; worker collection follows its normal schedule and public-address checks.
