## ADDED Requirements

### Requirement: Preview reader settings before import
The settings hub SHALL accept a bounded UTF-8 feeder-settings version 1 JSON file, validate both reader values, and show current and proposed values before an explicit apply operation. It SHALL state that other sections are not applied. Invalid files and cancellation SHALL cause no writes. Applying SHALL use the existing protected reader settings API.

#### Scenario: Review and apply
- **WHEN** a valid exported settings file is selected
- **THEN** current and proposed reader settings are shown and no setting changes until the reader applies them

#### Scenario: Reject invalid input
- **WHEN** a file exceeds 1 MiB or has malformed JSON, an unsupported format/version, or invalid reader values
- **THEN** an error is shown and no settings are written
