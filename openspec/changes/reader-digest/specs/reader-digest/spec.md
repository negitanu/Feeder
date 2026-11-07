## ADDED Requirements

### Requirement: Source digest with actual counts
Feeder SHALL summarize visible saved articles published in the past 24 hours or seven days, optionally scoped to one feed or folder, without sending notifications or generating AI text.

#### Scenario: Invalid publication dates
- **WHEN** saved articles have unknown, invalid or future publication times
- **THEN** those articles are excluded from counts and digest items

#### Scenario: Muted article
- **WHEN** an enabled mute rule matches an article
- **THEN** the article is excluded from both digest counts and items without deleting the original

### Requirement: Deterministic bounded display
Feeder SHALL select the newest ten articles per source and newest 100 overall, order equal publication times by ID descending, and report total counts and truncation explicitly.

#### Scenario: Digest exceeds display limits
- **WHEN** qualifying articles exceed either display limit
- **THEN** the response reports actual total, displayed count and truncation, and the UI explains the limit

#### Scenario: Absolute publication time
- **WHEN** a reader opens the digest
- **THEN** article times use the configured UTC or JST timezone and existing article detail navigation
