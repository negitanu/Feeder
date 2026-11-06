## Why
Readers need browser-local controls for article text size, line spacing, and readable line length without changing shared reader settings.

## What Changes
- Add a standalone Japanese React panel for three bounded article readability preferences and reset.
- Persist preferences in browser localStorage, apply them through document root CSS variables, and tolerate invalid or denied storage.
- Document the behavior and its integration point for the parent application.

## Capabilities
### New Capabilities
- `reader-readability`: Browser-local presentation preferences for article content.
### Modified Capabilities
None. Existing shared reader settings and article persistence remain unchanged.

## Impact
Frontend-only: new `frontend/src/ReadabilityPanel.tsx` and `frontend/src/readability.css`; no backend, shared API, or article data changes. Parent integration imports the default panel and invokes the named `applyReadability()` during browser startup.

## Existing Web Specification and Implementation Mapping
- `openspec/specs/reader-foundation/spec.md` defines the Web reader and article detail; this change is an optional browser presentation layer within that reader.
- `frontend/src/style.css` currently sets `.article-text` to `font-size:14px` and `line-height:2.1`, while `.article-detail` is constrained to 740px. The new stylesheet overrides only article text typography and bounds `.article-body` width using root variables.
- `frontend/src/ReaderSettingsPanel.tsx` already distinguishes browser-local presentation preferences from shared API settings. Readability follows that browser-local model, but uses dedicated storage and startup application.

## Acceptance Criteria
- Font size is bounded from 13 to 22 px, line height from 1.5 to 2.4, and article text width from 40 to 90 ch, including during storage parsing.
- Startup application is browser-only, tolerates inaccessible localStorage and malformed/unknown values, and always leaves usable defaults applied.
- Each labelled input updates the document root variables and attempts to persist locally; storage denial is surfaced accessibly without breaking interaction.
- Reset restores defaults, persists them when possible, and announces the result.
- The panel is usable by keyboard and screen readers; the CSS does not alter unrelated page layout or cause article overflow.
- Implementation remains limited to the two named frontend files, this OpenSpec change directory, and `docs/reader-readability.md`.
