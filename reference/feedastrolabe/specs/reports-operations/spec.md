# reports-operations Specification

## Purpose
Provide immutable classified research exports, explicitly approved notifications, recoverable library operations and bounded scoped management APIs.
## Requirements
### Requirement: Portable sourced outputs
The system SHALL export selected research records with citation/evidence manifests and classification filtering, protecting CSV and explicit STIX mapping.

#### Scenario: Portable sourced outputs acceptance
- **WHEN** a report contains restricted evidence and a CSV cell begins with a formula marker
- **THEN** restricted data is excluded by default and the cell is safely encoded

#### Scenario: Immutable attributed local selection
- **WHEN** a selected record is shared with a nonpublic workspace or its source URL contains a known secret marker
- **THEN** default exports exclude the shared nonpublic record and reject the secret-bearing metadata, while accepted records retain immutable IDs and hashes

### Requirement: Controlled notification delivery
The system SHALL preview opt-in outgoing payloads, validate recipients/endpoints and retry durable notifications without duplicate successful sends.

#### Scenario: Controlled notification delivery acceptance
- **WHEN** a webhook job is retried after completion
- **THEN** the completed delivery is not sent again and its audit record remains

#### Scenario: Exact opt-in bounded delivery
- **WHEN** an approved preview changes classification or an outbound attempt has an ambiguous transport result
- **THEN** the system stops dispatch or automatic replay and retains safe audit metadata without broadening recipient or credential access

### Requirement: Recoverable operations
The system SHALL show safe worker/job health, create consistent bounded-generation backups and reject incomplete or online restore.

#### Scenario: Recoverable operations acceptance
- **WHEN** a backup lacks a referenced evidence blob or workers are active during restore
- **THEN** restore fails without replacing the live library

#### Scenario: Disabled schedule and bounded generations
- **WHEN** the operator explicitly enables periodic backups with a bounded generation policy
- **THEN** the worker verifies the database and every referenced blob before success and retires only recorded verified generations

### Requirement: Bounded management APIs
The system SHALL provide documented scoped settings/bulk-state/OPML operations with safe limits and record CI setup separately.

#### Scenario: Bounded management APIs acceptance
- **WHEN** a bulk article-state request exceeds its limit or uses a read key
- **THEN** it is rejected without changing articles

#### Scenario: Settings and OPML atomic management
- **WHEN** settings contain an invalid field value or OPML contains a forbidden local URL or entity declaration
- **THEN** the operation is rejected without partial library changes and key issuance remains UI-only
