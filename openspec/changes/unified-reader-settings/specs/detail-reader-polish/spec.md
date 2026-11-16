## REMOVED Requirements


## MODIFIED Requirements

### Requirement: Detail content order
Detail SHALL present an available hero, title, timestamp in the configured timezone, separator, description, separator and navigation. Missing images SHALL not leave a broken image or reserved gap.
#### Scenario: Read an article
- **WHEN** an article opens
- **THEN** the requested sequence is respected and the hero image fits within the available reading column on desktop and mobile

### Requirement: Smooth recoverable dismissal
Detail SHALL provide accessible dismissal and short enter and exit transitions where applicable, disabled with reduced motion. Exiting content SHALL immediately stop accepting input and stale responses SHALL not reopen it. Desktop split-pane outside actions SHALL NOT dismiss detail; outside actions below 1024px SHALL preserve the clicked action when dismissing.
#### Scenario: Switch articles
- **WHEN** the user selects a different article outside the panel
- **THEN** the new article opens without a deferred close dismissing it.
