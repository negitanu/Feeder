## Why

Feeder has internal browser APIs but not the legacy scoped public API or key lifecycle. Browser mutations currently rely on Origin alone. Migrate these boundaries before offering a reviewed public-container deployment configuration.

## What Changes

- Add bounded browser sessions and CSRF verification on internal API writes.
- Add hashed, expiring, revocable read/write API keys with persistent per-key rate limits.
- Add the implemented RSS/article subset of `/api/v1`, retaining ID pagination and envelope contracts.
- Add local and externally protected tunnel access modes with fixed trusted origin and secure cookies.
- Provide key management within the settings hub and an unpublished-origin tunnel Compose override.

## Capabilities

### New Capabilities

- `api-access-control`: Browser CSRF, scoped public API keys and container access boundary.

### Modified Capabilities

None. Existing internal handler contracts remain, with production middleware enforcing CSRF.

## Impact

Schema v7, Rust access middleware/API routes, React session-aware requests/key UI, Linux Compose acceptance and deployment documentation. No actual external deployment, credentials or legacy data are accessed. SNS/research APIs and optional application-side Cloudflare JWT verification remain future work.
