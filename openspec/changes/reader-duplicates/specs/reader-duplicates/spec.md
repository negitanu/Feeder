## ADDED Requirements
### Requirement: Non-destructive duplicate groups
The system SHALL group scanned HTTP(S) article URLs after removing fragments and known tracking parameters, preserving semantic query parameters and stored rows.
#### Scenario: Tracking-only differences
- **WHEN** two originals differ only in fragment or utm_*, fbclid, or gclid parameters
- **THEN** they appear together with their individual source, title, URL, read and star states.
#### Scenario: Semantic differences
- **WHEN** URLs differ in remaining query parameters
- **THEN** they are not grouped solely as tracking variants.
### Requirement: Bounded and explicit coverage
The API SHALL scan at most 1000 articles and expose whether more articles exist; group counts SHALL describe only the scan and groups SHALL be paginated.
#### Scenario: More than 1000 articles
- **WHEN** the scan is truncated
- **THEN** the API and interface indicate that older articles are excluded.
### Requirement: Explicit original actions
The optional view SHALL show every original in a displayed group and allow opening or marking an individual original read without modifying other copies; users SHALL be able to return to the normal view.

#### Scenario: Individual read action
- **WHEN** a reader marks one original read in a duplicate group
- **THEN** only that original changes and other originals retain their read and star states
