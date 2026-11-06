# network-intelligence Specification

## Purpose
Record attributed passive DNS, registration, certificate and routing observations with explicit coverage and immutable history.
## Requirements
### Requirement: Temporal network observations
The system SHALL preserve attributed time-specific DNS/RDAP/CT/routing/RPKI records and field changes.

#### Scenario: Temporal network observations acceptance
- **WHEN** a watched domain DNS answer changes
- **THEN** both observations and a sourced change event remain available

### Requirement: Uncertainty-aware interpretation
The system SHALL distinguish missing coverage, observed absence, certificate issuance and confirmed endpoint facts.

#### Scenario: Uncertainty-aware interpretation acceptance
- **WHEN** CT reports a certificate containing a domain name
- **THEN** the UI records issuance without claiming the domain is actively serving that certificate

### Requirement: Passive bounded monitoring
The system SHALL use opted-in public identifiers with read-only allowlisted providers and budgeted schedules.

#### Scenario: Passive bounded monitoring acceptance
- **WHEN** a lookup requests a private target or an unapproved referral endpoint
- **THEN** the request is rejected before accessing it

#### Scenario: Scoped failure and DNS TTL semantics
- **WHEN** DNS returns NXDOMAIN, TTL-only decay, SERVFAIL or an expired observed TTL
- **THEN** scoped absence, unchanged RRset, unknown failure preserving last-good and stale TTL are distinguished without implying global absence

#### Scenario: Prefix origin and consent validation
- **WHEN** an RPKI lookup lacks an explicit public prefix-origin pair or a watch has no public consent
- **THEN** no request is sent and a valid observed pair is never converted into ownership or hijack attribution

#### Scenario: Bounded retention and historical anchors
- **WHEN** configured network metadata retention runs
- **THEN** the latest observation and referenced quotation/note anchors remain, while permitted old metadata is removed without deleting fixed evidence or article state
