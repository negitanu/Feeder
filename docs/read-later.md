# あとで読む

記事をスター・既読とは独立して保存する。保存順の新しいものから最大100件を表示し、明示的に削除できる。設定の「記事を開いたときに自動で削除する」は初期状態では無効。既読にするだけでは削除しない。

## 統合契約

- マイグレーショントランザクションで `read_later::migrate(&tx)`、ルーターで `read_later::routes()` を呼ぶ。
- 実際の記事訪問時に `read_later::on_visit(&connection, id)` を呼ぶ。既読操作からは呼ばない。
- `GET /api/read-later` は `{items:[{id,title,url,published,saved_at}]}` を返す。
- `POST /api/articles/{id}/read-later` は `{saved:boolean}` を受け、同じ形の状態を返す。存在しない記事は 404。
- `GET` / `PATCH /api/read-later/settings` は `{auto_remove:boolean}` を返し、PATCH は同じ形を受ける。
- `ReadLaterButton` は `id`, `saved`, `onChanged`、`ReadLaterPanel` は `onOpen(id)` を受け取る。親は記事の訪問と表示状態の更新を担当する。

保存先は `article_read_later` と単一行の `read_later_settings`。記事削除時は外部キーの cascade でキュー項目も削除する。既存 SQLite のバックアップ・復元に含まれる。テストは合成データのみを用いる。
