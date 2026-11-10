## ADDED Requirements
### Requirement: 取得元への識別
RSSなどの外部取得要求はアプリ名とバージョンをUser-Agentで送らなければならない。
#### Scenario: RSS取得
- **WHEN** 安全と判定された配信元にHTTP要求を送る
- **THEN** FeederのUser-Agentを送り、既存の安全な接続と取得上限を維持する。
