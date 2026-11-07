## ADDED Requirements

### Requirement: Distinct mobile reader layouts
The Reader SHALL preserve distinct standard, single-line and tile layouts on mobile.

#### Scenario: Select line view
- **WHEN** a reader selects line view on a phone
- **THEN** each article's source, title and absolute time are displayed in a single compact row without wrapping

#### Scenario: Select tile view
- **WHEN** a reader selects tile view on a typical phone viewport
- **THEN** articles appear in two columns without horizontal page overflow and controls remain usable

### Requirement: Readable themes
The Reader SHALL offer legible light, dark and system themes while maintaining existing interactions.

#### Scenario: Change theme or viewport
- **WHEN** the reader changes theme or uses a narrow viewport
- **THEN** text and controls remain legible and usable by touch and keyboard
