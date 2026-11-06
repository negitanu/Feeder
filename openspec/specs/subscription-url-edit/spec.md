## Purpose

Repair RSS subscription addresses while preserving reading data and coordinating collection.

## Requirements

### Requirement: Repair an RSS subscription URL
Feeder SHALL let a reader or a write-scoped public API client change an RSS feed URL without fetching it immediately or replacing the subscription and its articles.
#### Scenario: URL changes
- **WHEN** a valid distinct public HTTP(S) URL is saved for an existing subscription
- **THEN** the feed and article identities, article states and history remain intact, old HTTP validators and refresh status are cleared, and later collection applies the existing outbound-network protections
#### Scenario: URL remains the same
- **WHEN** the saved URL normalizes to the current URL
- **THEN** the existing collection validators and status are retained

### Requirement: Atomic input and refresh conflict handling
Feeder SHALL reject invalid, duplicate and concurrently refreshing subscriptions before any requested edit or deletion changes persistent data.
#### Scenario: Duplicate or private URL
- **WHEN** an edit contains a duplicate, credential-bearing or non-public URL alongside other requested changes
- **THEN** the complete request is rejected and no folder, feed or article state is changed
#### Scenario: Refresh is active
- **WHEN** an unexpired refresh lease exists for the subscription and a browser or public API client edits or deletes it
- **THEN** Feeder returns 409 without changing the subscription or its articles; an expired lease does not block a later operation

### Requirement: Contextual accessible edit
Feeder SHALL expose the RSS URL in the subscription edit dialog and provide actionable errors without closing the dialog on rejection.
#### Scenario: Correct a subscription
- **WHEN** a user opens the subscription menu, edits the URL and saves
- **THEN** the new URL is saved without losing the current library or article state, and the dialog remains usable with keyboard and mobile layouts
