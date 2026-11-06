## Purpose

Provide the implemented sidebar-organization capability from reader-platform-provider-refresh.

## Requirements

### Requirement: Persistent folder and feed direct ordering
The sidebar SHALL support dragging folders and individual feeds into any position, with insertion feedback. Folder order and feed order within folders SHALL be stored in the shared DB and persist across reloads and devices. Sorting SHALL NOT navigate or change folder disclosure.

#### Scenario: Reorder either kind
- **WHEN** a folder or a feed is dragged before or after another item of the same kind
- **THEN** the resulting order is saved and restored on reload and in another browser session

#### Scenario: Existing library and new subscriptions
- **WHEN** an existing library without explicit feed positions is migrated, or a new feed is added
- **THEN** existing visible order is preserved and new feeds append to their destination folder

#### Scenario: Cancel or failed write
- **WHEN** a drag is cancelled, interrupted, conflicts with changed membership or fails to save
- **THEN** the UI restores the prior authoritative order and announces the error without silently losing memberships

### Requirement: Move feeds between folders
Dropping a feed into another folder SHALL atomically update its membership and both folders' order. A drop onto a collapsed folder SHALL append the feed without expanding that folder. Article IDs, read/star state and collection settings SHALL remain unchanged.

#### Scenario: Cross-folder drop
- **WHEN** a user drops a feed into another folder or between its feeds
- **THEN** membership, insertion position and sidebar counts reflect the committed result with no duplicate feed entry

#### Scenario: Empty unclassified identity and rename
- **WHEN** a feed moves to/from the empty-name folder or its folder is renamed
- **THEN** exact folder identity and feed order remain consistent and a folder literally named 未分類 remains distinct from the empty-name folder

### Requirement: Touch and keyboard parity
Reordering SHALL support pointer/touch handles and keyboard Up/Down/Home/End. A feed's context menu SHALL provide folder movement so cross-folder moves do not require a pointer. Touch scrolling outside drag handles SHALL remain available.

#### Scenario: Keyboard and touch manipulation
- **WHEN** an item is moved by keyboard or touch and subsequently selected
- **THEN** its saved order is correct, focus is retained, and selection opens the intended articles without unintended disclosure changes

### Requirement: Resizable sidebar
The desktop sidebar SHALL expose a visible drag separator with separator semantics and keyboard adjustment. Width SHALL be clamped to the viewport and persisted per browser. Mobile SHALL use the responsive full-width navigation rather than a constrained desktop width.

#### Scenario: Resize and reload
- **WHEN** the separator is dragged or adjusted with left/right arrows and the app reloads
- **THEN** the stored width is restored while leaving enough width for the article area

#### Scenario: Small viewport or denied storage
- **WHEN** the viewport shrinks, a stored width is invalid or browser storage is unavailable
- **THEN** safe responsive bounds apply without horizontal page overflow or losing sidebar access
