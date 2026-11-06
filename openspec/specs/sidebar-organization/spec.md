## Purpose

Persist the Web reader library arrangement and provide accessible responsive sidebar sizing.

## Requirements

### Requirement: Persistent ordered library
The system SHALL persist folder order and feed order within each folder and the unfiled group. Migration SHALL preserve current visible order; new sources and reassigned feeds SHALL append. Ordering SHALL preserve article IDs, read/star state, visits and collection settings.

#### Scenario: Migration and append
- **WHEN** a v4 fixture is upgraded and a new source is added
- **THEN** existing visible order is unchanged and the new source is last in its group, with a pre-upgrade backup retained

#### Scenario: Reload from another session
- **WHEN** a reader saves an arrangement and another browser loads the library
- **THEN** both sessions see the saved order and membership

### Requirement: Atomic conflict-checked movement
The server SHALL atomically validate complete expected and desired source snapshots, rejecting stale snapshots, missing or duplicate identities, unknown destinations and hostile origins without partial writes. A cross-folder move SHALL update membership and source/destination order together. Cancelled drags SHALL not save.

#### Scenario: Move and conflict
- **WHEN** a feed is moved into a collapsed folder
- **THEN** it appends without expanding or navigating; a stale competing save returns409 and reloads authoritative state

#### Scenario: Invalid input
- **WHEN** the desired snapshot duplicates a feed or uses an unknown folder
- **THEN** no memberships or positions change

### Requirement: Accessible direct manipulation
The sidebar SHALL provide pointer/touch handles with insertion feedback, keyboard Up/Down/Home/End ordering, and a menu-accessible folder movement action. Sorting SHALL preserve focus, selection and folder disclosure, with errors announced. Touch scrolling outside handles SHALL remain available.

#### Scenario: Keyboard and pointer reorder
- **WHEN** a reader reorders a source by handle or keyboard and reloads
- **THEN** the new position persists without navigating and the moved control retains focus after saving

### Requirement: Responsive resizable sidebar
The desktop sidebar SHALL expose a visible separator with separator semantics, pointer resizing and keyboard adjustment. Width SHALL persist per browser, remain safely bounded and fall back when storage fails. Mobile SHALL retain full-width navigation without page overflow.

#### Scenario: Resize and denied storage
- **WHEN** a reader resizes and reloads, or browser storage is denied or invalid
- **THEN** a valid stored width is restored, otherwise safe defaults apply without blocking the app

#### Scenario: Mobile viewport
- **WHEN** a desktop width exists and viewport becomes mobile
- **THEN** the separator is hidden and navigation uses the available width without horizontal overflow
