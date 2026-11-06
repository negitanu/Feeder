# watchlists-triage Specification

## Purpose
Provide repeatable workspace triage with bounded literal queries, immutable rule revisions, idempotent explained matches and reversible exact duplicate groups that retain provenance.
## Requirements
### Requirement: Repeatable triage
The system SHALL persist workspace saved searches and bounded include/exclude/priority rules with matching explanations.

#### Scenario: Repeatable triage acceptance
- **WHEN** an article matches a product rule and an exclusion rule
- **THEN** the displayed triage result explains both and retains the original article

#### Scenario: Bounded literal query and immutable revision
- **WHEN** a client submits excessive or invalid AST conditions or edits a stale expected revision
- **THEN** invalid input is rejected atomically, a stale edit returns conflict, and valid edits preserve prior rule revisions

#### Scenario: Scoped evidence and archived workspace
- **WHEN** rules run for a workspace or that workspace is archived
- **THEN** only its associated evidence is evaluated and an archived workspace pauses automatic and explicit matching while retaining readable history

### Requirement: Idempotent matching
The system SHALL create durable in-app watch events once per rule/evidence version and allow explicit reprocessing.

#### Scenario: Idempotent matching acceptance
- **WHEN** a worker retries the same evidence version
- **THEN** no duplicate watch event or digest item is created

#### Scenario: Bounded worker and stable cursor
- **WHEN** more than twenty enabled rules or one hundred evidence versions await processing, including after unpinned evidence retention
- **THEN** bounded fair batches continue without losing new versions through identifier reuse and preserved event confirmation state is independent of article read/star flags

### Requirement: Provenance-preserving groups
The system SHALL group exact duplicates while preserving their sources, evidence and read/star state.

#### Scenario: Provenance-preserving groups acceptance
- **WHEN** two publishers provide identical content
- **THEN** the group exposes both sources without removing either record

#### Scenario: Undo survives retries
- **WHEN** a user suppresses an exact duplicate group and collection or grouping is retried
- **THEN** the group remains suppressed, every attributed evidence version is retained, and explicit redo is available

#### Scenario: Meaningful URL query is preserved
- **WHEN** URLs differ only in defined tracking parameters or instead in a meaningful query value
- **THEN** only the former share a canonical URL group and a URL match is not asserted to be an exact text match
