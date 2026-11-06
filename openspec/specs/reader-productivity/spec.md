## Purpose

Provide durable reading history, detailed shared filter scope and atomic bulk state operations.

## Requirements
### Requirement: Independent persistent visits
Feeder SHALL record explicit detail visits atomically with read state, retain star state and provide a recent-history view that survives restart.
#### Scenario: Non-visit mutations
- **WHEN** an article is fetched, starred or bulk marked read without a detail visit
- **THEN** its visit history remains unchanged
### Requirement: Shared detailed filter scope
Feeder SHALL use the same bounded validated literal, source, calendar-date and ordering filters for listing and scoped mutations. Calendar dates SHALL follow the shared UTC or Asia/Tokyo display timezone, including inclusive start and exclusive next-day end, and exclude unknown publication dates.
#### Scenario: Invalid condition
- **WHEN** a request has invalid dates, missing sources, invalid CSV, unknown scope or invalid ordering
- **THEN** it is rejected before state changes
#### Scenario: Japanese calendar boundary
- **WHEN** Asia/Tokyo is selected and a reader filters a calendar day or bulk marks it read
- **THEN** articles from 15:00 UTC on the preceding day through before 15:00 UTC on that day match consistently, including offset timestamps, while outside and unknown dates do not
### Requirement: Scoped complete bulk read
The UI SHALL expose 全て既読 as a prominent theme-aware control and mark every article matching the active filters including undrawn articles.
#### Scenario: Unloaded matches
- **WHEN** all-read is applied before all matches are loaded
- **THEN** all matching articles are read, unrelated articles and history are preserved
### Requirement: Atomic selected states
Feeder SHALL validate at most 100 distinct in-scope selected IDs before atomically updating read/star state.
#### Scenario: Invalid member
- **WHEN** a bulk selection includes an unknown or out-of-scope ID or duplicates
- **THEN** the complete action is rejected without partial changes
### Requirement: Preserved migration and reader state
Feeder SHALL preserve article identity, content, state and collection settings across recognized v3 migration, and retain loaded-list selection and position during detail navigation.
#### Scenario: Detail round trip
- **WHEN** a selected article is opened and closed after more items load
- **THEN** the loaded list, selected IDs and list position remain available
