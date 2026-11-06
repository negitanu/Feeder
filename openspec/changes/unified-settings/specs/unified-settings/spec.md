## ADDED Requirements
### Requirement: Unified settings workspace
Settings SHALL contain subscription creation, collection controls, OPML, presentation, export and key management with section shortcuts. The main navigation SHALL not expose a separate Collect destination.
#### Scenario: Manage subscriptions
- **WHEN** the reader opens Settings
- **THEN** subscriptions and OPML are available with the existing confirmation and persistence behavior.
#### Scenario: Navigate sections
- **WHEN** the user activates a settings section shortcut
- **THEN** its heading is reachable without resetting settings or reader state.
