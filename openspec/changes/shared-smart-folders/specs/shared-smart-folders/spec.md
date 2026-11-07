## ADDED Requirements
### Requirement: Shared validated snapshots
The system SHALL persist at most 100 uniquely named, bounded snapshots in the shared database and provide CRUD through the authenticated API.
#### Scenario: Invalid snapshot
- **WHEN** a client submits a malformed snapshot
- **THEN** the operation is rejected without changing records
#### Scenario: Reader filter validation
- **WHEN** a snapshot is created or its conditions are updated
- **THEN** reader filter syntax, date ranges, and referenced sources are validated using the reader rules, including later and include_muted
#### Scenario: Source removed after saving
- **WHEN** a saved source is removed
- **THEN** the snapshot remains stored and can be renamed, but cannot be applied by broadening its scope
### Requirement: Optimistic concurrency
The system SHALL require the current revision for updates and deletes.
#### Scenario: Stale revision
- **WHEN** a client modifies a stale record
- **THEN** the operation returns conflict and preserves the newer record
### Requirement: Safe application and local retention
The system SHALL retain browser-local searches and copy them only on explicit request. Shared application SHALL use existing source validation without broadening scope.
#### Scenario: Missing source
- **WHEN** an applied snapshot references a removed source
- **THEN** it is not silently broadened to all sources
