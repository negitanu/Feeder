## Why
Start the configured Cloudflare connector with the normal Compose command.
## What Changes
Remove the tunnel profile and update configuration guidance. The connector uses the existing token and protection settings.
## Capabilities
### New Capabilities
- `default-tunnel`: Start the tunnel alongside Web and worker.
### Modified Capabilities
## Impact
Compose configuration, documentation and synthetic configuration checks. An unset token causes the connector to fail; local Web remains available.
