## ADDED Requirements

### Requirement: Web-only successor
Feeder SHALL provide the existing Web application's agreed capabilities through a Rust backend and React frontend, while excluding the retired desktop application and cross-device synchronization.

#### Scenario: Migration scope is reviewed
- **WHEN** a migration phase is planned
- **THEN** its Web requirements, excluded desktop and synchronization features, and verification state are traceable to the inventory

### Requirement: Preserve Web data and recovery
Feeder SHALL migrate existing Web data and operational state through a validated process with a consistent recoverable backup, without simultaneous writes from old and new applications.

#### Scenario: Cutover is attempted
- **WHEN** the operator performs a production cutover
- **THEN** writers are stopped, backup and restore are verified, migrated data is reconciled, and rollback including subsequent writes is defined

### Requirement: Preserve security and usability
Feeder SHALL retain the Web application's applicable authentication, authorization, input protection, renderer isolation, accessibility and responsive interaction requirements.

#### Scenario: A feature is declared complete
- **WHEN** its migration is reviewed
- **THEN** tests verify the applicable behavior, security boundaries, data persistence, navigation and responsive themes

### Requirement: Staged acceptance
The migration SHALL proceed in independently testable phases with explicit dependencies, release gates and documented unverified external integrations.

#### Scenario: A phase is complete
- **WHEN** its exit criteria have passed
- **THEN** the canonical specifications, operating documentation, tests and verification record are updated before the next dependent cutover
