## Purpose

Provide bounded, scoped reader continuation and recoverable incremental loading.

## Requirements

### Requirement: Stable bounded traversal
The internal reader SHALL traverse matching surviving articles without offset-shift skips when earlier rows are inserted or deleted, using deterministic ordering and a fixed initial article-ID high-water mark. Date ties, offset-equivalent timestamps and unknown dates SHALL follow the existing order/ID contract. Existing offset-only and public v1 pagination SHALL remain available.
#### Scenario: Insert and delete between batches
- **WHEN** new articles are collected and an earlier rendered article is deleted before a continuation request
- **THEN** surviving matching articles with unchanged ordering keys are neither skipped nor rendered twice and new higher IDs wait for explicit refresh

### Requirement: Authenticated scoped continuation
Tokens SHALL be bounded, versioned, expiring and authenticated, and bind all normalized filters, order, shared timezone and relevant source membership. Tampering, mixed cursor/offset, mismatched scope and invalid tokens SHALL fail before returning data or changing state.
#### Scenario: Token alteration or scope change
- **WHEN** a cursor is modified, expired or reused after changing search, order, timezone or relevant folder membership
- **THEN** no broader batch is returned and the UI offers explicit recovery while retaining loaded cards

### Requirement: Recoverable progressive UI
The UI SHALL retain loaded cards, selection and reading position after continuation failure, expose an explicit keyboard-operable load/retry action, and continue without IntersectionObserver. A failed batch SHALL NOT trigger an automatic retry loop. Changing scope SHALL invalidate pending old responses.
#### Scenario: Failed append without observer
- **WHEN** the observer is unavailable or a later batch fails and the reader activates retry
- **THEN** the next batch appends once without resetting the first batch, losing selection or creating a visit

### Requirement: Timezone-bound mutations
The browser SHALL submit the expected shared display timezone for scoped all-read and selected-state actions. A mismatch SHALL reject the entire mutation and require explicit recovery rather than silently applying another calendar range.
#### Scenario: Another browser changes timezone
- **WHEN** a date-filtered tab submits a bulk action after the shared timezone changes
- **THEN** no article changes until the settings and scope are refreshed and the user explicitly retries
