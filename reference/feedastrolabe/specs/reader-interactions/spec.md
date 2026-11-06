## Purpose

Provide the implemented reader-interactions capability from reader-platform-provider-refresh.

## Requirements

### Requirement: Persistent disclosure without reset control
The reader SHALL remove the disclosure-reset button and its event handling while retaining stored folder disclosure preferences and arrow-only toggling.

#### Scenario: Existing collapsed folder
- **WHEN** a user revisits a folder that was collapsed before the update
- **THEN** it remains collapsed, no reset control is offered, and selecting its name displays articles without expanding it

### Requirement: Pause and resume subscriptions in context
Every supported feed kind SHALL offer 購読停止 or 購読再開 through its right-click and ellipsis menus. A stop SHALL disable subsequent scheduled and user-triggered collection for that feed, preserve its registration and historical data, and exclude it from global manual refresh. Resume SHALL restore eligibility using the saved interval and source configuration. Already running requests MAY complete; the stop SHALL NOT be presented as cancellation of an in-flight network request.

#### Scenario: Stop without deleting
- **WHEN** a user stops an enabled RSS, supported SNS or page feed
- **THEN** it remains readable, its articles/read/star values and source settings are unchanged, and no new collection job is dispatched for it

#### Scenario: Resume
- **WHEN** a user resumes a stopped feed
- **THEN** it becomes eligible for the normal scheduler, the menu changes to 購読停止 and the action itself does not fetch articles

#### Scenario: Queued collection and stopped SNS automation
- **WHEN** a feed is stopped while collection or automatic extraction work is queued
- **THEN** queued work is prevented from starting or is cancelled, and workers recheck eligibility before outbound execution

#### Scenario: Unsupported source and validation
- **WHEN** resume is attempted for retired X collection, a missing feed, invalid values, duplicate parameters or a request without CSRF
- **THEN** the mutation is rejected and other settings or feeds are not changed

### Requirement: Prominent scoped bulk read
The bulk-read control SHALL use the exact visible label 全て既読 and primary theme-aware styling. It SHALL mark all articles in the active filter as read, including articles not yet rendered.

#### Scenario: Bulk read before all cards load
- **WHEN** a user chooses 全て既読 in a folder/search/unread/workspace/tag filter
- **THEN** all matching articles become read, articles outside that filter keep their state, and loaded content is reconciled with the result

#### Scenario: Light dark and system appearance
- **WHEN** the app uses light, dark or system appearance
- **THEN** the button remains visually prominent with visible keyboard focus and text contrast of at least 4.5:1

### Requirement: Floating article detail and original access
The reader SHALL open article details on ordinary selection in a right-aligned floating panel without shrinking or scrolling the retained list. The panel SHALL fit mobile viewports and scroll independently using theme colors. Middle-clicking a card outside its state controls SHALL open its original HTTP(S) URL in a new tab without selecting or marking the article read, including appended cards and mark-read-on-open mode. Missing original URLs SHALL NOT open a tab. The detail title SHALL link to the original in a new tab with noopener noreferrer when available, and remain plain text otherwise. The separate original button and upper previous/count/next navigation SHALL be removed; footer navigation SHALL remain and explicit read-next controls SHALL be removed.

#### Scenario: Open and close floating detail
- **WHEN** a user selects and closes an article on desktop or mobile
- **THEN** list geometry, loaded cards and scroll position remain unchanged and the detail fits the viewport

#### Scenario: Middle click original
- **WHEN** a user middle-clicks an initial or appended card with either read-on-open setting
- **THEN** its original opens in a new tab with no detail selection or read-state mutation

#### Scenario: Simplified detail header
- **WHEN** a detail has an original URL
- **THEN** its title opens that URL in a new tab, no separate original button or upper sequence appears, and footer navigation remains

#### Scenario: Missing original
- **WHEN** an article has no original URL
- **THEN** its detail title is plain text and middle click opens no new tab

### Requirement: Content-first article detail
The detail SHALL present an optional hero image, linked title, timestamp, read/star controls, a horizontal separator, sanitized article description, a horizontal separator, research/tag and evidence controls, and footer previous/count/next navigation in that order. The source/close bar SHALL remain available. All read-next controls SHALL be removed.

