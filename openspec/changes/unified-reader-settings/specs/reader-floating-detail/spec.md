## REMOVED Requirements

### Requirement: Retained floating reader detail
**Reason**: Replaced by the responsive split-pane requirement.
**Migration**: Use Retained responsive reader detail.

### Requirement: Non-blocking dismissal and stale response protection
**Reason**: Dismissal differs between desktop and smaller widths.
**Migration**: Use Responsive dismissal and stale response protection.

## ADDED Requirements

### Requirement: Retained responsive reader detail
The reader SHALL retain the loaded list and selection while showing an independently scrollable detail; at widths of 1024px and above it SHALL display a resizable split pane, and at smaller widths it SHALL use the responsive detail presentation.
#### Scenario: Open and close detail
- **WHEN** a reader opens and closes an article on desktop or mobile
- **THEN** the list and selection remain available, the list width is restored on desktop, and the detail remains within the viewport

### Requirement: Responsive dismissal and stale response protection
The reader SHALL close detail on Escape; at widths below 1024px it SHALL also dismiss on an outside primary pointer or keyboard-activated action, preserving the outside action. At wider widths outside actions SHALL NOT dismiss the detail. Obsolete responses SHALL NOT reopen dismissed detail.
#### Scenario: Close while loading
- **WHEN** the user dismisses a pending detail and its visit request later succeeds
- **THEN** the panel stays closed and the successful read state is reflected in the retained list
#### Scenario: Switch quickly
- **WHEN** another article is selected before an earlier request finishes
- **THEN** only the latest requested article is shown, regardless of response order
#### Scenario: Outside action at different widths
- **WHEN** an outside action occurs with detail open
- **THEN** detail remains open at desktop split-pane widths and dismisses below 1024px while preserving the outside action where it is available
