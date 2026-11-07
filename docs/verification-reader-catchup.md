# Reader キャッチアップの統合検証

実施: 2026-11-08（JST）。OpenSpec `reader-catchup-foundation`、`reader-catchup`、`collection-reliability` の統合受入。Astra の先行計画を親が仕様化し、Terra がバックエンド、Sol が画面の初期実装、Luna が合成ブラウザー検証を担当。利用上限で中断した画面・検証を親が完成し、以下を実測した。

## 成功した検証

- `cargo test --locked --manifest-path backend/Cargo.toml`: **106件成功**。記事版・関連判定・選定順位・初回／復旧除外・ミュート・同一ホスト上限・5,001候補・関連展開上限・確認の競合／Undo・別SQLite接続の設定競合・収集リース／待機／継続カーソルを含む。
- `cargo clippy --locked --manifest-path backend/Cargo.toml --all-targets -- -D warnings`、Rust整形、React TypeScript/Viteビルド、3つのOpenSpec strict検証が成功。
- `extensions::tests`: v9の全既存テーブルの型付きセル値を比較して移行による変更がないことを確認。v9退避ファイル、v10再起動、改訂・関連除外・関心・確認・runtimeを含む復元、外部キー削除連動、未知schemaの無変更拒否を確認。既知の旧版移行テストも通過。
- `auth::tests::feature_routes_enforce_browser_boundary`: 新しいGETと書込みを含め、Host／Origin／セッション／CSRFを確認。public bearer tokenだけでは内部の書込みを許可しない。
- `node scripts/reader-catchup-browser-smoke.mjs`: 独立した一時DBと専用serverで、API契約・決定的順位・source cap・版更新後の再提示・確認／取り消し・履歴／関連表示・連続した関心設定保存・409／503時の表示を確認。記事詳細の訪問は1回だけ、戻る／進むは追加訪問を発生させない。PC 1440px／モバイル390px、light／dark、キーボード・ポインター・タッチと横はみ出しを検証。
- `node scripts/reader-expansion-browser-smoke.mjs`: 既存の共有検索、あとで読む、ミュート、ダイジェスト、Feedly型行表示などの回帰が成功。
- Linux `backend-test` イメージで同じ **106件成功**。候補イメージ `feeder:catchup-candidate-20261108` でキャッチアップのブラウザー受入も成功。
- Linux Composeの独立生成プロジェクトで `container-smoke.py` が成功。非root、read-only、capabilities、秘密の合成canary、Host／Origin、状態保持、設定、既存詳細UI・中クリック既読・履歴復元、完全volumeバックアップ／復元、workerの正常停止を確認。関心設定と関連除外の保存・復元も確認した。

## 検証データと環境

実記事・実購読・秘密をfixtureへ持ち込んでいない。通常の実行は `frontend/dist` と `backend/target/debug/feeder` を使用する。`FEEDER_SMOKE_BINARY` でLinuxサーバー用ラッパー、`FEEDER_SMOKE_PYTHON` で合成SQLite操作用ラッパーを指定できる。Linux検証ではDBを操作するプロセスもLinux側に揃えた。macOS側のSQLiteからDocker bind mountの稼働DBを書き換える検証はファイルキャッシュの反映が一致せず、実運用で使うLinux named volumeの検証と混同しない。fixtureの観測時刻はサーバーのUTC時刻を使う。

ブラウザーの外部通信は遮断し、任意アイキャッチ取得は404の合成応答に置換。スクリーンショットは一時領域に保存して目視確認した。検証用server、container、volume、DBは検証終了時に片付けた。CIへ独立キャッチアップ受入を追加したが、GitHub上のCIはこの作業では実行していない。

## 制限と次段階

- 既存記事の版・関連索引は次回収集から記録する。配信元で過去に行われた編集は復元せず、履歴は最新20版。初回／復旧取得を新着候補として捏造しない。
- 明示的な関心語、優先フィード、更新と期間内の未読から選ぶ。真偽や社会的重要性の判定、個人別の暗黙プロファイルではない。共有Readerの設定・確認状態である。
- 候補評価は5,000件、関連展開は100回、関連記事の索引分岐ごとは500件。制限に達した場合は部分結果を明示する。同じ配信ホストは最大2件であり、独立した取材元の数を示さない。
- SNS実サービスの認証不要APIによる連続収集、長期の大量データ性能、実ドメインのAccess認証受入は未実施。同一ホストの通信制限はプロセス内のみ。取得完全性を保証しない。
- トピック増減統計、ニュースレター、外部RSSアダプター、外部通知、任意の翻訳・要約は次段階。[残タスク](remaining-port-tasks.md)を参照。
