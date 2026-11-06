## Purpose

Provide deliberate feed onboarding, controlled periodic collection and subscription portability.

## Requirements
### Requirement: Bounded safe discovery
Feeder SHALL discover at most eight deduplicated RSS/Atom alternate URLs against the final site URL using the existing public-address-pinned fetch protections.
#### Scenario: Relative alternate links
- **WHEN** an HTML site provides valid relative feed links
- **THEN** candidate URLs resolve against the final redirect URL without adding subscriptions
### Requirement: Deliberate feed confirmation
Feeder SHALL preview at most five entries without database writes and require an expiring session-bound plan before preview confirmation refetches and commits a feed.
#### Scenario: Invalid confirmation
- **WHEN** a plan is expired, altered, reused or presented in another session
- **THEN** confirmation fails without adding subscriptions or changing article states
### Requirement: Controlled periodic collection
Feeder SHALL persist an explicit collection switch and interval, refresh only due unpaused feeds in a separate worker and exclude overlapping refresh jobs.
#### Scenario: Disabled or paused source
- **WHEN** collection is disabled or a feed is paused
- **THEN** the worker does not start a new refresh for that scope
#### Scenario: Competing refresh
- **WHEN** manual refresh and workers attempt the same feed concurrently
- **THEN** at most one active lease performs collection and errors preserve existing articles
### Requirement: Duplicate-aware OPML portability
Feeder SHALL preview bounded OPML, require explicit import and export escaped subscriptions without secrets or article data.
#### Scenario: Repeated import
- **WHEN** OPML containing existing or repeated URLs is imported
- **THEN** valid new subscriptions are added once in unpaused state and the result reports skipped and invalid entries
#### Scenario: Unsafe XML
- **WHEN** OPML includes entity declarations, excessive size or excessive nesting
- **THEN** it is rejected without database writes or remote access
### Requirement: Versioned safe migration
Feeder SHALL back up and migrate only recognized v2 databases while preserving subscriptions, article identities, states and folders.
#### Scenario: Migration and restart
- **WHEN** a known v2 database is opened and then reopened
- **THEN** its data and collection settings persist with a retained backup
