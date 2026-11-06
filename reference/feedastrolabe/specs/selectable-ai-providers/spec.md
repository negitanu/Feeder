## Purpose

Provide the implemented selectable-ai-providers capability from reader-platform-provider-refresh.

## Requirements

### Requirement: One provider selection interface
The former OpenRouter-specific settings entry SHALL become one AI接続 screen with a provider selector including OpenRouter and OrcaRouter. Model settings, credential availability and relevant options SHALL reflect the selected provider. Settings for each provider SHALL be retained independently, and saving settings SHALL NOT send prompts or run generation.

#### Scenario: Switch and return
- **WHEN** a user configures OrcaRouter and later switches back to OpenRouter
- **THEN** each provider's saved model/options are restored and no credential or request is silently routed to the other provider

#### Scenario: Unconfigured key or unsupported capability
- **WHEN** the selected provider has no credential or the selected model cannot meet the extraction requirements
- **THEN** settings remain editable, outbound generation is prevented with a concrete reason, and the app does not silently use another provider/model

### Requirement: Provider-specific request and result contracts
The application SHALL use separate provider adapters behind a common generation interface. Each adapter SHALL validate structured results, usage, cost and lineage according to that service's documented contract. HTTPS destinations and authentication SHALL be bound to the chosen provider; provider-only fields SHALL NOT leak into another adapter's payload.

#### Scenario: OrcaRouter extraction
- **WHEN** an OrcaRouter extraction runs with a supported model
- **THEN** its authenticated request uses the OrcaRouter endpoint and its result is normalized and schema-validated before being shown

#### Scenario: Invalid or incomplete response
- **WHEN** a provider returns invalid JSON, incomplete output, missing usage, a refusal or unsupported model behavior
- **THEN** no unvalidated result is published and the error is attributed to the correct provider

### Requirement: Preserve budgets and provider lineage
Every queued job SHALL store immutable provider identity, model and configuration revision. Daily application budgets SHALL cover all providers together and SHALL NOT reset on a provider switch. Unknown cost SHALL NOT be treated as zero. Possibly charged generation POSTs SHALL NOT be automatically replayed.

#### Scenario: Switch with queued work
- **WHEN** the active provider changes while jobs are queued
- **THEN** prior jobs remain bound to their original provider and credential reference or fail explicitly if unavailable

#### Scenario: Missing or delayed settlement
- **WHEN** immediate cost is absent or settlement is pending
- **THEN** the reservation remains accounted for, bounded read-only cost lookup may reconcile it, and generation is not resent to obtain a receipt

#### Scenario: Cross-provider budget
- **WHEN** calls consume budget under OpenRouter and the user switches to OrcaRouter
- **THEN** the consumed daily total remains charged against the shared limit

### Requirement: Compatible OpenRouter migration
Existing OpenRouter configuration and queued/history records SHALL migrate with provider=openrouter, preserving models, budgets and lineage. Credentials SHALL remain external to the settings DB, export and browser.

#### Scenario: Upgrade existing installation
- **WHEN** the DB is upgraded
- **THEN** the active provider remains OpenRouter and an old OpenRouter settings URL reaches the unified screen with OpenRouter selected
