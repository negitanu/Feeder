## ADDED Requirements

### Requirement: Shareable reader location
The reader SHALL encode its view, search, source, detailed filters and selected article in a validated canonical URL, and restore a deep link after settings and library load.
#### Scenario: Reload a deep link
- **WHEN** a valid reader URL is reloaded
- **THEN** the matching list is loaded and the article is visited using the existing visit semantics.

### Requirement: Bounded browser history
The reader SHALL restore loaded list, selection and scroll on back and forward using bounded in-memory snapshots, without history push loops or stale article openings.
#### Scenario: Return to a loaded list
- **WHEN** a reader opens an article and navigates back
- **THEN** the prior loaded list and scroll position are restored without an additional article visit.
