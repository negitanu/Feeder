# Feeder

現在の方針: FeedAstrolabe の旧データの移行・変換は行わず、Web 機能の移植に注力します。Feeder 自身に保存済みのデータの保全・復旧は引き続き必要です。

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

設定を変更する場合は `cp .env.example .env` で `.env` を作成し、必要な値を記入してください。Compose が起動時に読み込みます。Cloudflare Tunnel のトークンや将来の AI 連携用キーも指定できます。設定方法と対応状況は[環境変数とAPIキー](docs/runtime-config.md)を参照してください。

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

フィードの「編集」では、購読先URLを変更できます。URLを変えても既存の記事・既読・スターは維持され、次の収集で新しいURLを検証します。収集中は編集・削除を受け付けないため、終了後に再試行してください。

既知のFeeder v1〜v7 DBは更新前に同じ場所へ `*.vN-backup.sqlite3` を作成します。同名バックアップがある場合は上書きせず起動を拒否します。旧FeedAstrolabeのデータは移行対象外です。

## 自動収集と OPML

「集める」の自動収集で有効／無効と更新間隔（60〜86400秒、既定900秒）を保存します。初期状態は無効です。Compose が Web と worker を一緒に起動します。Web プロセス単体では定期収集しません。

各フィードの「編集」では個別の収集間隔を指定できます。空欄にすると全体設定を使います。

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

「未読」は検索やフォルダーの条件を解除して全体を表示し、ライブラリ全体の未読件数を示します。他の画面での変更はフォーカス復帰時と定期的な再取得で反映されます。記事カードを中クリックすると既読にせず元記事を別タブで開けます。

記事詳細の末尾から前後の記事へ移動できます。対象は詳細を開いた時点で一覧に読み込み済みの記事で、移動した記事は通常どおり既読・閲覧履歴に記録されます。

フォルダー・フィードの並べ替えと幅変更は [ライブラリの配置](docs/sidebar-organization.md)を参照してください。順序は共有DB、幅はブラウザに保存します。

[表示設定・設定エクスポート](docs/reader-settings-export.md)では、初期表示、UTC／日本時間、標準・1行・タイル表示と、安全な設定の持ち出しを扱います。

[アクセス制御とAPIキー](docs/api-access-control.md)に、キーの発行・失効、公開APIのRSS／記事サブセットと、外部公開用の保護されたTunnel構成を記載しています。通常のComposeは引き続きローカル専用です。

## 開発状況

[実装状況](docs/implementation-status.md)と[移植の優先順位](docs/priority-backlog.md)を参照してください。
