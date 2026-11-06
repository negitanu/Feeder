## Design
親が GET /api/library の feeds/folders を渡し、変更後に再取得する。既存 Feed 型の last_checked、last_error、paused を表示する。編集は親の onEdit に委譲し、PATCH /api/feeds/{id} と POST /api/feeds/{id}/refresh は apiFetch の CSRF 保護を利用する。停止中の更新を無効にする。検索はタイトル、URL、フォルダー名を対象とする。
選択は明示的に行い100件を上限とする。一括処理は逐次実行し、個々の失敗を報告して成功・失敗件数を区別する。すべての操作は処理中ロックし、完了後は成功・失敗を問わず親に再取得を通知する。ネットワーク障害時は反映結果が不明になり得るため再取得が必要である。既存の lease/conflict 判定はサーバーが担当する。
## Constraints
React/TypeScript の独立部品と合成データによる検証のみ。既存 App とフロントエンドファイル、保存データ、認証、外部取得保護を変更しない。
