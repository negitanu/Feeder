## Purpose

Provide the verified phase-one local-operation capability.

## Requirements

### Requirement: Local runtime and data
The system SHALL run on localhost using documented commands, persist SQLite data outside Git, support a separate periodic worker and prevent concurrent refresh runs.

#### Scenario: Restart
- **WHEN** the application restarts with the same data directory
- **THEN** subscriptions, articles and session signing key are retained

#### Scenario: Concurrent refresh
- **WHEN** a refresh is requested while another refresh holds the lock
- **THEN** it is rejected as busy without starting a second refresh

### Requirement: Input and content protection
The system SHALL require CSRF tokens for mutations, use parameterized SQL, strip unsafe feed HTML and permit only public HTTP(S) feed destinations on ports 80/443 with redirect revalidation and pinned DNS connections. Fetches SHALL have size, timeout and redirect limits.

#### Scenario: Unsafe destination
- **WHEN** a feed or redirect targets localhost or a private IP
- **THEN** the fetch is rejected

#### Scenario: Unsafe HTML
- **WHEN** an article contains scripts or javascript links
- **THEN** the article view cannot execute them

#### Scenario: Missing CSRF token
- **WHEN** a mutation is submitted without a valid session token
- **THEN** the request returns 400 and no data changes
