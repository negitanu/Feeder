## ADDED Requirements

### Requirement: Browser mutation protection
Internal browser API writes SHALL require an unpredictable bounded expiring session cookie and CSRF header, plus valid configured Origin when supplied. Cross-site fetches SHALL be rejected. Tokens SHALL stay out of URLs, browser storage, logs and configuration exports. Key administration SHALL require the browser session, never a public API token.
#### Scenario: Missing or stale browser proof
- **WHEN** a browser write lacks a valid session/CSRF or has an invalid Origin
- **THEN** it fails before mutation and the UI permits explicit recovery without replaying the action

### Requirement: Hashed key lifecycle and scopes
The settings hub SHALL issue expiring read/write keys once, list only safe metadata and revoke keys. Only SHA256 hashes SHALL be persisted. Public API authentication SHALL require Bearer independent from cookies and enforce persistent per-key minute limits with atomic checks.
#### Scenario: Read scope, expiry, revocation and throttling
- **WHEN** a read key writes, an expired/revoked key authenticates or a valid key exceeds its minute limit
- **THEN** the request is rejected with403/401/429 respectively, with Bearer challenge or Retry-After where applicable and no resource changes

### Requirement: Core legacy public API
The public API SHALL provide RSS subscription and article read/state routes under `/api/v1`, use data/error envelopes and deterministic ID pagination, validate complete inputs and reuse safe collection/state logic. Read GETs and state changes SHALL NOT create history visits. Unsupported migrated-source features SHALL fail explicitly.
#### Scenario: Ascending incremental articles
- **WHEN** a valid key requests after_id with bounded limit
- **THEN** matching new IDs are returned once in ascending order with continuation metadata and no reader visit

### Requirement: Explicit container access boundary
Local mode SHALL limit production Host authorities to local access. Tunnel mode SHALL require an explicit HTTPS origin, secure cookies and an unpublished origin behind Access JWT validation. Forwarded identity/scheme headers SHALL NOT bypass checks. Health SHALL remain available without credentials.
#### Scenario: Invalid publication configuration
- **WHEN** mode/origin is invalid or the tunnel override is resolved without a public origin, pinned image or token-file configuration
- **THEN** startup or configuration resolution fails without exposing a fallback unauthenticated public port

### Requirement: Preserved database migration
Recognized v6 databases SHALL be backed up before v7 migration and preserve articles, state, visits, source arrangement and existing settings. Unknown schemas and existing backup paths SHALL be refused.
#### Scenario: Migration and recovery
- **WHEN** a generated v6 library upgrades, restarts and is restored into a separate volume
- **THEN** all original state and new key metadata/rate settings remain intact and raw keys are absent from persisted rows
