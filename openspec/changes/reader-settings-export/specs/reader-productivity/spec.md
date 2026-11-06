## MODIFIED Requirements

### Requirement: Shared detailed filter scope
Feeder SHALL use the same bounded validated literal, source, calendar-date and ordering filters for listing and scoped mutations. Calendar dates SHALL follow the shared UTC or Asia/Tokyo display timezone, including inclusive start and exclusive next-day end, and exclude unknown publication dates.
#### Scenario: Invalid condition
- **WHEN** a request has invalid dates, missing sources, invalid CSV, unknown scope or invalid ordering
- **THEN** it is rejected before state changes
#### Scenario: Japanese calendar boundary
- **WHEN** Asia/Tokyo is selected and a reader filters a calendar day or bulk marks it read
- **THEN** articles from 15:00 UTC on the preceding day through before 15:00 UTC on that day match consistently, including offset timestamps, while outside and unknown dates do not
