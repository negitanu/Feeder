## ADDED Requirements
### Requirement: Single Compose configuration
Feeder SHALL support local operation and optional Cloudflare Tunnel operation using compose.yml and .env, while limiting the Web host port to loopback and protecting connector privileges.
#### Scenario: Tunnel enabled
- **WHEN** the tunnel profile and valid public settings are supplied
- **THEN** Compose configures the connector and Web in tunnel mode without publishing a non-loopback host port.
#### Scenario: Local operation
- **WHEN** the tunnel profile is not enabled
- **THEN** the connector is omitted and Web remains accessible on loopback.
