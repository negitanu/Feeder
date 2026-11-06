## Purpose

Provide the first direct-feed collection and organization stage for Feeder.

## Requirements
### Requirement: Direct subscriptions and organization
Feeder SHALL register valid unique direct RSS/Atom URLs, retain folders as stable identities, and filter articles by feed or folder.
#### Scenario: Register a feed
- **WHEN** a valid direct feed is submitted
- **THEN** its subscription and initial articles are committed atomically and duplicate URLs are rejected
### Requirement: Durable collection
Feeder SHALL support manual conditional refresh without resetting read/star state and SHALL retain previous data on errors.
#### Scenario: Repeated refresh
- **WHEN** an existing entry is refreshed after being read and starred
- **THEN** content updates without duplicates or state loss
#### Scenario: Pause and delete
- **WHEN** a feed is paused
- **THEN** collection stops and articles remain until explicit deletion
### Requirement: Safe external fetching
Feeder SHALL validate and pin public destination addresses on each redirect and enforce bounded timeout, body size and entry count.
#### Scenario: Unsafe destination
- **WHEN** a feed URL resolves or redirects to a private or otherwise prohibited address
- **THEN** collection fails before accessing that destination and stored data remains intact
### Requirement: Source controls and schema migration
Feeder SHALL expose contextual feed editing and migrate recognized Feeder v1 data without modifying unknown databases.
#### Scenario: Existing reader database
- **WHEN** a known v1 Feeder database is opened
- **THEN** article IDs, content and read/star state are preserved in the new schema

### Requirement: Read-only legacy inspection
The migration tooling SHALL inspect explicitly selected legacy database copies in read-only mode and report structural integrity and aggregate counts without row contents or credentials.
#### Scenario: Inspection input
- **WHEN** an existing database is inspected or a missing path is supplied
- **THEN** inspection leaves the database unchanged and never creates a missing database file
