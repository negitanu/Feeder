## Purpose

Provide consistent reader defaults, browser presentation and a read-only allowlisted configuration download.

## Requirements

### Requirement: Persisted reader defaults
Feeder SHALL persist a shared default view of all/unread/starred and display timezone UTC/Asia-Tokyo. Unknown fields and invalid values SHALL be rejected without writes. A recognized v5 database SHALL be backed up and migrated without changing article IDs, state, history, source order or collection settings.

#### Scenario: Saved defaults and migration
- **WHEN** the database is upgraded and a reader saves Japanese time and unread defaults
- **THEN** the pre-upgrade backup remains valid and a new browser starts with those defaults while all existing data is preserved

### Requirement: Browser article layouts
The reader SHALL provide standard, single-line and tile article layouts stored per browser. Changing layout SHALL retain loaded articles, filters, selected IDs and reading state, with responsive and keyboard-accessible controls in every theme. Storage failure or invalid stored values SHALL fall back safely.

#### Scenario: Layout and detail round trip
- **WHEN** a reader changes layout with selected appended articles, opens and closes a detail and reloads
- **THEN** loaded state is retained during navigation and the chosen layout is restored on reload

### Requirement: Versioned allowlisted export
The settings hub SHALL offer a read-only UTF-8 JSON settings attachment containing format/app/schema versions, timestamp, implemented shared configuration, ordered sources, validated current browser presentation and explicit omissions/unsupported sections. Export SHALL use a consistent read snapshot, no-store headers and same-origin validation and SHALL NOT start collection, mutate DB state or read secret stores.

#### Scenario: Current configuration download
- **WHEN** a reader exports configuration after changing source order and browser layout
- **THEN** the JSON attachment includes that order and presentation with no article/research content, runtime counters, errors or credentials

#### Scenario: Sensitive values and invalid presentation
- **WHEN** credentials occur in source URL userinfo/query/fragment or recognizable credential-bearing text, or presentation is invalid
- **THEN** sensitive values are omitted with field-only markers, or invalid requests fail without an attachment or database changes
