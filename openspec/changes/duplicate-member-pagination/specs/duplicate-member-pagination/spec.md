## ADDED Requirements
### Requirement: 大量グループの継続
重複表示 SHALL 同一URLの元記事を有限のページで継続表示できる。
#### Scenario: 100件を超える
- **WHEN** 利用者が続きを読み込む
- **THEN** 既に表示した記事を保ち、未表示の古い元記事を表示する。
