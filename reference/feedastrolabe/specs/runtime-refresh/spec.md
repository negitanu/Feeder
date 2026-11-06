## Purpose

Provide the implemented runtime-refresh capability from reader-platform-provider-refresh.

## Requirements

### Requirement: Latest compatible runtime images
Container sources SHALL follow current stable images with latest as the default tag policy. Explicit Python 3.15 compatibility SHALL take precedence if an unconstrained latest tag moves to another Python series. Usable Docker Hardened Images SHALL be preferred by role; unavailable or incompatible variants SHALL have a documented official-image fallback. Resolved digests SHALL be recorded for verification and rollback, not left as an indefinitely stale source pin.

#### Scenario: Refresh image sources
- **WHEN** a new build is prepared
- **THEN** upstream tags are freshly resolved and the deployed Python runtime is verified to be 3.15.x

#### Scenario: Evaluate hardened images
- **WHEN** a matching hardened image can be pulled and satisfies the runtime/architecture requirements
- **THEN** it is used for that role with its documented build/runtime variants

#### Scenario: Hardened variant cannot be used
- **WHEN** registry access, Python version, system libraries or Chromium dependencies prevent adoption for one role
- **THEN** the reason and official fallback are recorded per role and other compatible roles may still use hardened images

### Requirement: Python 3.15 and current packages
The application, workers, renderer and test environment SHALL use the latest stable Python 3.15 patch available at implementation. Direct/transitive Python dependencies, browser tooling and test dependencies SHALL be updated to current stable releases, resolved into reproducible constraints and checked for compatibility.

#### Scenario: Native dependencies and browser integration
- **WHEN** the updated dependency set is installed on supported Linux architectures
- **THEN** dependency consistency, cryptography, SQLite, process startup and the matched Playwright/Chromium workflow pass on Python 3.15

#### Scenario: A latest package is incompatible
- **WHEN** an upstream stable release is incompatible with Python 3.15 or another required dependency
- **THEN** the exact constraint and reason are recorded and the implementation does not silently downgrade Python, use a prerelease or claim that package was updated successfully

### Requirement: Preserve container behavior and data
Runtime changes SHALL retain persistent volumes, non-root execution, read-only filesystems, dropped capabilities, renderer network isolation and working health checks.

#### Scenario: Rebuild and restart
- **WHEN** the new containers replace the existing services
- **THEN** all services start successfully with their existing data and subscriptions, articles and read/star state remain intact
