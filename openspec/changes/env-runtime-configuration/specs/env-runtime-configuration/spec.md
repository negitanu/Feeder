## ADDED Requirements

### Requirement: Compose environment configuration
Feeder SHALL support a project `.env` and an explicitly selected Compose env file for runtime settings and SHALL preserve Compose shell-variable precedence.
#### Scenario: Configure a disposable environment
- **WHEN** an operator sets the port and optional provider credentials in a selected env file
- **THEN** Compose applies the port and passes the credentials to Web and worker without embedding them in the image or browser assets

### Requirement: Tunnel token sources
Feeder SHALL support a `.env` `TUNNEL_TOKEN` configuration and retain the existing secret-file configuration, with either mode keeping the Web host port unpublished.
#### Scenario: Start using a configured token
- **WHEN** the env-based tunnel override is selected with an explicit token and public origin
- **THEN** Compose passes the token to the connector environment, not its command, and does not publish the Web origin
#### Scenario: Missing token
- **WHEN** the env-based tunnel override is selected without a token
- **THEN** Compose refuses configuration before starting the connector

### Requirement: Secret boundaries and incomplete providers
Feeder SHALL keep env credentials out of source control, build context, browser responses, database and settings exports and SHALL document which provider features remain unimplemented.
#### Scenario: Credentials configured before provider support
- **WHEN** provider keys are configured in `.env`
- **THEN** no AI capability is claimed or enabled and their values are absent from rendered UI and settings exports

### Requirement: Existing API key lifecycle
Feeder SHALL retain the existing browser-issued API bearer key lifecycle and SHALL NOT silently turn provider credentials into Feeder API access keys.
#### Scenario: Configure a provider key
- **WHEN** only a provider credential is configured
- **THEN** it cannot authenticate to the Feeder public API
