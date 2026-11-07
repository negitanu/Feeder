## ADDED Requirements

### Requirement: Bounded public social continuation
Feeder SHALL collect social sources with a finite per-refresh page budget, persist continuation and a completed high-water mark, and preserve public author-only and no-repost filtering.

#### Scenario: Initial or recovery collection spans refreshes
- **WHEN** a source has more than two pages to collect
- **THEN** the accepted refresh stores its entries and continuation atomically and reports pending work
- **AND** initial or recovery ingestion classification persists until the cycle completes
- **AND** the completed high-water mark remains unchanged while work is pending

#### Scenario: Provider repeats a cursor across refreshes
- **WHEN** a provider returns a direct or multi-step cursor cycle
- **THEN** persistent bounded cycle-detection state detects the cycle across refresh boundaries
- **AND** the response fails without advancing the committed checkpoint or claiming complete coverage

#### Scenario: Previous high-water mark disappeared
- **WHEN** collection reaches the end or passes a Mastodon high-water boundary without seeing the previous mark
- **THEN** the accepted response records possible missing coverage
- **AND** articles discovered in that response are stored with recovery context rather than normal context

### Requirement: Preserve source publication and edit timestamps
Feeder SHALL preserve original publication time independently from a source-provided edit time.

#### Scenario: Mastodon edited status is collected
- **WHEN** an accepted public status contains a non-null edited_at
- **THEN** its article revision source_updated_at reflects edited_at
- **AND** its published time remains created_at

#### Scenario: Mastodon status has not been edited
- **WHEN** edited_at is absent or null
- **THEN** created_at remains the publication time and the initial update timestamp

### Requirement: Shared retry and lease protection
Feeder SHALL apply Retry-After seconds or HTTP dates on 429/503 and exponential failure backoff to both manual and scheduled collection, and SHALL reject commits from expired or replaced leases.

#### Scenario: Rate limit or temporary service failure
- **WHEN** an accepted attempt fails with 429 or 503 and a valid Retry-After
- **THEN** the persisted retry deadline is no earlier than that header or the exponential backoff deadline
- **AND** manual and scheduled refresh wait until the deadline

#### Scenario: Old worker completes
- **WHEN** a response belongs to an expired or replaced lease
- **THEN** entries, revisions, checkpoints and health remain unchanged

### Requirement: Host concurrency and request isolation
Feeder SHALL bound concurrent requests per host while preserving URL validation, redirect checks and DNS pinning.

#### Scenario: Multiple feeds share a host
- **WHEN** fetches to the same host overlap
- **THEN** at most one network request to that host is active within the process
- **AND** unrelated hosts may proceed independently

### Requirement: Truthful coverage and source reset
Feeder SHALL expose collection state, retry time, pending work and possible gaps, and SHALL clear source-specific collection state when a source URL or API identity changes.

#### Scenario: Catch-up requests coverage
- **WHEN** a scoped source is paused, disabled, uncollected, failing, waiting, pending, stale or possibly incomplete
- **THEN** the coverage helper reports incomplete collection

#### Scenario: Source changes
- **WHEN** an RSS URL or social provider, server or remote identity changes
- **THEN** old runtime, validators and diagnostics cannot be reused as evidence of success for the new source
- **AND** saved articles and reader state are preserved

### Requirement: Additive migration and ingestion integrity
Feeder SHALL add runtime storage only through the v10 migration and SHALL classify ingestion before applying successful-response metadata.

#### Scenario: Initial, normal and recovery responses reach article storage
- **WHEN** lease-owned parsed responses are stored before health is updated
- **THEN** new articles retain the appropriate initial, normal or recovery ingestion kind
- **AND** existing read and star values survive repeated collection

#### Scenario: Legacy schema upgrade
- **WHEN** the v10 migration is applied to a recognized older database
- **THEN** existing v9 migration definitions remain unchanged and collection state is additive
