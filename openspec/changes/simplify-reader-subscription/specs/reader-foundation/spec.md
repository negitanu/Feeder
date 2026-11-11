## ADDED Requirements
### Requirement: Compact reader and direct subscription
The reader SHALL remove catch-up and its mode switch, and show its heading and list controls on one row when space permits, without explanatory subtitle or timezone label. Settings links SHALL navigate to their named sections. Entering a site or feed URL SHALL automatically discover and validate a feed before the user registers it with an optional name and destination folder. Stale discovery results SHALL never register a different URL. Existing reader and subscription data SHALL be preserved.
#### Scenario: Multiple feeds
- **WHEN** a site advertises multiple feeds
- **THEN** the reader can select a candidate and register it after validation.
