## Purpose

Provide the implemented contextual-source-management capability from reader-platform-provider-refresh.

## Requirements

### Requirement: Source management belongs to the library
The settings hub and settings subnavigation SHALL remove 情報源の管理 and the registered information-source index. Feed editing SHALL remain accessible from the sidebar's right-click/ellipsis menus and shared in-place editor; source registration SHALL remain under 集める.

#### Scenario: Settings navigation
- **WHEN** a user opens settings or a settings section
- **THEN** source-management entries and source-specific edit lists are absent, while global collection/display settings remain accessible

#### Scenario: Reach source controls
- **WHEN** a user opens a sidebar feed's context menu
- **THEN** collection, URL, folder, stop/resume and deletion actions appropriate to that feed are available without opening a management page

#### Scenario: Old bookmarks and no JavaScript
- **WHEN** an old /settings/feeds link or a direct feed editor bookmark is opened
- **THEN** the list URL leads to the reader and a direct editor remains usable as a fallback, without reintroducing the removed settings entry

#### Scenario: Nonduplicated collection entry points
- **WHEN** a user opens 集める
- **THEN** source registration links appear once within the task groups, without a second shortcut panel above them, and RSS/Atom, SNS topics, page monitoring and Mastodon account subscriptions remain reachable

#### Scenario: Task hubs without redundant shortcut rows
- **WHEN** a user opens 集める, 調べる, まとめる or 設定
- **THEN** the page omits the upper shortcut panel and section-jump link row, retains its functional groups and unique actions, and does not repeat the lower task entries above them

#### Scenario: Hub page identity without duplicate visual headings
- **WHEN** a task hub is displayed on a desktop or mobile viewport
- **THEN** its active global navigation and accessible main name identify the area without repeating its name as a visible page heading, and functional groups align in responsive columns without horizontal overflow
