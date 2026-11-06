## Purpose

Open safe original article URLs without unintended reading-state changes.

## Requirements

### Requirement: Open the original without marking read
The reader SHALL allow a middle click on a card with a valid HTTP(S) original to open it separately without selecting the article, changing read/star state or recording a visit.
#### Scenario: Middle click a card
- **WHEN** a reader middle-clicks an initial or appended card with a valid original URL
- **THEN** a new isolated tab opens and the reader state and visit history remain unchanged
#### Scenario: Unsafe original
- **WHEN** an article has an empty, malformed or non-HTTP(S) original URL
- **THEN** middle-click opens no tab and causes no mutation

### Requirement: Linked detail title
The article detail title SHALL link to a valid HTTP(S) original with `noopener noreferrer`; absent or unsafe originals SHALL remain plain text.
#### Scenario: Keyboard access
- **WHEN** the reader activates the linked title
- **THEN** the original opens in a separate tab without exposing the opener
