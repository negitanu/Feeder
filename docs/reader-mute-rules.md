# ミュートルールの統合契約

この機能は記事を非表示にするだけで、記事や既読・スターを変更しない。通常の一覧と一括操作に `NOT ({HIDDEN_SQL})` を同じ条件として追加し、`include_muted=true` の場合だけ省略する。述語はプレースホルダーを持たず、外側の記事テーブル名は `articles` が必要。フィード ID が NULL のルールは全配信元、`both` はタイトルと本文。SQLite の `lower`/`instr` で ASCII の大小文字を区別しないリテラル部分一致を行い、Unicode の大文字小文字変換は行わない。

`mute_rules::migrate(tx)` を共有マイグレーションで呼び、`mute_rules::routes()` を組み込む。外部キーを有効にして配信元削除時の対象ルールの連鎖削除を適用する。ページ継続・一括操作の fingerprint は、ルール追加・変更・削除および配信元削除による連鎖削除を検出すること（各行の revision の合計だけでは不十分）。認証・CSRF/Origin 保護は既存 API と同等に組み込む。

- `GET /api/mute-rules`: `{ "rules": [{ "id", "name", "term", "field", "feed_id", "enabled", "revision" }] }`
- `POST /api/mute-rules`: `{ "name", "term", "field", "feed_id", "enabled" }`、成功 201 とルール。最大100件。
- `PATCH /api/mute-rules/{id}`: `revision` 必須、他の項目は部分指定。`feed_id: null` は全配信元、未指定は変更なし。成功時は更新されたルール。
- `DELETE /api/mute-rules/{id}`: JSON `{ "revision": N }`、成功 204。
- `GET /api/articles/{id}/mute-reasons`: `{ "reasons": [{ "id", "name" }] }`。有効かつ一致するルールのみ。存在しない記事は 404。

名前は空白のみ不可・80 UTF-16 単位以内、term は空白のみ不可・200 UTF-16 単位以内。field は `title`, `body`, `url`, `both`。存在しない配信元や不正入力は 400、古い revision は 409、存在しないルールは 404、100件上限は 409。削除・更新には既存 API と同様 Origin 検証を行う。

`MuteRulesPanel` は `feeds: Feed[]` と任意の `onChanged(): void` を受け取る。ルール保存後の一覧再取得や継続トークン破棄は親の責務。現時点で画面への接続、reader の絞り込み、認可・マイグレーション登録と実ブラウザーでの PC／モバイル・テーマ・マウス／タッチ／キーボード検証は統合担当の未完了作業。

統合後は一覧・一括操作・ダイジェストに適用し、ミュートを含めて表示した記事の詳細で一致したルール名を表示する。[統合検証](verification-reader-expansion.md)。
