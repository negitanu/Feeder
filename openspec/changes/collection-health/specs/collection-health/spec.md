## ADDED Requirements

### Requirement: Collection completion diagnostics
Feeder SHALL record last successful collection and consecutive failures only for accepted lease-owned completions, without modifying article read or star states.

#### Scenario: Accepted success after failures
- **WHEN** the current refresh owner completes successfully
- **THEN** the completion transaction records success time and resets consecutive failures

#### Scenario: Stale completion
- **WHEN** a completion belongs to an expired owner or a paused or deleted source
- **THEN** it does not change diagnostic counters or article data

### Requirement: Bounded schedule overview
Feeder SHALL expose last check, last success, errors, consecutive failures, effective interval and next due time in a read-only bounded settings view.

#### Scenario: Paused subscription
- **WHEN** a feed is paused or global automatic collection is disabled
- **THEN** next due is absent and no scheduled refresh is implied

#### Scenario: Never checked subscription
- **WHEN** collection is enabled and a subscription has never been checked
- **THEN** it is immediately due and success remains unknown
