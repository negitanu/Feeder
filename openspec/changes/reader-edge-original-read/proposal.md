## Why
The detail panel is inset on wide screens, and opening an original by middle click leaves the article unread.
## What Changes
Anchor detail to the page's right edge on desktop/mobile. Middle click opens the safe original synchronously and marks the article read using the existing state endpoint without opening detail or adding a detail visit.
## Capabilities
### New Capabilities
- `reader-edge-original-read`: Edge-aligned detail and middle-click read state.
### Modified Capabilities
## Impact
React/CSS and browser tests. Supersedes the old reader-original-access no-read behavior. No schema changes.
