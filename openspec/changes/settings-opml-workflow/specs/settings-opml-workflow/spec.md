## ADDED Requirements
### Requirement: Discoverable settings
Feeder SHALL provide settings through the main navigation and preserve existing settings persistence.
#### Scenario: Open settings
- **WHEN** the user chooses settings
- **THEN** presentation, export and API key controls are available.
### Requirement: Independent OPML workflow
OPML preview and import SHALL have independent busy/error state and retain the existing explicit confirmation and paused import behavior.
#### Scenario: Cancel preview
- **WHEN** the user cancels an OPML preview
- **THEN** pending preview responses cannot restore it and the same file can be selected again.
#### Scenario: Import fails
- **WHEN** an import fails
- **THEN** the preview remains available for retry and file changes are blocked only while import is pending.
