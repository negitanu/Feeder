## ADDED Requirements
### Requirement: Detail content order
Detail SHALL present an available hero, title, JST timestamp including seconds, separator, description, separator and navigation in that order. Missing images SHALL not leave a broken image or reserved gap.
#### Scenario: Read an article
- **WHEN** an article opens
- **THEN** the requested sequence and fixed 480px maximum image width are respected on desktop and mobile.
### Requirement: Smooth recoverable dismissal
Outside primary clicks SHALL close detail while preserving the clicked action. Enter and exit SHALL use short opacity/translation transitions, disabled with reduced motion. Exiting content SHALL immediately stop accepting input and stale responses SHALL not reopen it.
#### Scenario: Switch articles
- **WHEN** the user selects a different article outside the panel
- **THEN** the new article opens without a deferred close dismissing it.
