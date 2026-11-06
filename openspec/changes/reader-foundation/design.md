## Context
The repository contains plans only. Rust is not installed in the current shell. Node 22 is available.
## Goals / Non-Goals
Goals: runnable reader slice, generated data, persistent SQLite, minimal trusted HTTP boundary.
Non-goals: production deployment, legacy database migration, external collection and full parity.
## Decisions
Use a Rust HTTP service and SQLite, with React/TypeScript and Vite for development. Serve frontend assets from the backend for a same-origin production-like local build. APIs use /api/articles with limit/offset, optional unread/starred/q filters, /api/articles/{id}, POST /api/articles/{id}/state with read/starred optional booleans. Detail fetch is read-only; UI explicitly records read. Require JSON and reject cross-site mutation origins. Bind loopback by default. Use plain text article bodies initially; do not render untrusted HTML. Sample data is explicit opt-in. Reject non-Feeder SQLite schema before writes. Dependencies are selected from available current registries and locked after verification.
## Risks / Trade-offs
This slice does not establish full parity or deployment security. Legacy DB migration remains a separate gate. Node runtime is build-time only for the assembled application.
