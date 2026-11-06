## ADDED Requirements
### Requirement: Save reader searches
Readers SHALL save, apply and remove named searches in the current browser with a maximum of 50 entries. Persisted data SHALL be validated before use.
#### Scenario: Restore search
- **WHEN** a user applies a saved search
- **THEN** its committed query and filters are restored using the current display timezone, without restoring article selection or cursors.
#### Scenario: Missing source
- **WHEN** a saved source no longer exists
- **THEN** applying reports the problem without broadening the search.
