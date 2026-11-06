## ADDED Requirements

### Requirement: Web-only successor
Feeder SHALL provide the agreed RSS/SNS collection, reading and organization capabilities through a Rust backend and React frontend, while excluding research/investigation functionality, the retired desktop application and cross-device synchronization.

#### Scenario: Migration scope is reviewed
- **WHEN** a migration phase is planned
- **THEN** its Web requirements, excluded research, desktop and synchronization features, and verification state are traceable to the inventory

### Requirement: Preserve Feeder data and recovery
Feeder SHALL preserve its own operational data through version upgrades using consistent recoverable backups. Migration of old FeedAstrolabe data is out of scope.

#### Scenario: Feeder is upgraded
- **WHEN** the operator upgrades Feeder with existing Feeder data
- **THEN** backup and restore are verified and rollback including subsequent writes is defined

### Requirement: Preserve security and usability
Feeder SHALL retain the Web application's applicable authentication, authorization, input protection, any applicable renderer isolation, accessibility and responsive interaction requirements.

#### Scenario: A feature is declared complete
- **WHEN** its migration is reviewed
- **THEN** tests verify the applicable behavior, security boundaries, data persistence, navigation and responsive themes

### Requirement: Staged acceptance
The migration SHALL proceed in independently testable phases with explicit dependencies, release gates and documented unverified external integrations.

#### Scenario: A phase is complete
- **WHEN** its exit criteria have passed
- **THEN** the canonical specifications, operating documentation, tests and verification record are updated before the next dependent cutover
