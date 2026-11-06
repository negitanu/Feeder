## Why

Readers need saved display defaults and calendar-date filtering consistent with displayed dates. The existing Web settings export must be migrated without including credentials, runtime data or article content.

## What Changes

- Add shared default view and UTC/Asia-Tokyo display timezone with a backed-up v5-to-v6 migration.
- Apply the same timezone to article dates and date-filter boundaries for lists and bulk updates.
- Add standard, line and tile layouts stored per browser without resetting loaded reader state.
- Export a versioned allowlisted JSON snapshot including implemented shared settings, ordered sources and validated browser presentation, with sensitive values omitted.
- Keep configuration export read-only; whole-configuration import and unmigrated service settings remain future work.

## Capabilities

### New Capabilities

- `reader-settings-export`: Shared reader defaults, browser presentation and safe configuration download.

### Modified Capabilities

- `reader-productivity`: Date range interpretation follows shared display timezone.

## Impact

Rust schema, reader query scope and configuration API; React collection/settings hub and article layouts; Linux container and browser acceptance. Existing article state, order, history and collection behavior are preserved. No new runtime dependencies, live-data access or external network tests.
