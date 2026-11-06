# research-workspaces Specification

## Purpose
Organize the shared library into research workspaces with versioned safe Markdown notes, shared tags and bounded authenticated operations.
## Requirements
### Requirement: Research organization
The system SHALL create, edit, archive and list workspaces, with explicit purpose/questions/status and many-to-many feed/article associations.

#### Scenario: Research organization acceptance
- **WHEN** an article is linked to two workspaces and one workspace is archived
- **THEN** the article/read/star state and the other association survive

### Requirement: Versioned notes
The system SHALL store revisioned notes with fact/source-claim/hypothesis labels and reject stale writes atomically.

#### Scenario: Versioned notes acceptance
- **WHEN** two clients edit the same note revision
- **THEN** the first edit succeeds and the second returns409 without overwriting it

### Requirement: Scoped workspace access
The system SHALL offer paginated UI/API operations with input bounds, write scope and CSRF for UI mutations, and explicit outbound classification.

#### Scenario: Scoped workspace access acceptance
- **WHEN** a read-only key attempts to modify a workspace or note
- **THEN** 403 is returned and stored state is unchanged

#### Scenario: Archive read-only and feed membership
- **WHEN** a workspace has an associated feed and is archived
- **THEN** current/future articles remain readable, ordinary edits return409, and only explicit unarchive reopens editing

#### Scenario: Safe note rendering and history
- **WHEN** a note contains hostile HTML, unsafe links or remote images and is edited or archived
- **THEN** the UI renders sanitized Markdown without active content/images, retains immutable revisions, and allows conflict recovery without lost submitted text

### Requirement: Shared tags and bounded filtering
The system SHALL create and explicitly rename shared tags, associate them with workspaces/articles/notes, and validate bounded paginated filters without duplicate results.

#### Scenario: Shared tag and workspace pagination
- **WHEN** an article is both directly linked and included by a linked feed and matches a shared tag
- **THEN** workspace/tag search returns it exactly once with stable cursors and rejects nonexistent filter IDs

#### Scenario: Upgrade and backup preservation
- **WHEN** a v4 library is upgraded or a v5 research backup is restored
- **THEN** source/article IDs, read/star flags, workspace links and all note revisions survive with valid foreign keys
