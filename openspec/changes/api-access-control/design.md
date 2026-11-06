## Context

Legacy references are `feedastrolabe/access.py`, `keys.py`, `api.py`, `settings.py` and `docs/api.md`. Public API uses `/api/v1`; current Feeder `/api/*` routes are browser-facing internals. Legacy default public mode delegates identity and JWT verification to Cloudflare Access/Protect with Access over an unpublished origin. The current shared database is v6.

## Goals / Non-Goals

**Goals:** Browser write protection, separated API scopes, persistent key lifecycle/rate limits, core RSS/article API contracts, and reviewable container access configuration.

**Non-Goals:** Live external deployment or secret creation for deployment, application-side JWT/JWKS verification, migrated legacy keys, unmigrated research/SNS/page APIs, browser user accounts/passwords.

## Decisions

- Schema v7 adds API-key metadata/hash (never raw keys) and a singleton API rate-limit setting default120/minute (1..1000). Preserve schema recognition v1..v6, back up before migration, keep data/history/order/settings.
- Middleware wraps the production router, preserving the internal test handler helper. Public `/api/v1/*` requires Bearer only, never browser-cookie substitution; write scope is required for non-safe methods. Internal key administration cannot be authorized by a Bearer key.
- `GET /api/session` creates/reuses an unpredictable in-memory browser session and returns `{csrf_token}` with no-store. Cookie `feeder_ui` is HttpOnly, SameSite=Strict, Path=/, Secure in tunnel mode. Sessions expire after12h and are bounded1024; process restart invalidates them. Mutating `/api/*` outside `/api/v1` requires a valid cookie plus constant-time-checked `X-Feeder-CSRF` and matching Origin when present, even without Origin. Sec-Fetch-Site cross-site is refused. The frontend primes sessions via a shared request wrapper; expired requests are not silently replayed.
- GET/POST `/api/keys`, DELETE `/api/keys/{id}`, GET/PATCH `/api/keys/settings` are session-protected. Issue `{name,scope,days}` (trimmed1..80 chars, read|write, integer1..365). Return `{id,token}` once with no-store; list `{keys:[{id,name,prefix,scope,created_at,expires_at,revoked_at,last_used_at}]}` has no hash or token. Rate settings `{rate_limit}`. Generate256 random bits; store SHA256 hex only. Revoke atomically. No key material enters export or logs.
- Public core endpoints GET feeds/list/detail, POST/PATCH/DELETE feed and POST refresh, GET articles/list/detail and PATCH article with `{is_read?,is_starred?}`. GET list uses ID desc, before_id, or after_id asc; limit1..100 default30, feed/is_read/is_starred/q (literal<=200chars), source_type=rss only. Return `{data,...}` with next_before_id/next_after_id. Reject duplicate/unknown/conflicting filters and unsupported source settings. Preserve old RSS field meanings (folder name, enabled), with absent source-specific functionality explicitly unsupported rather than silently ignored. Network creation/refresh reuse safe collector. Article GET/PATCH never creates visits.
- Authenticate and count requests in an immediate SQLite transaction to serialize revocation/expiry/rate checks across processes. Return JSON error envelope `{error:{code,message}}`;401 Bearer challenge,429 Retry-After. Keys last_used/rate counters are runtime and excluded from export. No token in query accepted.
- `FEEDER_ACCESS_MODE=local` default restricts production Host authorities to loopback/localhost/web in addition to existing allowlist. `tunnel` requires `FEEDER_PUBLIC_ORIGIN=https://<explicit DNS authority>` and only that host (except healthz); uses exact configured origin, ignores Forwarded/X-Forwarded identity. Cloudflared verifies Access before origin traffic. Strip a validated HTTPS Origin before existing internal handlers, rather than trusting arbitrary proxy scheme headers. Invalid mode/config fails startup. Worker needs no browser access config.
- Optional Compose override clears published Web ports and connects a cloudflared service with a secret-mounted token file. Configuration must explicitly supply public origin, pinned tunnel image and token-file location. Do not run the connector or publish anything during this work. Document Access policy and Protect with Access checks and the residual trusted-network boundary.

## Risks / Trade-offs

- Tunnel mode trusts its unpublished ingress network → never publish origin; connect trusted services only; do not interpret the presence of an assertion header as verification. Direct JWT mode remains unavailable.
- Restored DB contains token hashes → treat DB backups as secret; rotating/revoking keys after restore is operator policy. JSON config export omits key metadata/hash and rate counters.
- Session restart/expiry breaks pending actions → clear client cache on403 and require explicit retry; no automatic replay.
- New API dependencies → use RustCrypto sha2 and subtle with locked versions and Linux tests. No handwritten hashing.

## Migration Plan

Run synthetic v6 migration and production-router tests; stop Web/worker before deployment; retain v6 backup. For rollback stop both and restore pre-upgrade DB into a separate volume with prior image. Verify external unauthorized/allowed/service-token paths separately with actual Access configuration before deployment acceptance.

## Open Questions

None for this bounded implementation. Full legacy public API field parity remains tracked separately.
