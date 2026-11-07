# 収集の信頼性

## 動作と制限

SNS は 1 回につき最大 2 ページ、各 40 件を取得する。継続 cursor と、完了した収集サイクルの high-water mark を保存し、最後まで到達するまで high-water mark を更新しない。最初のサイクルも同様に継続し、完了前は「続きを取得予定」と表示する。新着と過去分の区別には取得前の `collection_context` を用いる。SNS の公開投稿・対象作者・リポスト除外は各ページに適用する。

Mastodon は `max_id` を使い、短い応答でも次の空ページまたは既存高水位まで継続する。`edited_at` と `created_at` を別々に保存する。Bluesky はサーバーが返す opaque cursor をそのまま次の API パラメーターに渡す。Bluesky のページ間に一貫したスナップショットが保証されないため、取得漏れの可能性を表示する。旧 high-water mark が消失した場合も欠落の可能性を記録する。欠落フラグは自動的に完全性を回復したと見なさない。

429/503 の `Retry-After`（秒数または HTTP-date）と、失敗ごとに 60 秒から最大 1 日まで増える指数バックオフの遅い方を次回期限とする。手動・定期の双方で期限前の取得を拒否する。待機中の feed が他 feed の定期収集を塞がない。各ホストの通信はプロセス内で同時 1 件に制限し、リダイレクトごとに URL 検証と DNS pinning を行う。

処理結果、記事および checkpoint は、期限内の所有リースを確認した単一トランザクションで保存する。URL・SNS 接続先の変更は対応する runtime と診断を消去する。取得元の変更で既存記事・既読等は削除しない。

## 統合契約

- `collection_runtime::migrate(tx)` は既存 v9 の変更ではなく、新規 v10 transaction に追加する。旧 schema 判定と移行処理の意味を変えない。
- `collection_runtime::is_incomplete(db, feed_id, folder_id)` は指定範囲に停止、定期収集無効、未成功、エラー、未到来の再試行期限、継続中、欠落可能性または期限超過があれば true。両指定は AND、どちらもなしは全体。対象 feed がない場合 false。
- `collection_context(db, feed_id)` は取得結果を記事へ保存する**前**に呼び、`InitialBackfill` / `Normal` / `RecoveryBackfill` を記事版保存 hook に渡す。新 RSS 購読の最初の保存は `InitialBackfill` を直接渡す。SNS の初回サイクルは複数ページにまたがって初回扱い、障害後または前回成功から設定間隔の 2 倍を超えた後のサイクルは回復扱いとし、複数ページにまたがって区分を維持する。記事版 hook は `store_entries` に接続済み。
- RSS 登録時は成功した最初の fetch と記事保存を同一 transaction に記録する。URL 変更時は診断と取得時刻が消え、次の取得は初回扱いとなる。
- Health API は `collection_state`（`paused` / `waiting` / `error` / `catching_up` / `uncollected` / `idle`）、`next_retry`、`more_pending`、`gap_possible` を返す。`next_due` は定期収集が有効な場合の実行可能時刻を示す。

## 出典と検証

[Mastodon account statuses](https://docs.joinmastodon.org/methods/accounts/#statuses) は `max_id` と最大 40 件を定義する。[Bluesky author feed Lexicon](https://github.com/bluesky-social/atproto/blob/main/lexicons/app/bsky/feed/getAuthorFeed.json) は要求・応答双方の `cursor` を定義する。

合成 SQLite で v10 状態の追加と URL 変更時の消去、範囲別の不完全性、収集文脈、待機と古いリースを検証する。SNS checkpoint のページ遷移と停止条件、HTTP 日付、ホスト別同時制限も合成テストで検証する。複数のrefreshをまたぐカーソル循環は、サイズ一定の検出状態をDBに保持して拒否する。実 DB・実サービスの取得テストではない。
