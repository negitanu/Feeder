## ADDED Requirements
### Requirement: Direct interval editing
Users SHALL directly replace collection interval seconds and save an integer from 60 through 86400. Empty or invalid drafts SHALL remain editable without sending updates.
#### Scenario: Replace seconds
- **WHEN** a user clears and types a new valid interval
- **THEN** the complete draft is accepted and saved without numeric coercion during editing.
### Requirement: Collection-ready defaults
Newly imported subscriptions SHALL start unpaused. A newly created collection settings row SHALL be enabled. Existing saved pause/disabled settings SHALL be preserved.
#### Scenario: Import new and existing feeds
- **WHEN** a confirmed OPML import contains new and existing URLs
- **THEN** only new subscriptions start unpaused, existing states are unchanged, and import does not itself fetch remote URLs.
