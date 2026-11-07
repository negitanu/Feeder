## ADDED Requirements
### Requirement: Independent read-later queue
The system SHALL persist a queue independently of read and starred state.
#### Scenario: Save and remove
- **WHEN** a user saves or removes an existing article
- **THEN** the queue reflects the requested state without changing read or starred state.
#### Scenario: Missing article
- **WHEN** a user changes queue state for an unknown article
- **THEN** the request fails without adding an entry.
### Requirement: Explicit visit removal
The system SHALL default to manual removal and optionally remove a queued article on an actual visit.
#### Scenario: Mark read
- **WHEN** an article is marked read without a visit
- **THEN** its queue state remains unchanged.
