## Purpose

Keep the reader list available while viewing an article in a responsive floating panel.

## Requirements

### Requirement: Retained floating reader detail
The reader SHALL display article detail in an independently scrollable responsive panel without hiding or resizing the loaded list.
#### Scenario: Open and close detail
- **WHEN** a reader opens and closes an article on desktop or mobile
- **THEN** the list, selection and current scroll position remain available and the panel stays below the visible header and within the viewport

### Requirement: Non-blocking dismissal and stale response protection
The reader SHALL close detail on Escape or a primary pointer or keyboard-activated action outside it, preserving the outside action and preventing obsolete responses from reopening it.
#### Scenario: Close while loading
- **WHEN** the user dismisses a pending detail and its visit request later succeeds
- **THEN** the panel stays closed and the successful read state is reflected in the retained list
#### Scenario: Switch quickly
- **WHEN** another article is selected before an earlier request finishes
- **THEN** only the latest requested article is shown, regardless of response order
