# Reader catch-up backend

正本は [reader-catchup-plan.md](reader-catchup-plan.md) と `openspec/changes/reader-catchup/`。この文書は backend の実装・局所検証を記録する。統合受入、環境・画面別の検証完了を意味しない。

## 統合契約と保存

- 親は v10 の単一 transaction 内で `catchup::migrate_catchup(tx)` を呼び、`catchup::routes()` を既存 Reader の認証・Origin・CSRF 境界に組み込む。旧 migration は変更しない。
- 関心語と優先 feed は共有 Reader の設定であり、全体を revision 付きで更新する。feed の削除に伴う優先指定の削除でも revision が進む。確認・見送りは `article_id`、revision、fingerprint ごとに保存し、現行版照合と PUT は同一 `IMMEDIATE` transaction で行い、設定更新と Undo も書込み前に予約ロックを取得する。別接続の書込み完了後に現在版を再確認するが、競合が busy timeout を超えた場合は失敗を返す。Undo は指定版のみを削除し、繰り返しても成功する。
- 既存記事の読取は親の `lookup()` と共通 Article metadata を使用する。更新の判断は改訂担当の `article_revision_state` と訪問履歴を参照し、収集不足は `collection_runtime::is_incomplete()` を呼ぶ。関連記事の最終判定は `story_groups::related_ids()` を使用する。
- `catchup_preferences`、`catchup_priority_feeds`、`catchup_acknowledgements` は追加テーブル。優先 feed と確認記録は feed／article 削除に連動する外部キーを持ち、通常の完全 DB バックアップ・復元の対象となる。

## 選定と上限

単一の UTC 時刻を上限とし、24 時間または 168 時間の期間から最大 5,000 件を評価する（さらに 1 件を取得して走査打ち切りを検出）。新着は通常取得された未読記事で、有効な配信日時が対象期間内にあるものに限る。初回・復旧・既存 baseline は新着にしない。更新は最後の訪問より後の実際の内容変更で、変更時刻が期間内にある場合だけ示す。ミュートと現行版の確認・見送りを除く。

明示関心語の文字どおりの大文字小文字を区別しない一致、優先 feed、更新、イベント時刻、記事 ID の順で決定的に並べ、関連代表を作り、同じ配信ホストを最大 2 件に抑える。関心語は最大 20 件・各 80 Unicode 文字、優先 feed は最大 20 件。理由は `interest_term`、`priority_feed`、`updated_since_visit`／`recent_unread`、`source_diversity` で返す。

関連記事については公開索引の URL／本文・タイトル・日付キーで一致可能性を絞ってから story helper で判定する。索引条件の変更時はこの事前絞り込みも同時に見直す。関連展開は最大 100 回で打ち切り、各 helper も公開契約の走査上限に従う。走査、関連展開または関連記事側の上限に達した場合 `truncated=true` となる。`scanned` は取得して評価した候補数、`eligible_count` は関連代表にまとめた評価済み部分の件数であり、関連展開打ち切り時は全走査候補の件数ではない。結果は網羅性を保証しない。`collection_incomplete` は収集の不足を別途示す。

## 局所検証

`cargo test --locked --offline --manifest-path backend/Cargo.toml catchup::tests` の合成 9 テストで、日英・文字どおりの語句、設定 revision と feed 削除、HTTP 入力、確認競合と Undo、別スレッドの現行版更新との競合、別 SQLite 接続同士の設定更新競合、次版の再提示、ミュート、初回／復旧、未来・不明日付、関連記事の除外、同一ホスト上限、5,001 件の走査上限、関連記事展開の打ち切りを確認した。既読・スター・訪問・後で読むは確認操作から独立する。索引付き合成 5,001 件のデバッグ実行は手元で約 33 ms だったが、実環境の性能保証ではない。

全 backend テスト・Clippy および実環境／ブラウザの統合結果は親の受入検証を参照する。
