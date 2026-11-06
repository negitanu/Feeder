## ADDED Requirements
### Requirement: Linux image build
Feeder SHALL build the React UI and Rust service in separate stages and ship a Linux runtime without compiler or Node runtime dependencies.
#### Scenario: Clean build
- **WHEN** the image builds from tracked source and lockfiles
- **THEN** its Web process serves the compiled UI and API
### Requirement: Protected persistent deployment
Compose SHALL run Web and worker as non-root with read-only root filesystems, dropped capabilities and a dedicated persistent data volume.
#### Scenario: Restart
- **WHEN** services are recreated
- **THEN** subscriptions, article state and settings remain present without changing the legacy deployment
### Requirement: Explicit HTTP boundary
The Web service SHALL validate explicit trusted Host authorities and reject unrelated Origins, and default host publication SHALL be loopback only.
#### Scenario: Forged request
- **WHEN** a request uses an untrusted Host or mutation Origin
- **THEN** it is rejected even when the process binds all container interfaces
### Requirement: Observable orderly lifecycle
Compose SHALL check Web health before starting worker and allow graceful shutdown within the configured stop period.
#### Scenario: Stop and resume
- **WHEN** services receive termination and restart
- **THEN** they stop starting work, finish bounded in-flight operations and reopen the preserved database successfully
