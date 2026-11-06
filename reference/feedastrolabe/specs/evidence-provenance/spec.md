# evidence-provenance Specification

## Purpose
Preserve immutable captured evidence, attributed source metadata, verified quotation anchors and bounded attachments with consistent blob-aware recovery.
## Requirements
### Requirement: Immutable evidence
The system SHALL preserve versioned captured representations with explicit capture method, provenance timestamps and hashes without claiming authenticity.

#### Scenario: Immutable evidence acceptance
- **WHEN** the upstream article changes after a quotation is saved
- **THEN** the quotation still resolves to its unchanged original evidence version

### Requirement: Independent retention
The system SHALL preserve pinned evidence when its source subscription or ordinary page article is deleted and enforce blob quota/retention.

#### Scenario: Independent retention acceptance
- **WHEN** a subscription with pinned evidence is deleted
- **THEN** source references become detached but pinned evidence and quotes remain readable

### Requirement: Safe attachment lifecycle
The system SHALL validate bounded attachments, isolate parsing, require authenticated access and verify DB/blob backup integrity.

#### Scenario: Safe attachment lifecycle acceptance
- **WHEN** an attachment exceeds quota or a restore is missing a referenced blob
- **THEN** the operation fails explicitly without publishing unsafe content or accepting an incomplete restore

#### Scenario: Bounded isolated parsing
- **WHEN** a hostile, encrypted, malformed or oversized PDF is uploaded
- **THEN** parsing runs without DB/credentials/network in the container, is time/memory/output bounded and fails without publishing evidence

#### Scenario: Capture and retention
- **WHEN** a feed/page changes or unpinned evidence expires
- **THEN** changed captured text creates a new immutable version, identical retries do not duplicate it, pinned versions remain and quota failures are visible without losing ordinary articles

#### Scenario: Atomic bundle restore
- **WHEN** a bundle has a wrong hash, traversal path, symlink or absent blob
- **THEN** restore rejects it before changing live DB/files and valid bundle restoration preserves source, workspace, note and quotation state
