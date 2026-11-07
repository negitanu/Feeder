## Design
POST /api/social/sources は provider, actor, server（Mastodon の公開 HTTP(S) origin のみ）を検証し、公開 API で ID を解決する。Bluesky は public.api.bsky.app の AppView のみを使う。GET は sources 配列、DELETE は共通購読を削除する。全取得は既存の SSRF 対策、応答サイズ制限、タイムアウトを使う。SNS 設定は feed_id を外部キーとし、フィード URL は通常編集不可。更新は既存 lease と schedule、記事保存を使う。応答は公開投稿のみ選び、XML エスケープして feed-rs に渡す。ユーザー情報や秘密をログ・Git に保存しない。
## Existing behavior
既存 feeds/articles、フォルダ、既読、スター、検索、更新設定、削除と同一。認証と CSRF は既存ルータと origin 検査を使う。取得不能時は既存更新エラーに従う。
