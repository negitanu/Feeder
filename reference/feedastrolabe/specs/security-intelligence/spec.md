# security-intelligence Specification

## Purpose
Keep independent attributed vulnerability observations and reviewed applicability, using explicitly opted-in bounded public connectors.
## Requirements
### Requirement: Attributed vulnerability cards
The system SHALL display dated KEV/EPSS/severity/remediation observations with source/version and stale/unknown states.

#### Scenario: Attributed vulnerability cards acceptance
- **WHEN** EPSS retrieval fails after a successful prior observation
- **THEN** the prior score remains visible as stale and missing data is not shown as zero

### Requirement: Qualified applicability
The system SHALL label watched product/version applicability as confirmed, possible or unknown with supporting evidence.

#### Scenario: Qualified applicability acceptance
- **WHEN** only a product name matches an advisory
- **THEN** the result is possible or unknown rather than a confirmed affected asset

### Requirement: Bounded connectors
The system SHALL enforce provider endpoint/input/rate/credential policies and record provenance for read-only enrichment.

#### Scenario: Bounded connectors acceptance
- **WHEN** a provider is rate-limited or returns a private redirect
- **THEN** the job waits or fails safely without bypassing limits or leaking credentials

#### Scenario: Independent signals and immutable provenance
- **WHEN** KEV membership, EPSS probability and multiple CNA/ADP CVSS metrics are observed
- **THEN** their dates, source versions and fixed evidence remain separate and are not combined into an unexplained confidence score

#### Scenario: Explicit public identifier consent
- **WHEN** an integration is disabled or an identifier has not been explicitly declared public
- **THEN** no request is sent and private notes, products, versions and attachments remain local

#### Scenario: Reviewed applicability revisions
- **WHEN** a person confirms applicability with a quotation from the same workspace or attempts a stale or cross-workspace edit
- **THEN** accepted assessments retain prior versions and invalid edits leave records unchanged
