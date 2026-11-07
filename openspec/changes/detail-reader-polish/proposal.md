## Why
The requested detail view should prioritize article content and consistently dismiss outside, with smooth accessible transitions.
## What Changes
Order detail content as optional fixed-width hero, title, full JST timestamp, separator, description, separator, previous/count/next. Add enter/leave transitions with reduced-motion support, preserving outside actions and request race protection. Keep shared header/favicon SVG and integrate rounded selected-action dropdown.
## Capabilities
### New Capabilities
- `detail-reader-polish`: Content-first detail interactions.
### Modified Capabilities
## Impact
React detail presentation and browser acceptance. Hero endpoint and bulk controls are independently specified. No article state changes except existing visit and explicit bulk actions.
