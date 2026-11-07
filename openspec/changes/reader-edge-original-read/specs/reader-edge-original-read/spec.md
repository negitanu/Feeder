## ADDED Requirements
### Requirement: Edge-aligned detail
The article detail SHALL align its right boundary with the page viewport on desktop and mobile.
#### Scenario: Wide viewport
- **WHEN** detail opens on a wide display
- **THEN** no centered-container gutter separates it from the page's right edge.
### Requirement: Middle-click marks read
A safe middle-click original open SHALL mark the article read without opening detail or recording a detail visit. Pending duplicate mutations SHALL be coalesced.
#### Scenario: Open an unread article
- **WHEN** the user middle-clicks a valid original
- **THEN** its isolated tab opens and successful read changes reconcile cards, unread membership and counts.
#### Scenario: Unsafe URL
- **WHEN** the original is invalid or non-HTTP(S)
- **THEN** no tab or mutation occurs.
