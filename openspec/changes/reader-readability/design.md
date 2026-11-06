## Context
The Web reader renders article content in `.article-text` inside `.article-body`. Existing typography is fixed in the shared stylesheet. Preferences are personal to the browser and do not belong in the shared Rust reader settings API.

## Decisions
- Store a small versionless JSON object under a dedicated `feeder-readability` localStorage key. Parse each property independently, clamp finite numeric values to supported bounds, and fall back to defaults for malformed data.
- Export `applyReadability()` as the explicit boot hook. It checks for `window`/`document`, reads storage inside a try/catch, applies defaults even if storage is denied, and writes CSS custom properties on `document.documentElement`.
- The no-props React panel initializes from the same sanitized browser state. Updates apply immediately and save locally. A live status message reports saved state or storage denial. Reset returns all three controls to defaults.
- Use native range inputs with visible value output and explicit Japanese labels. Provide keyboard-operable controls and `role="status"` feedback.
- Scope typography to `.article-text`; constrain `.article-body` with `max-width: min(100%, var(--readability-max-width, 70ch))` and `min-width: 0` so the preference cannot exceed its container.

## Risks
- The parent must call `applyReadability()` during boot for preferences to affect the first rendered article; this task deliberately does not edit the parent application.
- Browser privacy settings may deny localStorage. The controls remain functional for the current page session, with a status message explaining persistence is unavailable.
