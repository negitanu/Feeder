## Purpose

Provide the implemented configuration-export capability from reader-platform-provider-refresh.

## Requirements

### Requirement: Versioned configuration download
The settings hub SHALL offer 設定をエクスポート without a new feature page. The download SHALL be UTF-8 JSON with format version, app/schema version, export time and explicit configuration sections. It SHALL include global collection/display settings, subscriptions and source-specific options, folder/feed order, selected AI provider and its non-secret settings, connector/model/notification/backup settings and current browser presentation preferences.

#### Scenario: Export configuration
- **WHEN** a user chooses 設定をエクスポート
- **THEN** a JSON attachment is downloaded and a consistent DB snapshot contains the current settings without starting collection or generation

#### Scenario: Browser-local preferences
- **WHEN** width, appearance, article layout or disclosure state is stored in the current browser
- **THEN** validated values from that browser are included as presentation settings, distinct from shared DB configuration

### Requirement: Explicit allowlist and secret exclusion
Export serialization SHALL use an explicit per-section field allowlist. It SHALL omit API keys, token values, session/signing material, credential files/contents, authentication headers and embedded URL credentials. It SHALL exclude article bodies, research content, generation payloads, logs, queued jobs and runtime usage counters. Secret requirements MAY be represented by non-secret configured/unconfigured metadata only.

#### Scenario: Secret-bearing installation
- **WHEN** credentials exist in environment variables, DB-backed profiles, private files or configured endpoints
- **THEN** their values do not appear in the downloaded bytes and secret locations are not traversed or copied

#### Scenario: Configuration references
- **WHEN** a watch or profile references a workspace, entity or secret-backed connection
- **THEN** the exported format records an explicit non-secret reference or omitted-field marker without copying the referenced research content or pretending a complete restore is available

### Requirement: Read-only authorized export
The browser export action SHALL require the current access/session and valid CSRF when receiving browser preferences. It SHALL validate submitted presentation data, use no-store and attachment headers and SHALL NOT mutate configuration or issue credentials. This change SHALL provide export only, with import/restoration treated as a separate feature.

#### Scenario: Invalid export request
- **WHEN** access, CSRF or submitted presentation values are invalid
- **THEN** the request fails without a partial attachment or DB mutation
