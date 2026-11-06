## Purpose

Provide the corresponding Feeder reader and collection capability.

## Requirements

### Requirement: Per-feed polling interval
Feeder SHALL allow an RSS subscription to inherit the global interval or use an integer interval from 60 to 86400 seconds, in the UI and write-scoped public API.
#### Scenario: Update a feed interval
- **WHEN** an operator saves a valid interval or clears it to inherit
- **THEN** the setting is persisted, returned by feed APIs and included in non-secret configuration export
#### Scenario: Invalid update
- **WHEN** an interval is out of range or not a JSON integer
- **THEN** the entire edit is rejected without modifying the feed or creating a folder

### Requirement: Independent worker scheduling
Feeder SHALL use each explicit feed interval, otherwise the global interval, both when finding due work and when claiming the refresh lease.
#### Scenario: Interval changes before claim
- **WHEN** the feed is no longer due at claim time
- **THEN** the worker does not fetch it

### Requirement: Preserve existing Feeder data
Feeder SHALL back up and upgrade recognized Feeder v7 databases transactionally while rejecting unknown schemas and existing backup collisions.
#### Scenario: Upgrade existing installation
- **WHEN** a recognized Feeder v7 database is opened
- **THEN** a v7 backup is retained, existing data is preserved and all feeds initially inherit the global interval
