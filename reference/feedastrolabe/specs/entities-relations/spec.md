# entities-relations Specification

## Purpose
Keep original source-anchored identifier mentions and immutable quoted assessments, with human review, reversible merges and bounded exploration.
## Requirements
### Requirement: Normalized mentions
The system SHALL retain normalized identifiers and source-linked original mentions while supporting reviewed aliases.

#### Scenario: Normalized mentions acceptance
- **WHEN** equivalent IPv6 spellings appear in two evidence records
- **THEN** one entity shows both original mentions and evidence links

#### Scenario: Human correction survives extraction retry
- **WHEN** a reviewed mention is corrected or rejected and its fixed evidence is reprocessed
- **THEN** its immutable original spelling, source position and normalized identity history remain intact and human review is not overwritten

#### Scenario: Ambiguous names and extraction limits
- **WHEN** organizations share a name or a source exceeds the configured extraction budget
- **THEN** names are not automatically merged, candidates remain unreviewed and the bounded incomplete extraction is explicitly signalled

### Requirement: Evidence-backed relations
The system SHALL store assertions with source anchors, assessment type/confidence and temporal metadata without inferring ownership from shared infrastructure.

#### Scenario: Evidence-backed relations acceptance
- **WHEN** two domains share an observed IP
- **THEN** the relation shows the observation and does not label their owners identical

#### Scenario: Quoted relation and atomic revision validation
- **WHEN** a relation references another workspace quotation, an invalid interval or a stale revision
- **THEN** the edit is rejected atomically, while accepted edits keep prior quoted assertion versions and human confidence is distinguished from calibrated probability

### Requirement: Reversible exploration
The system SHALL support bounded graph/timeline views and audited entity merge/unmerge.

#### Scenario: Reversible exploration acceptance
- **WHEN** a mistaken alias merge is undone
- **THEN** the original entities and their evidence-backed mentions are restored

#### Scenario: Bounded and scoped exploration
- **WHEN** a graph is explored within a workspace or its node/edge budget is exhausted
- **THEN** only current nonarchived quoted workspace assertions are included, truncation is visible and paginated relation/timeline views retain access to omitted records
