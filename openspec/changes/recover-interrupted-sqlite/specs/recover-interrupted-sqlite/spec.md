## ADDED Requirements
### Requirement: 中断されたDBの安全な復旧
Feeder SHALL 書込み中断で残ったjournalを、コピーの整合性と既知スキーマを検証したうえで復旧できる。
#### Scenario: 既知スキーマの中断
- **WHEN** 読み取り専用の検査がhot journalにより失敗する
- **THEN** コピーを検証してからSQLiteのロールバックで起動する。
#### Scenario: 未知スキーマ
- **WHEN** 復旧コピーのスキーマが未知である
- **THEN** 元のDBとjournalを変更せず拒否する。
