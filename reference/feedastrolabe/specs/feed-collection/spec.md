## Purpose

Provide the verified phase-one feed-collection capability.

## Requirements

### Requirement: Subscription management
The system SHALL register unique HTTP(S) RSS/Atom URLs, assign a folder, allow folder changes and delete subscriptions with their articles. It SHALL reject invalid feeds and show duplicate registrations without losing data.

#### Scenario: Register and organize
- **WHEN** a valid RSS or Atom URL and folder are submitted
- **THEN** the feed and initial articles are stored and appear in the sidebar

#### Scenario: Duplicate or invalid feed
- **WHEN** an existing URL or a non-feed URL is submitted
- **THEN** an explanatory error is shown without adding a duplicate subscription

#### Scenario: Delete subscription
- **WHEN** a subscription is deleted
- **THEN** its articles are deleted and other subscriptions remain

### Requirement: Idempotent refresh
The system SHALL provide manual and periodic refresh, update existing content without resetting read/star state, persist HTTP validators and process 304 responses. A failed refresh SHALL preserve previous articles and expose an error.

#### Scenario: Repeat refresh
- **WHEN** the same entry is retrieved twice after it was read and starred
- **THEN** exactly one article exists with updated content and both states retained

#### Scenario: Unchanged feed
- **WHEN** a server responds 304 to conditional headers
- **THEN** articles and validators are preserved and the check time is updated

#### Scenario: Remote failure
- **WHEN** one feed returns invalid content or an HTTP error
- **THEN** its error is visible and other feeds can still update


### Requirement: Safe website discovery and subscription preview
The system SHALL accept an HTTP(S) site or RSS/Atom URL in 集める, reuse bounded public-address-validated fetching, and discover at most eight deduplicated RSS/Atom alternate links against the final redirect URL. Candidate previews SHALL show a validated title, source URL and up to five latest articles before explicit subscription confirmation. Preview and discovery SHALL NOT register feeds or articles. Confirmation SHALL require CSRF and a session-bound signed plan expiring after fifteen minutes, revalidate/refetch the feed and preserve duplicate subscriptions.

#### Scenario: Website with multiple feeds
- **WHEN** a site declares multiple relative or absolute RSS/Atom alternate links
- **THEN** candidates can be compared and individually previewed without creating a subscription

#### Scenario: Confirm direct feed
- **WHEN** a direct valid RSS/Atom URL is previewed and explicitly confirmed
- **THEN** exactly one subscription and initial articles are stored in the selected folder

#### Scenario: Unsafe unavailable or stale input
- **WHEN** a candidate redirects to a private address, is invalid, has no alternate feed, exceeds bounds, or a plan is modified, expired or used in another session
- **THEN** a helpful error is shown without registration or read/star changes
