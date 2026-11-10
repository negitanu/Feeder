## ADDED Requirements
### Requirement: 有限の大きなRSS収集
収集は8 MiBかつ10,000件以下のRSSを受け入れ、超過は保存せず拒否しなければならない。
#### Scenario: 500件を超えるRSS
- **WHEN** サイズと件数が上限以内のRSSを取得する
- **THEN** 既存記事状態を維持して記事を収集する。
#### Scenario: 上限を超過するRSS
- **WHEN** 本文または項目が上限を超える
- **THEN** 途中までの結果や取得ヘッダーを保存せずエラーとする。
