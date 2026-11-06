## Why
Manage Cloudflare Tunnel and runtime credentials using one Compose file and .env.
## What Changes
Use an optional tunnel profile, loopback-only Web port, and environment-based connector token. Remove tunnel override files.
## Capabilities
### New Capabilities
- `unified-tunnel-compose`: Single-file local and tunnel configuration.
### Modified Capabilities
## Impact
Compose, runtime documentation and configuration validation. Existing data volumes remain in place. Cloudflare Access remains required.
