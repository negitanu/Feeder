## Purpose

Apply explicit selected-article actions with consistent accessible controls.

## Requirements
### Requirement: Consistent selected-article dropdown
Selected articles SHALL expose a labelled rounded action select, a separate execute button and a clear-selection action, using the current theme and keyboard focus styles.
#### Scenario: Choose an action
- **WHEN** a reader selects an action
- **THEN** no article is modified until Execute is activated.
#### Scenario: Pending or failed action
- **WHEN** execution is pending or fails
- **THEN** duplicate execution is blocked and failed choices remain available for retry.
