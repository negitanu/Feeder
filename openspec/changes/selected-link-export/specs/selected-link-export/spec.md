## ADDED Requirements
### Requirement: 選択記事リンクの書き出し
Reader SHALL 選択した読み込み済み記事のタイトルと安全なHTTP(S)リンクをテキストで保存する。
#### Scenario: 選択した記事を書き出す
- **WHEN** 利用者がリンク書き出しを選ぶ
- **THEN** 有効なリンクを一覧順に保存し、無効なリンクの除外数を伝え、記事状態と選択を維持する。
