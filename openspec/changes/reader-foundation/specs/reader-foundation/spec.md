## ADDED Requirements
### Requirement: Persistent reader slice
Feeder SHALL list and filter generated articles, show article text, and persist read and starred changes in its own SQLite database.
#### Scenario: Restart persistence
- **WHEN** an article state is changed and the service restarts
- **THEN** the same state is returned
### Requirement: Safe local HTTP boundary
Feeder SHALL bind loopback by default, reject cross-site state mutations and avoid rendering untrusted article HTML.
#### Scenario: Cross-site mutation
- **WHEN** a mutation comes from an unrelated Origin
- **THEN** the request is rejected without changing article state
### Requirement: Preserve source databases
Feeder SHALL refuse unsupported existing database schemas before modification and create sample content only with explicit opt-in.
#### Scenario: Existing source database
- **WHEN** the configured database contains non-Feeder tables
- **THEN** startup fails without modifying the database
### Requirement: Accessible reader UI
The reader SHALL provide Japanese labels, progressive article loading, read/star controls, light/dark/system themes and responsive layout.
#### Scenario: More articles
- **WHEN** the reader reaches the end of a partially loaded list
- **THEN** additional articles load without numbered pagination
