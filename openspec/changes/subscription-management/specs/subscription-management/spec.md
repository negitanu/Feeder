## ADDED Requirements
### Requirement: 購読状態の検索と表示
Feeder SHALL provide the following bounded reader subscription behavior.
利用者は購読をタイトル、URL、フォルダー名で検索し、停止状態、最終確認日時、エラーを確認できる。
#### Scenario: 状態を検索する
- **WHEN** 検索語を入力する
- **THEN** 一致する購読とその状態が表示される
### Requirement: 安全な操作
Feeder SHALL provide the following bounded reader subscription behavior.
利用者は購読を編集、停止、再開、または更新できる。停止中は更新できない。削除操作は提供しない。
#### Scenario: 処理中の操作
- **WHEN** API 操作が進行中である
- **THEN** 他の変更操作を開始できず、終了時に親へ再取得を通知する
### Requirement: 制限された一括操作
Feeder SHALL provide the following bounded reader subscription behavior.
利用者が明示的に選んだ最大100件に対して停止または再開を逐次実行し、成功と失敗を個別に報告する。
#### Scenario: 一部が失敗する
- **WHEN** 一括操作の一部が失敗する
- **THEN** 成功数と失敗した購読を示し、残りの対象も処理する
