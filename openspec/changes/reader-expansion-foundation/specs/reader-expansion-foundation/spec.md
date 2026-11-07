## ADDED Requirements
### Requirement: Additive reader feature migration
Recognized Feeder databases SHALL retain articles, state, visits, order and settings through a backed-up additive migration. Unknown schemas SHALL be rejected before mutation.
#### Scenario: Upgrade v8
- **WHEN** an existing recognized v8 library is opened
- **THEN** its backup is retained and the new tables are added without changing saved content or reading state.
### Requirement: Shared filtered reading semantics
Read-later and mute visibility SHALL apply consistently to article lists and condition-wide operations. Source metadata SHALL be available for publisher/title/absolute-time display.
#### Scenario: Read a filtered library
- **WHEN** a reader changes a queue or mute filter
- **THEN** pagination is scoped to those conditions and bulk operations do not silently widen the scope.
