## Why
The Linux browser regression found that closing an in-flight article detail also discarded an already committed visit response from the retained list, leaving its read state stale.

## What Changes
Separate detail visibility invalidation from list response scope validation. A committed visit updates the matching retained list while a dismissed or superseded panel stays closed.

## Impact
React Reader only. No database or authentication changes. Existing delayed-visit browser regression supplies acceptance coverage.
