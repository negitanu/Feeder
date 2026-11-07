# Feeder v1.0.0 リリースノート

Feeder は Rust・React・Linux コンテナで動作する Web 専用 RSS / SNS Reader です。

- RSS / Atom、公開 Mastodon・Bluesky の購読と自動収集
- フォルダー、検索、既読、スター、あとで読む、ミュート、OPML
- 関心語や優先フィードを使ったキャッチアップ、記事の更新履歴と関連記事
- 標準・1行・タイル表示、モバイル対応、明暗テーマ
- Docker Compose と Cloudflare Tunnel、API キー、バックアップ・復元

## 更新前に

[配備とバックアップの手順](deployment.md)に従い、Web と worker を停止して `/data` 全体の整合バックアップを取得してください。認識済み Feeder DB は更新前に SQLite のバックアップを作成します。同名の移行バックアップがある場合は上書きせず起動を中止します。

旧 FeedAstrolabe のデータ移行、デスクトップアプリと端末間同期は対象外です。過去の編集履歴は生成されず、記事の版は収集後に記録されます。SNS の収集完全性は保証されません。外部 Access 認証と実サービスの長期受入には別途検証が必要です。詳細と残作業は[検証記録](verification-reader-catchup.md)と[残タスク](remaining-port-tasks.md)を参照してください。
