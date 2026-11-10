## ADDED Requirements
### Requirement: 安全なアーカイブ検証
検証はアーカイブのパスとSQLite整合性を確認し、本番データを変更してはならない（SHALL NOT）。
#### Scenario: 一貫したバックアップ
- **WHEN** 書き込み停止後のFeederデータを検証する
- **THEN** 危険なパスとDBの不整合を拒否し、結果を表示する。
