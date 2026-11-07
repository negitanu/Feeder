# Linux コンテナでの実行

Feeder の正式な実行環境は Linux コンテナです。ネイティブの Mac/Windows アプリは提供しません。Docker Engine と Compose v2 を用意し、このリポジトリのルートで操作します。現在の Web/worker は移植途中の実装で、既存 FeedAstrolabe の本番置換は全体受入後です。

## 起動

```sh
# 既定値は .env なしでも動作する。ポート等を変える場合のみ編集する。
cp .env.example .env
docker compose config --quiet
docker compose build --pull
docker compose up -d --wait
docker compose ps
```

`http://127.0.0.1:8080` を開きます。React の静的ファイルは Rust の Web プロセスから配信します。ホストに Rust/Node.js をインストールする必要はありません。イメージ内も runtime にはコンパイラや Node.js を含めません。

初期状態は空ライブラリです。「設定」の購読追加から購読します。デモ用の生成記事が必要な使い捨て環境のみ `FEEDER_DEMO=1` を設定してください。Web と worker は同じ `/data/feeder.sqlite3` を使用し、プロジェクト専用 named volume `feeder_feeder-data` に永続化します。既存 FeedAstrolabe のボリュームは使用しません。

## Web と worker

Web の health が成功してから worker が起動します。「設定」の自動収集を有効にすると、worker が期限を迎えた購読を更新します。新規環境の初期状態は有効、更新間隔は900秒です。既存環境で明示的に無効にした設定は保持されます。worker は30秒ごとに確認し、一巡で最大100購読を処理します。購読の停止は記事を保持し、無効化は新しい定期取得を止めます。

```sh
docker compose logs --tail 100 web worker
docker compose stop
docker compose start --wait
```

SIGTERM/SIGINT で新規処理を止め、上限付きの実行中処理を終えてから終了します。停止猶予は30秒です。Web と worker は同じ UID/GID 10001、read-only root filesystem、capabilities 全削除、no-new-privileges、容量制限付き tmpfs で動きます。データの書込み先は専用 `/data` です。

## ポートと HTTP の許可

`FEEDER_PORT` でホスト側ポートを変えられます。既定の公開先はホストの `127.0.0.1` のみです。コンテナ内は `0.0.0.0:8080` で待受け、許可された Host と同一 authority の Origin を検査します。forwarded ヘッダーでこの検査を迂回できません。

`FEEDER_ALLOWED_HOSTS` を明示指定する場合はクライアントのポートを含め、health 用 `127.0.0.1:8080` も残してください。既定値は Compose が `FEEDER_PORT` に合わせて計算します。[実行設定](runtime-config.md)を参照してください。

標準 Compose はホストローカルからの利用を前提とします。外部向けの Cloudflare Tunnel 構成例はありますが、実ドメインでの認証受入は未実施です。旧 renderer/observer は未移植です。

## 更新と永続データ

```sh
docker compose build --pull
docker compose up -d --wait
```

再作成しても named volume は保持されます。既知の Feeder v1〜v8 DB は移行前に `/data` 内へバックアップを作り、未知のスキーマを拒否します。旧 FeedAstrolabe DB はそのままマウントしないでください。

DB 変更を含む更新前には Web と worker の両方を止め、一貫したデータ一式を退避・復元確認します。Feeder は今後の根拠ファイル等も含めて保全する方針です。例として、停止中の `/data` をホストへ取得できます。

```sh
mkdir -p backups
docker compose stop
docker compose run --rm --no-deps -T --entrypoint tar web -C /data -czf - . > backups/feeder-data.tar.gz
# アーカイブの内容確認。実際の復元は別プロジェクトの新規volumeで試験する。
tar -tzf backups/feeder-data.tar.gz
docker compose start --wait
```

通常の `docker compose down` はコンテナを削除しても volume を残します。運用データへ `down --volumes` を実行しないでください。バックアップと切替の受入は [データ移行計画](data-cutover.md)に従います。上のtar手順は実環境復旧を完了させるものではなく、更新前に別volumeで復元と整合性を確認する必要があります。

## Linux での自動検証

```sh
docker build --target backend-test .
docker compose --env-file /dev/null build
python3 scripts/container-smoke.py
```

最後のスクリプトは UUID を使った新しい専用プロジェクトを作り、生成データで health、UI/API、Host/Origin、非root/read-only、再作成後の状態・設定保持、別volumeでのバックアップ復元、worker正常終了を検証します。終了時にその実行が作ったテストコンテナとテストvolumeだけを削除します。通常の `feeder` 環境は操作しません。

ブラウザも接続する場合は、検証用Playwright/Chromiumを用意して `FEEDER_BROWSER_SMOKE=1 python3 scripts/container-smoke.py` を実行します。アプリ本体はLinuxコンテナ内で実行されます。

DB v5 はライブラリの配置を保存します。v4 からの更新前には `feeder.v4-backup.sqlite3` を作ります。更新時は Web/worker の両方を停止し、以前のイメージへ戻す際は停止したまま更新前バックアップを別volumeへ復元して検証してください。v5 DB を以前のイメージへ直接渡さないでください。

DB v6 は共有の表示タイムゾーンと初期表示を追加します。v5 から更新する際は `feeder.v5-backup.sqlite3` を先に作成します。Web／workerを停止して更新し、以前のイメージへ戻す場合は対応する更新前バックアップを別volumeへ復元して確認してください。設定エクスポートJSONはDBのバックアップとして使えません。

DB v7 はAPIキーのハッシュ・期限・失効とAPI回数上限を追加します。v6から更新する前に `feeder.v6-backup.sqlite3` を作ります。セッションはメモリー上のため、Web再起動後にはブラウザ操作の再試行が必要な場合があります。

DB v8 はフィードごとの収集間隔を追加します。認識済みの Feeder v7 から更新する際、`feeder.v7-backup.sqlite3` を作ります。未設定のフィードは全体設定を継承します。

外部公開には[保護されたTunnel構成](api-access-control.md)を参照してください。`.env` にTunnel設定を行い、`docker compose up -d` で起動します。設定の構文検査だけで実際のAccessポリシー・JWT検証が完了したとは扱いません。実ドメインの受入は別途必要です。

DB v9 は共有検索、公開SNS、あとで読む、ミュート、収集診断の追加保存先を作成します。認識済みv8からの更新前に `feeder.v8-backup.sqlite3` を保存し、既読・スター・履歴・購読順序・既存設定を保持します。完全な復旧には `/data` 全体の整合バックアップを用いてください。最新の実施結果と制限は [Reader拡張の検証](verification-reader-expansion.md) を参照してください。
