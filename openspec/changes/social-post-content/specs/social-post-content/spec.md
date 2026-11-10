## ADDED Requirements
### Requirement: SNS投稿本文の独立した制限
SNS収集 SHALL 空文字および256 KiB以下の投稿本文を受け入れ、IDやURLの上限を維持する。
#### Scenario: 長文の投稿
- **WHEN** 公開投稿に2048バイトを超える有効な本文が含まれる
- **THEN** ページ全体を拒否せず投稿を保存する。
#### Scenario: 異常な本文
- **WHEN** 本文が欠落、型不正または上限を超える
- **THEN** ページを拒否して継続位置を進めない。
