# Reader detail arrow keys

## Purpose

Provide scoped keyboard navigation for loaded article details.

## Requirements

### Requirement: Navigate details with scoped arrow keys
The reader SHALL allow unmodified left and right arrow keys to activate the existing previous and next article navigation while focus is inside an open detail. It SHALL disclose the shortcut and preserve the captured loaded sequence and existing visit semantics.

#### Scenario: Navigate with a keyboard
- **WHEN** focus is inside a loaded detail and the reader presses right or left
- **THEN** the next or previous captured article opens through the normal visit operation, if available

#### Scenario: Respect other keyboard interactions
- **WHEN** a key is modified, repeated, composing, already handled, occurs during loading or outside the detail, or targets an editable or arrow-operated control
- **THEN** it does not trigger article navigation

#### Scenario: Boundary
- **WHEN** the user presses an arrow with no article in that direction
- **THEN** no visit is requested and the key is not consumed
