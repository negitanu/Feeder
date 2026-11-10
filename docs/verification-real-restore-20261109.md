# 実バックアップの隔離復元確認（2026-11-09）

`backups/reader-quality-0aqS3ISr/feeder-data.tar.gz` を検証し、新規の専用Docker volumeに復元した。workerを起動せず、隔離Webのhealthを確認し停止後、バックアップDBと復元DBの全26テーブルの件数、既読・スター数、共有の表示・収集設定、SQLite integrity_check・foreign_key_checkを照合し一致を確認した。全記事の本文や画面上の全項目を比較したわけではない。元volumeは変更していない。確認後に検証用コンテナ・volume・一時コピーを削除した。

## 中断されたjournalの復旧と再発防止

Docker再起動後、未処理のrollback journalにより読み取り専用のスキーマ検査が失敗する事象が判明。停止中のDBとjournalを `backups/recovery-incident-20261109/data-with-journal.tar.gz` に保全し、コピー上でSQLiteのrollback・整合性と外部キーを確認した後に復旧した。元のDBとjournalはvolume内の `.pre-recovery-20261109` ファイルにも残した。

起動時に同じエラーが発生した場合、元DBに触れる前に安全な一時コピーでrollbackし、既知スキーマと整合性を確認する処理を追加。合成DBの強制終了テストでは、既知スキーマの復旧と未知スキーマのDB・journalが無変更で拒否されることを確認。実際に残った中断バックアップを別volumeへ復元した検証でも、修正後のWebが起動し、手動復旧したコピーと全26テーブルの件数および既読・スター数が一致した。すべての行の値の比較ではない。検証用volumeは削除した。
