## ADDED Requirements
### Requirement: SNS購読の公開API
公開API SHALL write権限を持つクライアントが公開SNSアカウントを購読できるようにする。
#### Scenario: 不正な接続先
- **WHEN** 内部ネットワークや不正なアカウントが指定される
- **THEN** 安全な既存検証により購読を作成しない。