#### Scenario: Read before research
- **WHEN** a user opens an article
- **THEN** description precedes research/evidence controls with explicit horizontal separators and no read-next control is displayed

### Requirement: Optional bounded hero image
The reader SHALL attempt to obtain an Open Graph or Twitter Card image from the saved original URL without delaying the detail HTML response. It SHALL render a successfully loaded image before the title at 480 CSS pixels, constrained by available width. Missing or failed images SHALL leave no visible placeholder. Original page and image requests SHALL reuse public-address validation, DNS pinning, redirect validation and time/size limits. Only PNG, JPEG, GIF and WebP image signatures SHALL be served through a same-origin article endpoint; SVG and non-image responses SHALL be rejected. Successes and failures SHALL be cached with bounded entries and lifetime. Existing CSP and access controls SHALL be preserved.

#### Scenario: Image metadata exists
- **WHEN** the original page declares a usable og:image or twitter:image including relative URLs
- **THEN** the image is fetched with the existing network protections and appears above the title without overflowing mobile viewports

#### Scenario: Image unavailable or unsafe
- **WHEN** the article has no URL, metadata, a successful image response, or points to a blocked address or unsupported format
- **THEN** the article remains readable and no broken-image placeholder appears

### Requirement: Detail bounded by the article area
The floating detail SHALL start no higher than the article-area heading or the visible lower edge of the global navigation and reader breadcrumb. Its bottom SHALL remain within the viewport and its contents SHALL scroll independently. Its position SHALL update during scrolling, viewport resizing and sidebar layout changes without shrinking or scrolling the list. The global header SHALL remain clickable above the detail.

#### Scenario: Open detail below upper navigation
- **WHEN** a desktop or mobile user opens a detail or loads a detail URL
- **THEN** the panel starts within the article area, does not overlap the global navigation or breadcrumb, and fits above the viewport bottom

#### Scenario: Scroll or resize
- **WHEN** the user scrolls the list, resizes the window or changes mobile sidebar height
- **THEN** detail bounds follow the visible article area and header navigation remains available

### Requirement: Mark displayed article as read
The reader SHALL persist read state together with the CSRF-protected visit POST after displaying an article detail, independently of legacy read-on-open preferences. GET requests and middle-click original access SHALL NOT mutate read state. Successful saves SHALL update the matching retained card, detail state controls and scoped counts without closing the detail or losing loaded cards and scroll position. Failed saves SHALL display an error without claiming success. Explicitly marking the displayed article unread SHALL remain possible without immediately marking it read again.

#### Scenario: Select unread article
- **WHEN** a user opens an unread article in any list using mouse, touch or keyboard
- **THEN** the displayed article becomes read, its star remains unchanged, the unread count updates and the detail stays open even in the unread view

#### Scenario: Navigate and fail
- **WHEN** the user navigates to another detail or returns through browser history
- **THEN** the displayed detail is recorded as read; a failed detail fetch or failed visit save does not claim a successful read update

#### Scenario: Explicit unread and middle click
- **WHEN** the user marks the current detail unread or middle-clicks another article
- **THEN** that explicit unread state is preserved and middle-click changes no read state

### Requirement: Global unread library entry
The library 未読 navigation SHALL open unread articles across the entire library, clearing folder, feed, search and other selected scopes. Its badge SHALL always show the global unread article count, including after detail visit updates. A folder- or feed-scoped unread view SHALL NOT mark the global unread entry as active. Explicit folder and search filters SHALL remain available after entering global unread.

#### Scenario: Leave a selected folder
- **WHEN** a user selects 未読 while viewing a folder, feed or scoped search
- **THEN** unread articles from all folders including unclassified feeds appear, the URL contains no previous scope and the global unread entry is active

#### Scenario: Consistent unread badge
- **WHEN** a user views a folder and opens one of its unread articles
- **THEN** the badge decreases from the library-wide unread count while folder result counts remain scoped

#### Scenario: Scoped unread and bulk read
- **WHEN** a user explicitly chooses a folder within unread or marks the selected folder read before choosing global 未読
- **THEN** the folder retains its filter while selected, and choosing global 未読 still includes remaining unread articles outside that folder
