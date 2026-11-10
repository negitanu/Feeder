# Collection diagnostics contract

This feature reports collection state for Feeder's existing Web subscriptions. It does not migrate FeedAstrolabe data, initiate a collection, claim a refresh lease, or write on GET.

## Owned implementation and integration boundary

`backend/src/collection_health.rs` provides `migrate(&Transaction)`, `record_attempt(&Connection, feed_id, success)`, `routes() -> Router<Db>`, and the Axum handler `handler(State<Db>)`. `routes()` serves GET `/api/collection/health`; main merges it under the application's normal read-session/API-key authorization. The migration creates `collection_health(feed_id PRIMARY KEY REFERENCES feeds(id) ON DELETE CASCADE, last_success TEXT NULL, consecutive_failures INTEGER NOT NULL DEFAULT 0)` and seeds known legacy successes (`last_checked` present, `last_error IS NULL`). The collection implementation owner, not this route, calls `record_attempt` only after it confirms that the finishing attempt still owns the accepted lease. On a rejected/stale lease completion, do not record an attempt. A successful accepted attempt resets failures; a failed accepted attempt increments them while retaining the last success.

The handler's response type is:

```json
{
  "global_enabled": true,
  "global_interval_seconds": 900,
  "truncated": false,
  "feeds": [{
    "id": 12,
    "name": "Example Feed",
    "url": "https://example.invalid/feed.xml",
    "paused": false,
    "last_checked": "2026-11-07 01:02:03",
    "last_success": "2026-11-06 23:02:03",
    "error": null,
    "next_due": "2026-11-07 01:17:03",
    "consecutive_failures": 0,
    "stale": false,
    "individual_override": false,
    "poll_interval_seconds": null
  }]
}
```

The endpoint reads at most 1001 rows and returns at most 1000, ordered by feed id. `truncated` explicitly signals that additional rows exist. Global settings and feed data are obtained with SELECT only; the endpoint does not create missing rows or update legacy fields. `next_due` is null while the feed is paused or global collection is disabled; otherwise it is now when never checked (matching the scheduler's immediate due behavior), or `last_checked` plus the explicit feed interval or inherited global interval. `stale` is false for paused/global-disabled feeds; otherwise a feed is stale if it has no known successful collection or its last success is at least one effective interval old.

For old feeds with no `collection_health` row, `last_success` falls back to `feeds.last_checked` only if `feeds.last_error` is null. If a legacy error exists, its success history is unknown and `last_success` remains null. `consecutive_failures` is zero for legacy rows because the old schema does not retain a count; no historical count is inferred. Once a row exists, the new recorded `last_success` is authoritative.

## UI component

`frontend/src/CollectionHealthPanel.tsx` is mounted in the Settings health section of `App.tsx`. App supplies the authorized refresh callback, increments `refreshKey` after a retry, and passes the configured display timezone. The reusable component API is:

```ts
CollectionHealthPanel({
  onRetry: (id: number) => Promise<void>,
  refreshKey?: number,
  timezone?: 'UTC' | 'Asia/Tokyo',
})
```

The panel GETs on mount and when `refreshKey` changes, and offers all/error/stale filters. Timestamps are absolute (including date and timezone), never relative. Retry delegates only to `onRetry`; it does not assume or implement an endpoint. The component reports the 1000-row truncation and has responsive table overflow.

## Verification status

Backend unit tests use an in-memory synthetic SQLite schema and `example.invalid` URLs. The collection-health tests exercise migration and legacy-success seeding, failed/successful attempt transitions, legacy fallback, paused/global-disabled due times, per-feed override, cascade deletion and the 1000-row response limit. The full backend `cargo test` suite passed (75 tests in the latest checkout run). The frontend production build (`npm run build`, including TypeScript) passed. Browser interaction, visual layout, and assistive-technology checks have not been run for this change. No real subscriptions or secrets are fixtures.

## 収集失敗の案内

新しい失敗は配信元のHTTPステータス、名前解決、接続、タイムアウト、受信、転送、安全制限、サイズ超過、フィード解析を安全な固定コードで記録し、設定の収集状態に原因と確認先を示す。配信元の応答本文・URLや例外の詳細は診断として追加保存しない。過去の詳細がないエラーは原因不明と表示する。最後の成功と次回予定を併記し、購読管理へ移動できる。停止中のフィードも直近のエラーを表示する。待機中の再試行は無効であり、429/503の `Retry-After` を維持する。
