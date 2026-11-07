# Reader catch-up 統合リリース計画

作成: 2026-11-07。状態: **採用済み設計契約**（2026-11-08）。Astra の先行計画を親が OpenSpec に具体化し、実装を分担した。実装・動作受入の結果は [検証記録](verification-reader-catchup.md) を参照。

## 先行する分担・統合契約

- Astra（計画担当）: 本書の先行API・保存・分担計画。親が `openspec/changes/reader-catchup/*` の要件・設計・受入条件へ具体化。
- 収集担当Terra: `collection.rs`、`social.rs`、`collection_health.rs`、新規 `collection_runtime.rs`、HealthPanel、別change `collection-reliability`。`collection_runtime::migrate(tx)` を新v10だけで呼ぶ。収集の詳細DDL・ページング契約は同担当の仕様を正とする。
- 改訂・関連記事担当Terra: 新規 `article_revisions.rs`、`story_groups.rs` と合成テスト。保存hookを親へ渡す。収集担当のファイルは直接編集しない。
- catch-up担当Terra: 新規 `catchup.rs` と合成テスト。関心・有限選定・確認/見送り/Undoを所有。
- UI担当Sol: 新規 `CatchupPanel.tsx`、`ArticleRevisionsPanel.tsx`、`RelatedArticlesPanel.tsx`、専用CSS/API型。`App.tsx` は編集しない。既存のテーマ・状態遷移・アクセシビリティを維持。
- ブラウザー検証担当Luna: 独立した合成DB、API/画面の回帰スクリプトと受入記録。
- 親: `main.rs`、`extensions.rs`、`ARTICLE_SOURCE_FIELDS`/`article()`、`App.tsx`、route/auth統合、保存hook接続、v10のスキーマ厳密照合・退避・復元テスト、目的別コミット。並行更新・移行・全体の統合検証を行う。

既存の各 `migrate()` と `extensions::recognized_v9()` の意味は不変。新規moduleの `migrate_catchup(tx)`（収集のみ既定の `collection_runtime::migrate(tx)`）を親がv10の単一transactionで集約する。以下の追加テーブル宣言は設計契約であり、この計画担当はDDLを実行しない。

## 先行API契約

既存の内部Reader認証・same-origin/CSRF方針を継承する。`/api/v1` のpublic APIは変更しない。GETは既読・訪問・catch-upの確認状態を変更しない。

- `GET /api/catchup?hours=24&limit=10`。`hours=24|168`、`limit=10|20`、任意の `feed_id` または `folder_id`。返却は `{generated_at, window_start, window_end, limit, policy_version, scanned, eligible_count, truncated, collection_incomplete, items}`。各itemは `{article, event_key, fingerprint, revision, kind, reasons, related_count}`。`kind=new|updated`。既存Article型へ後述metadataを追加し、独自の記事読取モデルを増やさない。
- `GET /api/catchup/interests`、`PUT /api/catchup/interests`。設定全体 `{revision, terms, priority_feed_ids}` を楽観ロックで更新。語句は文字通りのタイトル/本文一致、最大20語・各80 Unicode文字。Feedは最大20件。明示設定以外の閲覧履歴推定はしない。
- `PUT /api/catchup/acknowledgements`。body `{article_id, revision, fingerprint, action:"confirmed"|"skipped"}`。対象は当該版だけ。返却 `{article_id, event_key, revision, fingerprint, action}`。現在版と不一致なら409で無変更。
- `DELETE /api/catchup/acknowledgements`。body `{article_id, revision, fingerprint}`。同じ版へのUndo。既に取り消されていれば成功。新しい版の確認状態へ波及しない。
- `GET /api/articles/{id}/revisions?before_revision=...&limit=10`。降順履歴、本文は保存済みplain text、`{items, has_more, next_before_revision, history_complete, history_start}`。各版は `{revision, fingerprint, title, content, url, published, source_updated_at, observed_at, baseline}`。差分は隣接版のタイトル/本文/URL比較。事実としての「訂正」と自動断定しない。
- `GET /api/articles/{id}/related?limit=20`。`{article_id, method_version, articles, total, truncated, excluded}`。各関連要素に`article`と`reason`を返す。本文類似のgroupと正規化URL重複は理由を分ける。
- `PUT /api/articles/{id}/related-exclusion`。body `{excluded:true|false}`。誤集約の解除/復帰。解除記事はcatch-upで独立候補へ戻り、記事原本や他記事の読了には作用しない。

`reasons` は安定したコードと根拠値の配列（`interest_term`、`priority_feed`、`updated_since_visit`、`recent_unread`、`source_diversity`）。点数を真実性・社会的重要性・人気として表示しない。`source_diversity` は配信ホストの偏りを抑える選定枠を意味し、独立した取材源の数を意味しない。

共通Articleの追加metadataは `{revision:number|null, content_fingerprint:string|null, changed_at:string|null, has_update:boolean}`。版履歴の最初のsnapshotや移行時baselineを「更新あり」としない。`has_update` は実際の本文等変更の検知時刻と既存 `article_visits.visited_at` に基づく。catch-up表示はこのmetadataに加え、当該版への確認/見送り記録を参照する。

## 追加schemaの責任境界

- 改訂: `article_revision_state`（articleごとの初回観測・取得区分・現在版・fingerprint・最終内容変更・配信元更新時刻）、`article_revisions`（版ごとの本文snapshot）。同一transaction内で初回保存/変更前baseline/変更後snapshotと状態更新を行う。
- group: `article_story_index`（articleごとの保守的な索引鍵・本文fingerprint・発行時刻・source key）、`article_story_exclusions`（ユーザーの集約解除）。既存 `duplicates::migrate()` と `/api/duplicates` は維持。
- catch-up: `catchup_preferences`（単一共有Readerの関心設定・設定revision）、`catchup_acknowledgements`（article・版・fingerprintを主キーとするconfirmed/skipped）。共有DB設定・状態であることをUIに明記し、存在しない個人別認証を仮定しない。
- 収集: `collection_runtime` 担当が別契約で確定。初回バックフィル/通常取得/復旧バックフィルを改訂hookへ渡し、収集完全性をcatch-upから読めるようにする。名前未確定の収集テーブルへcatch-up担当がSQLを直書きしない。

完全な項目・hook・選定手順・受入条件は同changeの `design.md` と `specs/reader-catchup/spec.md` を正とする。契約を変更する場合は実装前にこの先行契約と合わせて更新し、親へ通知する。
