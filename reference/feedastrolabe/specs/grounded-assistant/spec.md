# grounded-assistant Specification

## Purpose
Provide explicitly approved, workspace-scoped research assistance with immutable citations, bounded model use and independent human review.
## Requirements
### Requirement: Grounded assistance
The system SHALL provide opt-in evidence-scoped summaries/translations/answers with validated citations and insufficient-evidence responses.

#### Scenario: Grounded assistance acceptance
- **WHEN** the selected workspace has no evidence supporting a question
- **THEN** the assistant reports insufficient evidence without inventing a source

### Requirement: Controlled external processing
The system SHALL enforce configured local/remote profile, classification/secret checks and explicit selected-input preview for remote jobs.

#### Scenario: Controlled external processing acceptance
- **WHEN** restricted research text is selected for a remote model
- **THEN** submission is blocked by default and no external request is sent

### Requirement: Safe reproducible jobs
The system SHALL record model/prompt/evidence lineage and enforce budgets without executing instructions from evidence.

#### Scenario: Safe reproducible jobs acceptance
- **WHEN** a retrieved article contains instructions to read secrets or change settings
- **THEN** those instructions are treated as data and no external action or configuration mutation occurs

#### Scenario: Exact-preview and stale policy
- **WHEN** approval hash or profile/classification changes after preview
- **THEN** the job is rejected or failed before transmitting selected text

#### Scenario: Citation validation and human revisions
- **WHEN** output refers to an unselected source or a reviewer edits the answer
- **THEN** invalid model output is rejected and human edits remain independent revisions without modifying source evidence

#### Scenario: Interrupted paid request
- **WHEN** generation is interrupted or usage is uncertain
- **THEN** no automatic retry occurs and the reserved budget remains charged conservatively until a new explicit operation
