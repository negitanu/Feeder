## ADDED Requirements
### Requirement: Standalone pull to refresh
When running as a standalone web app, the reader SHALL allow a downward pull at the document top to reload the current page, provide progress feedback, and preserve normal scrolling and other gestures.
#### Scenario: Pull in standalone mode
- **WHEN** a reader pulls downward beyond the threshold from the top and releases
- **THEN** the current page reloads and its persisted state remains intact.
#### Scenario: Nested scrolling or horizontal gestures
- **WHEN** the reader scrolls a drawer, detail, dialog or scrollable control, or swipes horizontally
- **THEN** the pull gesture does not trigger a reload.
