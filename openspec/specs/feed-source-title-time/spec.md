## Purpose
Make article lists easy to scan by showing the subscribed Feed, title and absolute publication time in the configured timezone.

## Requirements
### Requirement: Article row source, title, and absolute time
The Reader SHALL present each article's source, title, and absolute publication time in that order.
#### Scenario: Feed title is available
- **WHEN** an article row has a non-empty feed title and a valid publication timestamp
- **THEN** the row shows the feed title, article title, and a full timestamp formatted in the selected timezone
#### Scenario: SQLite UTC timestamp
- **WHEN** a timestamp uses SQLite’s `YYYY-MM-DD HH:mm:ss` format
- **THEN** it is interpreted as UTC regardless of the browser timezone
#### Scenario: Feed title is unavailable
- **WHEN** an article has no feed title
- **THEN** the row uses the URL hostname as its source label
#### Scenario: Publication time is unavailable
- **WHEN** the publication timestamp is absent or invalid
- **THEN** the row displays an explicit unknown-date label
#### Scenario: Compact line layout
- **WHEN** the Reader uses line layout on a desktop viewport
- **THEN** source and time occupy bounded columns, title receives the flexible space, and text truncates without horizontal overflow
#### Scenario: Narrow viewport
- **WHEN** the Reader is displayed on a narrow viewport
- **THEN** source, title, and time can wrap while remaining readable at 11px or larger
#### Scenario: Excerpt layouts
- **WHEN** the Reader uses standard or tile layout
- **THEN** the excerpt remains available after the source/title/time content
