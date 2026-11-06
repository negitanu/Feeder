## ADDED Requirements
### Requirement: Bound long sidebar names
Sidebar rows SHALL remain within their container with long folder/feed names. Titles SHALL truncate while preserving operation controls and paused state.
#### Scenario: Narrow sidebar
- **WHEN** a long Japanese or unbroken name is shown in a narrow sidebar
- **THEN** the title truncates, its full value remains available, and the menu stays within the row.
