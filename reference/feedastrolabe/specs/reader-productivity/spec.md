## Purpose

Provide safe discovery and productive daily article reading without changing existing API pagination.

## Requirements

### Requirement: Independent recent reading history
The reader SHALL persist the latest actual detail visit per article using CSRF-protected POST that also marks the displayed article read while preserving star state. A 最近読んだ記事 view SHALL order by latest visit by default and retain all normal filters. Bulk read, star changes, GET/API reads and middle-click original access SHALL NOT create visits. The shared-library history SHALL survive restart and disappear with deleted articles.

#### Scenario: Read an old article again
- **WHEN** an old article is opened and its visit is recorded
- **THEN** it appears first in history with read state saved and star state preserved

#### Scenario: Non-visit state changes
- **WHEN** articles are bulk-marked read or starred without opening them
- **THEN** history remains unchanged

### Requirement: Shared deterministic ordering
The reader SHALL offer 新しい順 and 古い順 for publication date, and 最近読んだ順 for history. Ordering SHALL include article ID as tie-breaker and be shared by continuation, detail neighbours, next unread and explicit read-next. Signed continuation scope SHALL include order and all filters.

#### Scenario: Ascending continuation and next article
- **WHEN** old-first results span multiple batches with equal publication dates
- **THEN** all matching IDs are returned once in ascending date/ID order and detail next uses that order

#### Scenario: Cursor scope mismatch
- **WHEN** a continuation is reused with a different order or detailed condition
- **THEN** the request is rejected without broadening results

### Requirement: Detailed literal search form
The normal reader SHALL expose title/body/both, additional any-of phrases, exclusion phrases, a publication calendar-date range and multiple information sources in a keyboard/touch-operable form. Any-of and exclusion controls SHALL be single-line inputs accepting comma-separated phrases with CSV quoting. Existing newline-based search URLs SHALL retain their meaning. Sources SHALL use a collapsed dropdown containing checkboxes with multiple selections, preserved submitted selections and no-script access. Existing q semantics SHALL remain a single literal phrase. Groups SHALL combine with AND, any-of phrases with OR and exclusions with NOT. Dates SHALL use the library display timezone and exclude unknown original publication times. All request values SHALL be bounded and duplicate/conflicting or invalid parameters rejected. Details, mutations and no-script access SHALL preserve the complete condition.

#### Scenario: Combine conditions
- **WHEN** a user searches body text with any-of phrases, excludes a phrase and selects a date range and two sources
- **THEN** only articles satisfying the complete condition appear, including every continuation batch and bulk read

#### Scenario: Invalid conditions
- **WHEN** a date is invalid or reversed, source is missing, scope is unknown or parameters conflict
- **THEN** no mutation occurs and an explanatory error is shown

#### Scenario: Compact fields and dropdown
- **WHEN** a desktop or mobile user expands detailed search
- **THEN** phrase controls occupy one input line, sources precede search scope in DOM and visual order, all six controls share one row when the advanced search area is at least 600 CSS pixels wide, source choices appear only after opening the dropdown, and selected sources survive search and subsequent reader actions

#### Scenario: Legacy and literal punctuation
- **WHEN** legacy newline parameters or CSV input with quoted commas and quotes are used
- **THEN** the original literal phrases and bounds are preserved and conflicting input forms are rejected

### Requirement: Explicit selected-article bulk state
The reader SHALL allow selecting at most 100 distinct rendered articles and atomically marking them read/unread/starred/unstarred. Selection SHALL survive appended batches, detail navigation and card reconciliation within the same filter. An action SHALL state the selected count, remain distinct from 全て既読 and preserve unselected data. The server SHALL validate every ID, active scope and CSRF before changing any row. Existing API bulk-state SHALL reuse the same atomic state-update implementation and retain its contract. Standard HTML form submission SHALL work without JavaScript.

#### Scenario: Selected subset update
- **WHEN** two selected articles among loaded and appended cards are marked starred
- **THEN** both are starred atomically, unselected states and history remain unchanged, and selected count remains accurate

#### Scenario: Invalid selection
- **WHEN** a selection includes an unknown or out-of-scope ID, duplicates, more than 100 IDs, or missing CSRF
- **THEN** the complete action is rejected with no state changes
