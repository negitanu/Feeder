# ai-research-tracking Specification

## Purpose
Track immutable public paper and release versions, sourced model claims and qualified evaluations.
## Requirements
### Requirement: Versioned research tracking
The system SHALL collect versioned paper and release records incrementally and connect reviewed paper/model/repository references.

#### Scenario: Versioned research tracking acceptance
- **WHEN** a paper revision or repository release appears
- **THEN** a new version is added without overwriting evidence for the previous version

### Requirement: Attributed model cards
The system SHALL retain sourced license/use/hardware/model claims and explicit unknown states.

#### Scenario: Attributed model cards acceptance
- **WHEN** no license statement is found
- **THEN** the card shows unknown rather than assuming permissive use

### Requirement: Qualified evaluations
The system SHALL separate reported benchmarks from user reproductions and disclose differing evaluation conditions.

#### Scenario: Qualified evaluations acceptance
- **WHEN** two results use different dataset versions or harness settings
- **THEN** the comparison marks them incompatible or qualified instead of ranking them as equivalent

#### Scenario: Retry-safe bounded public collection
- **WHEN** a page fails or collection is retried
- **THEN** the same cursor remains until records are committed, previous evidence survives, and missing or capped coverage is disclosed

#### Scenario: Same-case source review
- **WHEN** a research link or model claim is saved
- **THEN** explicit review and a valid fixed quotation in that case are required, without following the link or executing artifacts

#### Scenario: Missing evaluation conditions
- **WHEN** an evaluation omits dataset version or harness settings
- **THEN** comparisons disclose unknown conditions and do not rank it as an equivalent measurement
