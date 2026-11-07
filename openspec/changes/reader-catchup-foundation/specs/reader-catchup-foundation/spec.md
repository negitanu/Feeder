## ADDED Requirements

### Requirement: Backed-up additive schema upgrade
Feeder SHALL upgrade a recognized prior Feeder schema to v10 only after creating a consistent backup and SHALL preserve existing library state.

#### Scenario: Existing v9 library is upgraded
- **WHEN** a recognized v9 database contains article states, subscriptions, ordering, saved searches, mute rules and settings
- **THEN** startup creates a v9 backup, adds the new storage transactionally and retains all prior values
- **AND** the resulting database can be reopened and restored from a separate consistent copy

#### Scenario: Unknown schema is rejected
- **WHEN** a database has unrecognized objects or unsupported metadata
- **THEN** startup rejects it before any mutation or backup creation

### Requirement: Existing authentication boundary
Feeder SHALL protect new Reader feature APIs with the existing browser authentication, Origin and CSRF rules.

#### Scenario: Unauthorized mutation
- **WHEN** a request lacks required session authorization or CSRF proof, or uses a hostile Origin
- **THEN** the request is rejected without changing article or catch-up data

### Requirement: Independent reading state
Feeder SHALL preserve article read, star, visit and read-later state when computing recommendations, grouping articles or storing source revisions.

#### Scenario: Catch-up data is computed or refreshed
- **WHEN** a feature selects recommendations, forms a group or records an article revision
- **THEN** existing reader state changes only if the user separately performs an authorized reading action

### Requirement: Reader mode navigation
Feeder SHALL expose article-list and catch-up modes in the existing Reader and persist the active mode in the URL without losing the current filter or detail route.

#### Scenario: Navigate between modes
- **WHEN** a reader switches to catch-up and then uses browser Back or Forward
- **THEN** the corresponding mode and article detail are restored
- **AND** ordinary source selection and search return to the article list

#### Scenario: Read from a catch-up item
- **WHEN** the reader opens a selected catch-up article
- **THEN** the existing article detail and explicit reading actions are available with revision and related-article controls
- **AND** closing the detail returns focus to the source item when it is still present
