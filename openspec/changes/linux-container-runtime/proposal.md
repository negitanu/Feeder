## Why
Feeder runs on Linux containers; reproducible Web/worker deployment and data persistence are required before further migration.
## What Changes
- Add multi-stage React/Rust Dockerfile and Web/worker Compose services sharing a dedicated volume.
- Preserve non-root, read-only filesystem, dropped capabilities and no-new-privileges.
- Add explicit trusted Host configuration, healthchecks and graceful shutdown.
- Add Linux container verification and operating instructions.
## Capabilities
### New Capabilities
- `linux-container-runtime`: Linux-only container build, protected runtime, persistent Web/worker operation.
### Modified Capabilities
None. Legacy renderer/observer and external authentication/tunnel remain migration gates until implemented.
## Impact
Dockerfile, compose.yml, build context, runtime configuration, shutdown, CI and documentation. Existing FeedAstrolabe deployment and volumes remain untouched.
