## Purpose

Provide consistent reader detail interactions and application branding.

## Requirements

### Requirement: Dismiss detail without losing the reader
The reader SHALL close an open detail when a primary click or touch click occurs outside it, preserving loaded articles, filter/order, selections and reading position. The original outside action SHALL still work. Clicking another article SHALL switch to that article. Inside actions SHALL retain the detail. Closing SHALL remove only article and its detail fragment from the URL, work with browser history, and prevent stale loading or mutation responses from reopening it.

#### Scenario: Click outside while retaining state
- **WHEN** a user selects an article checkbox or clicks the heading outside an open detail
- **THEN** the detail closes, the checkbox action completes, and the retained list and filters remain unchanged

#### Scenario: Continue reading or saving
- **WHEN** a user clicks inside the detail or selects a different article
- **THEN** the inside action works or the new article opens without an outside-close race

#### Scenario: Close during loading or saving
- **WHEN** an older detail or save response arrives after the user closed the detail
- **THEN** it does not reopen the closed panel, while successful state changes remain reflected

### Requirement: Smooth accessible detail transitions
The reader SHALL animate detail opening and closing with a short opacity and translation transition, including initial detail load and history navigation. Closing content SHALL immediately stop accepting input. Reduced-motion preference SHALL disable the transition. Existing close links SHALL remain usable without JavaScript.

#### Scenario: Open and close with motion preferences
- **WHEN** a user opens and closes a detail with normal or reduced motion
- **THEN** normal motion uses a short transition and reduced motion switches immediately without leaving a blocking panel

### Requirement: Consistent selected action controls
The selected-article action select SHALL share the border radius, theme colors and visible keyboard focus of reader selects without overflowing mobile or desktop layouts.

#### Scenario: Selected action in each theme
- **WHEN** the reader is shown on mobile or desktop in light, dark or system appearance
- **THEN** the selected action uses the same rounded corners, theme colors and keyboard focus as reader selects

### Requirement: Shared brand favicon
A single eight-ray brand SVG SHALL be used in the header and as the application favicon across pages.

#### Scenario: Load a branded page
- **WHEN** a user opens a reader, collection or settings page
- **THEN** its favicon resolves to the same mark as the header
