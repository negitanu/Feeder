## ADDED Requirements
### Requirement: Browser-local readability preferences
The Web reader SHALL allow each browser to adjust article font size, line height, and maximum text width independently of shared account/library settings. Supported ranges SHALL be 13–22 px, 1.5–2.4, and 40–90 ch respectively.

#### Scenario: Adjust reading appearance
- **WHEN** a reader changes a labelled readability control
- **THEN** the new bounded value is immediately applied through CSS custom properties on the document root and saved to that browser when storage is available

#### Scenario: Restore after reload
- **WHEN** the parent calls `applyReadability()` during browser startup
- **THEN** valid stored values are sanitized and applied before the parent renders, while malformed values or denied storage result in safe defaults

#### Scenario: Storage is unavailable
- **WHEN** reading from or writing to localStorage throws
- **THEN** the panel remains usable and communicates that the preference could not be saved

#### Scenario: Reset preferences
- **WHEN** the reader activates reset
- **THEN** all controls and root CSS variables return to their defaults and the reset state is persisted when possible

### Requirement: Bounded article layout
Readability styles SHALL affect article text typography and article body width only, and SHALL remain within the available viewport/container width.

#### Scenario: Narrow article container
- **WHEN** an article is displayed in a narrow viewport or floating detail panel
- **THEN** the configured maximum text width never forces horizontal page overflow
