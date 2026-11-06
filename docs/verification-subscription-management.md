# 購読管理の検証

## API と統合の前提
- 親は `GET /api/library` の `{ folders, feeds }` を読み込み、コンポーネントに渡す。folder は `{ id, name }`、feed は `{ id, url, title, folder_id, paused, last_checked, last_error, poll_interval }`。後ろの3項目は null になり得る。日時は API の文字列をそのまま表示する。
- `onChanged()` は成功・失敗に関係なく各操作完了時に一度呼ぶ。親がライブラリを再取得して props を更新する。コールバックの非同期完了は待たない。`onEdit(feed)` は親の編集 UI を開く。
- 停止・再開は `PATCH /api/feeds/{id}` に JSON `{ "paused": true|false }`、更新は `POST /api/feeds/{id}/refresh`。`apiFetch` が同一オリジンのセッションと CSRF を扱う。更新中の lease や停止中の更新は 409 になり得る。応答が失敗でも更新結果の再取得が必要。
- 一括操作は選択した最大100件を順番に送信する。検索で隠れた選択も対象。成功と HTTP エラー／通信エラーを区別し、通信エラーは反映不明の可能性を表示する。削除 API は呼ばない。

## 実施した検証
- `npm --prefix frontend run build`：型チェックとビルド成功。
- `node scripts/subscription-manager-browser-smoke.mjs`：合成 fixture とモック API を使用。検索、日時・エラー表示、編集、停止中の更新禁止、100件制限、逐次処理、部分失敗、実行中の操作ロック、CSRF、更新 API、390px 幅を確認。
- 実バックエンド・統合済み App の通し検証、テーマ、タッチ操作はこの単体検証の対象外。親による統合回帰で確認する。
