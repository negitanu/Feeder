## ADDED Requirements
### Requirement: Start tunnel by default
Compose SHALL include the tunnel service without requiring a profile.
#### Scenario: Standard startup
- **WHEN** the user starts Compose with a configured token
- **THEN** Web, worker and tunnel are included in the project.
