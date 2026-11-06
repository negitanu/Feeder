## Why

FeedAstrolabe displays article details in a floating panel while retaining the reading list. Feeder currently hides that list and lets delayed visit responses reopen a closed or superseded detail. Port the floating panel and safe dismissal independently of later browser-history/URL work.

## Correspondence

Reference: `reader-interactions` floating detail, bounds and read-state requirements, and `reader-panel-polish` outside dismissal and stale response requirements. This change does not implement URL/history navigation, hero images, research/evidence controls, footer navigation or a no-JavaScript reader.
