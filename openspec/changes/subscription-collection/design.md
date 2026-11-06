## Context
Reader foundation uses a dedicated SQLite schema with article states and loopback HTTP protection.
## Goals / Non-Goals
Goals: RSS/Atom registration, manual refresh, safe external fetching, folder membership, retained read/star, contextual stop/resume/delete.
Non-goals: website discovery, session-bound preview plans, periodic worker, OPML, full legacy migration, advanced drag ordering and complete parity.
## Decisions
Add feeds and folders with integer identities and article feed_id/external_id; preserve existing article IDs during v1 migration. Registration accepts a direct HTTP(S) feed and refetches bounded content before commit. Validate every redirect and resolved IP; pin validated addresses to the connection; reject credentials/private/loopback/linklocal/multicast/metadata endpoints; disable environment proxies. Bound bytes, redirects, timeout and entry count. Tests use an injected fetcher or fixture parser, not a production bypass of private-IP protection. Refresh snapshots feed settings without DB locks during network I/O; transaction rechecks paused/existence before changes and preserves article states. Conditional headers/304 retained. Parse/sanitize article bodies into plain text. API contract documented before UI integration. No raw HTML rendering.
## Risks / Trade-offs
SSRF/redirect/DNS rebinding require negative tests. Data migrations only apply to known Feeder schema and preserve sample records. Parent feed deletion cascades only its own articles. Feed pause retains all articles. Full source specification remains partially migrated until later phases.
