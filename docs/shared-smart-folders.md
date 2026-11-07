# 共有検索

ブラウザ内の保存検索はそのまま残ります。「共有にコピー」で同名の共有検索がない場合にコピーできます。コピー後は共有一覧を再取得し、更新ボタンからも再取得でき、適用時は既存の情報源検証を使います。削除済みフィードやフォルダを含む検索は適用せず、対象を全体に広げません。

## 統合契約

- `smart_folders::migrate(&tx)` を v9 トランザクションから呼び、`smart_folders::routes()` をルーターにマージする。厳格スキーマ照合とバックアップを既存のバージョン移行と同様に更新する。認証・CSRF の対象に全エンドポイントを含める。
- `smart_folders(id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE, snapshot TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision > 0))`。最大100件。`snapshot` は既存 `SearchSnapshot` の JSON。
- `GET /api/smart-folders` は `{id,name,snapshot,revision}` の配列。`POST` は `{name,snapshot}` を受け 201 と作成レコードを返す。`PATCH /api/smart-folders/{id}` は `{revision,name?,snapshot?}` を受け更新レコードを返す。`DELETE` は `{revision}` を JSON で受け 204 を返す。重複・上限・古い revision は 409、未存在は 404、無効入力は 400。JSON の形式不正は Axum の拒否応答。
- `SmartFolderList` の props は `{onApply:(snapshot:SearchSnapshot)=>string|null; refreshKey?:number}`。App から既存 SavedSearches の適用コールバックと同一の検証付き処理を渡す。コピー成功時に `SavedSearches.onShared` から `refreshKey` を進める。
- レコードの名前は前後の空白なしで80 UTF-16 code units 以内、空白のみ不可。query と各詳細文字列は1000 UTF-16 code units 以下。snapshot は既存保存形式の列挙値と正の安全整数の source ID に制限する。新規作成・条件更新時は reader と同じ構文と情報源の存在を検証する。`later` フィルターと `applied.later` / `applied.include_muted` の真偽値を保持する。保存後に情報源が削除されてもレコードは残り、名前変更できる。適用時は情報源を再検証し、範囲を広げない。

テストは合成データを使う。認証、CSRF、移行、別セッション共有と画面操作の合成検証は [統合検証](verification-reader-expansion.md) を参照。
