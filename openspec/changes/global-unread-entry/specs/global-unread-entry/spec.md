## ADDED Requirements

### Requirement: Global unread navigation
Feeder SHALL provide a global unread entry that clears existing source, search and advanced filters when explicitly activated.
#### Scenario: Navigate from a filtered library
- **WHEN** the reader selects 未読 while another source or search is active
- **THEN** unread articles across the library are shown and prior scopes are cleared
#### Scenario: Scope unread afterward
- **WHEN** the reader selects a source or adds a search after entering unread
- **THEN** the unread filter remains applied and the global entry is no longer marked current

### Requirement: Library-wide unread count
Feeder SHALL display a count independent of the active source and update it after successful local read state operations.
#### Scenario: Visit a scoped unread article
- **WHEN** an unread article is opened from a folder
- **THEN** the unread badge reflects the new global count without replacing the folder scope
