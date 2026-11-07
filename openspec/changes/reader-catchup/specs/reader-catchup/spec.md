## ADDED Requirements

### Requirement: Finite explained catch-up selection
Feeder SHALL provide a bounded queue of 10 or 20 candidates for 24 hours or 7 days with explicit selection reasons, coverage limits and source diversity.

#### Scenario: Reader requests a catch-up queue
- **WHEN** valid period, limit and optional source scope are requested
- **THEN** the result includes no muted article, at most two representatives per source host, deterministic ordering and a reason for each item
- **AND** the response identifies its observation window, scan limits and collection incompleteness

#### Scenario: Insufficient or backfilled data
- **WHEN** articles were imported initially, backfilled after recovery, have invalid/future publication times or have not been observed since migration
- **THEN** they are not presented as newly received events
- **AND** the queue may contain fewer items than its requested limit without claiming complete coverage

### Requirement: Explicit shared interests
Feeder SHALL let readers configure literal interest terms and priority feeds as shared settings, with revision conflict protection.

#### Scenario: Valid interest update
- **WHEN** at most 20 terms of at most 80 Unicode characters and at most 20 existing priority feeds are submitted with the current settings revision
- **THEN** the settings are saved and matching candidates expose the matching terms or priority source

#### Scenario: Concurrent or invalid update
- **WHEN** a stale revision, unknown feed or oversized settings payload is submitted
- **THEN** the request is rejected without replacing the current settings

### Requirement: Revision-specific acknowledgement
Feeder SHALL keep catch-up confirmation and skipping independent from article reading state and pinned to an exact content revision.

#### Scenario: Confirm or skip and undo
- **WHEN** the reader confirms or skips a current candidate and later undoes that action
- **THEN** only that revision's acknowledgement is created or removed
- **AND** read, star, visit and read-later values remain unchanged

#### Scenario: Newer revision arrives
- **WHEN** an article changes after an earlier version was acknowledged
- **THEN** the new revision can be selected again
- **AND** an acknowledgement for a stale version is rejected with 409 rather than acknowledging the new version

### Requirement: Observed article revision history
Feeder SHALL retain bounded snapshots of substantive changes received during normal collection, with source and observation timestamps and explicit history limits.

#### Scenario: Changed article is collected
- **WHEN** a previously stored article has a different title, body or URL in an accepted refresh
- **THEN** the prior and new content versions can be inspected without changing the article's existing read/star state
- **AND** a change after the last visit can be shown as an update

#### Scenario: Unchanged or legacy article
- **WHEN** a refresh changes only publication metadata or the database is upgraded from a legacy version
- **THEN** Feeder does not invent a new substantive edit or a historical source revision
- **AND** unavailable or pruned history is identified explicitly

### Requirement: Conservative reversible related coverage
Feeder SHALL group conservatively matching coverage with an explanation while preserving each article and its independent reading state.

#### Scenario: Multiple copies or closely related articles
- **WHEN** canonical URLs or conservative content/title and time criteria match
- **THEN** readers can expand related articles and open each original
- **AND** grouping does not delete articles or propagate read/star changes

#### Scenario: Reader corrects a grouping
- **WHEN** an article is excluded from related grouping
- **THEN** it becomes an independent catch-up candidate and the exclusion can be reversed

### Requirement: Accessible existing Reader integration
Feeder SHALL integrate catch-up, revisions and related articles into the existing React Reader using accessible controls and absolute times.

#### Scenario: Read and return on desktop or mobile
- **WHEN** a keyboard, pointer or touch user opens and closes a catch-up article
- **THEN** the existing detail behavior is retained and focus can return to the item
- **AND** controls and long content remain usable at 390px in light/dark themes

#### Scenario: Request is superseded or fails
- **WHEN** a request fails, conflicts or completes after the user changed selection
- **THEN** the UI explains the failure or ignores the obsolete response without presenting stale data as current

### Requirement: Bound related expansion work
The system SHALL evaluate at most 100 potentially related groups per catch-up response and SHALL expose truncation when that limit or an individual related scan limit is reached. Counts SHALL describe the evaluated portion.

#### Scenario: Many independent duplicate groups
- **WHEN** the ordered candidates require more than 100 related expansions
- **THEN** selection stops at the bound, truncated is true, and the UI describes partial evaluation without claiming that the 5000-candidate bound was necessarily reached
