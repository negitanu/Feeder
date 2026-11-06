# Feeder

FeedAstrolabe の Web 版を Rust バックエンドと React/TypeScript 画面で再実装するための、新しい独立リポジトリです。Linux コンテナ上の読書・購読・収集基盤を段階的に実装しています。既存 Web の全面移植と本番切替は未完了です。

- [段階的移行計画](docs/migration-plan.md)
- [移行元の仕様と機能の対応表](docs/source-inventory.md)
- [データ移行・切替・復旧の検討](docs/data-cutover.md)
- [OpenSpec 提案](openspec/changes/plan-web-migration/proposal.md)・[設計](openspec/changes/plan-web-migration/design.md)・[タスク](openspec/changes/plan-web-migration/tasks.md)
- [移行元の参照スナップショット](reference/feedastrolabe/README.md)

旧ローカルアプリ、端末間同期、旧ローカルのデータ移行は対象外です。既存 Web のデータと運用の保全は移行対象です。参照資料は実装完了を意味しません。

## Linux コンテナで起動

Docker Engine と Compose v2 を用意し、リポジトリのルートで実行します。Rust/Node.js のホストへの導入は不要です。

```sh
docker compose config --quiet
docker compose build --pull
docker compose up -d --wait
```

`http://127.0.0.1:8080` を開きます。Web と定期収集 worker が起動し、専用 named volume の `/data/feeder.sqlite3` を共有します。初期状態は空ライブラリ、自動収集は無効です。生成記事を使う検証環境のみ `FEEDER_DEMO=1` を指定してください。

[Linux 配備・更新・バックアップ手順](docs/deployment.md)に、ポート変更、ログ、停止、データ保全と検証方法を記載しています。標準構成はホストのループバックからの利用を前提とします。保護されたTunnel用の構成例もありますが、実ドメインでの外部認証の受入確認は未実施です。

## 検証

```sh
docker build --target backend-test .
python3 scripts/container-smoke.py
```

コンテナ検証スクリプトは、ビルド済みイメージを使う新しい専用プロジェクトで生成データを検証し、そのプロジェクトだけを片付けます。Linux CI には Rust/React の検査、ブラウザ回帰、Docker ビルド、Compose 受入を定義しています。

## フィードを登録する

「集める」で公開サイトまたは RSS/Atom のURLを入力し、候補を選んで内容をプレビューします。「このフィードを登録」で確定するまで購読は追加されません。プレビュー確認は同じセッション内で15分間有効で、再起動・期限切れ・失敗後は再プレビューしてください。左ペインのフォルダー名で記事を表示し、矢印で開閉します。フィードの右クリック／「⋯」から編集、移動、停止／再開、手動更新、削除ができます。停止は記事を残し、削除はその購読の記事を削除します。自動収集は下記の設定と独立workerを使用します。

既知のFeeder v1/v2/v3/v4/v5/v6 DBは更新前に同じ場所の `*.v1-backup.sqlite3` / `*.v2-backup.sqlite3` / `*.v3-backup.sqlite3` / `*.v4-backup.sqlite3` / `*.v5-backup.sqlite3` / `*.v6-backup.sqlite3` へバックアップを作成します。同名バックアップがある場合は上書きせず起動を拒否します。旧FeedAstrolabe DBは直接利用せず、[読取専用検査](docs/legacy-db-inspection.md)で移行準備を進めます。

## 自動収集と OPML

「集める」の自動収集で有効／無効と更新間隔（60〜86400秒、既定900秒）を保存します。初期状態は無効です。Compose が Web と worker を一緒に起動します。Web プロセス単体では定期収集しません。

```sh
docker compose logs --tail 100 worker
# 専用の検証環境で一巡だけ実行する場合
docker compose run --rm --no-deps worker worker --once
```

停止中の購読と、直前に確認済みの購読はスキップします。全体無効化で新規取得を止めますが、実行中の上限付き取得は完了する場合があります。workerは30秒ごとに期限を確認し、一巡で最大100購読を処理します。手動更新とworkerはDB上のリースで同時取得を防ぎます。

OPMLファイルは1 MiBまで、1000 outline・16階層までです。ファイル選択で内容を確認し、「確認して読み込む」で追加します。既存・重複・無効を区別し、新規購読は停止状態で追加されます。再開は左ペインの「⋯」から行います。書き出しは購読とフォルダーを含み、記事や認証キーは含みません。[形式と制限](docs/opml.md)。

ブラウザ回帰は専用生成DBで `browser-smoke.mjs` → `tests/seed_collection_fixture.py DB_PATH` → `collection-browser-smoke.mjs` → `onboarding-browser-smoke.mjs` の順に実行します。公開接続を使う検証は明示的に `FEEDER_LIVE_FEED=https://go.dev/blog/feed.atom node scripts/live-discovery-smoke.mjs` を実行してください。

## 読書履歴と一括操作

「最近読んだ記事」、詳細検索、現在の条件全体を対象とする「全て既読」、最大100件の選択状態変更を実装しています。履歴は実際の詳細閲覧で記録し、一括既読やスター変更では増やしません。[検索条件・履歴・一括操作](docs/reader-productivity.md)を参照してください。

フォルダー・フィードの並べ替えと幅変更は [ライブラリの配置](docs/sidebar-organization.md)を参照してください。順序は共有DB、幅はブラウザに保存します。

[表示設定・設定エクスポート](docs/reader-settings-export.md)では、初期表示、UTC／日本時間、標準・1行・タイル表示と、安全な設定の持ち出しを扱います。

[アクセス制御とAPIキー](docs/api-access-control.md)に、キーの発行・失効、公開APIのRSS／記事サブセットと、外部公開用の保護されたTunnel構成を記載しています。通常のComposeは引き続きローカル専用です。
