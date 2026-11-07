## ADDED Requirements
### Requirement: 公開アカウント購読
Feeder SHALL allow public subscriptions: 利用者は Mastodon のサーバーとアカウントまたは Bluesky のアカウントを登録できる。未対応 provider、不正入力、非公開アクセスを拒否する。
#### Scenario: 公開投稿を収集
- **WHEN** 公開アカウントを追加し更新する
- **THEN** 投稿は既存記事として保存され既読・スター・検索・フォルダに対応する
#### Scenario: 非公開投稿
- **WHEN** サービスが非公開投稿を返す
- **THEN** 保存しない
### Requirement: 安全な取得と保全
Feeder MUST enforce safe bounded requests: すべての外部リクエストは既存の SSRF 保護、上限とタイムアウトに従う。SNS フィード URL の通常編集は拒否し、削除時は既存の購読削除規則を適用する。
#### Scenario: 失敗
- **WHEN** 外部取得や検証が失敗する
- **THEN** 既存記事を保持し制限されたエラーを返す
