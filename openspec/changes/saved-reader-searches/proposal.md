## Why
Readers repeat the same keyword and source filters daily.
## What Changes
Save named reader search snapshots in the current browser and restore them without copying pagination or article selection. Limit to 50 saved searches, validate stored data, support removal and storage failure feedback.
## Capabilities
### New Capabilities
- `saved-reader-searches`: Browser-local saved reader filters.
### Modified Capabilities
## Impact
React reader only. No DB migration. Saved searches use the current display timezone; browser-only scope is explicit. Old research watchlists remain out of scope.
