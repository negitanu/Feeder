# 収集・設定・保全の検証（2026-11-09）

- 収集エラーは安全な固定診断コードを追加し、配信元HTTP、DNS、接続、時間超過、受信、解析、サイズ制限などを案内する。既存のRetry-Afterと待機は維持する。合成テストと403・429・旧形式・モバイルの画面確認を実施。
- 設定読み込みは表示・収集・API回数上限・フォルダー名の追加に対応。プレビュー時の状態と異なれば409で一括変更しない。実エクスポートJSONを使う合成DBのブラウザー試験で取消、競合、適用、不正値、モバイルを確認。購読はOPMLを使う。
- 継続読込はPC・モバイル、light・darkでObserverなし、503時の再試行と選択保持、409時の明示再読込をmacOS Chromeで確認。Linuxブラウザーの今回再実行は含まない。詳細は verification-reader-stable-continuation.md。
- Linux候補イメージでRust 111テスト、合成DBの独立volumeへのバックアップ・復元と起動確認に成功。バックアップ検証器の合成テスト6件が成功。再起動前の実データを停止して取得し、SQLite・外部キー・アーカイブを検証した。実データの別volume復元は今回行っていない。
- Mastodon・Blueskyの実公開APIに一時環境で2回ずつ接続し、記事と継続位置を確認。初回サイクル完走、実際の429/503、長期運用は未確認。詳細は public-social-reader.md。
- npm auditは全依存で0件。RustSec監査でRUSTSEC-2026-0459を検出したためrusqlite 0.41.1 / libsqlite3-sys 0.39.1へ更新し、再監査で該当は検出されなかった。監査時のCargo registry indexは利用不可だったが、RustSec advisory DBは取得済み。
- 実ドメインの未認証 `/` と `/api/session` は403を返した。認証済み利用者の通過やAccess設定の評価は未実施であり、認証境界の完全な受入は未完了。
- 稼働反映: Web・worker停止後、backups/reader-reliability-8hSeeaZk/feeder-data.tar.gz を作成・検証し、新イメージへ更新。旧イメージ feeder:before-reader-reliability-20261109 を保持。Web healthy、worker稼働、同一イメージ、healthzを確認。
