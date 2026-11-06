# Reader detail navigation

## Purpose

Navigate the captured loaded article order from the detail panel.

## Requirements

### Requirement: Navigate loaded article details
Feeder SHALL present previous, position and next controls at the end of an article detail for the list order captured when the article was selected.
#### Scenario: Move between articles
- **WHEN** a reader activates next or previous within the loaded range
- **THEN** the requested detail opens, its visit is recorded and the loaded list and selections remain intact
#### Scenario: First or last article
- **WHEN** the current article is at a boundary of the captured range
- **THEN** the corresponding navigation control is disabled without requesting another article

### Requirement: Accessible and recoverable navigation
Feeder SHALL support keyboard activation and preserve stale-response and dismissal protections during article navigation.
#### Scenario: Navigation fails
- **WHEN** fetching or recording a visit fails
- **THEN** the user sees an error and the previously displayed detail remains available without reporting a successful visit
