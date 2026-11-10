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

更新前には、稼働中のイメージと `/data` 全体を保全します。SQLiteを含むため、Webとworkerを停止してから取得します。次の操作はリポジトリのルートで実行し、失敗した場合は続けず原因を確認してください。実データを含むバックアップはGitに追加せず、アクセスを制限した場所に保管します。

```sh
# イメージ名は更新ごとに変え、前回の退避を上書きしない。
docker tag feeder:local feeder:before-update-YYYYMMDD
mkdir -p backups
# mktempが新規ディレクトリを作るので既存バックアップを上書きしない。
backup_dir=$(mktemp -d backups/feeder-update-XXXXXXXX)
docker compose stop worker web
docker compose run --rm --no-deps -T --entrypoint tar web -C /data -czf - . > "$backup_dir/feeder-data.tar.gz"
python3 scripts/verify-backup.py "$backup_dir/feeder-data.tar.gz"
```

検証が失敗したアーカイブは復元に使わず、停止状態、ディスク空き容量、アーカイブを確認して取得し直します。取得を中断して通常運転へ戻す場合、変更前のイメージ・元volumeのまま `docker compose start --wait web worker` を実行し、`docker compose ps` と画面で稼働を確認します。元々停止していたサービスは起動せず元の状態に戻してください。検証はgzipのCRC、危険なパス・リンク・重複、SQLite整合性と外部キー、主要テーブルを確認します。非空のWAL・SHM・rollback journalは拒否します。**全ファイルの意味やバックアップの真正性、完全なスキーマ互換性を保証するものではありません**。信頼できる保存元を使い、次の別volumeでの復元も確認してください。設定JSONやOPMLは完全バックアップの代わりにはなりません。

### 別volumeでの復元確認

以下は新規の検証専用volumeを作る例です。`backup_dir` は上記の値を使用します。指定名のvolumeが存在する場合は中断します。信頼でき、上記検証に成功したアーカイブだけを使ってください。`--no-same-owner` で元の任意UIDを復元せず、Feeder用の所有者を設定します。

```sh
(
set -eu
restore_volume="feeder-restore-check-$(date +%Y%m%d%H%M%S)"
if docker volume inspect "$restore_volume" >/dev/null 2>&1; then
  echo '同名volumeが既にあります。名前を変更して中断してください。' >&2
  exit 1
fi
docker volume create "$restore_volume"
docker run --rm -i --user 0 --entrypoint /bin/sh -v "$restore_volume:/data" feeder:local \
  -c 'tar --no-same-owner --no-same-permissions -C /data -xzf - && chown -R 10001:10001 /data' < "$backup_dir/feeder-data.tar.gz"
# 外部収集を行わないWebのみを、一時的な別ポートで起動する。
docker run --rm -d --name feeder-restore-check -p 127.0.0.1:18080:8080 \
  -e FEEDER_BIND=0.0.0.0:8080 -e FEEDER_DB=/data/feeder.sqlite3 \
  -e FEEDER_ALLOWED_HOSTS=127.0.0.1:18080,127.0.0.1:8080 \
  -v "$restore_volume:/data" feeder:local
curl --fail http://127.0.0.1:18080/healthz
# http://127.0.0.1:18080 を開き、購読・記事・既読・スター・設定を確認する。
docker stop feeder-restore-check
# 確認完了後、名前を再確認してこの検証専用volumeだけを削除する。
docker volume rm "$restore_volume"
)
```

別ポート・コンテナ名が既に使われている場合は適切な空きを選んでください。起動中に検証用DBがスキーマ更新されても元volumeには影響しません。未確認のバックアップを実volumeに上書きしないでください。検証を中断した場合は、検証用コンテナとvolumeを確認してから片付けます。

### 更新と切り戻し

復元確認後、停止した元環境を更新します。

```sh
docker compose build --pull
docker compose up -d --wait
docker compose ps
```

更新に失敗した場合は、Web／workerを停止し、更新後の `/data` も別にバックアップします。上記と同様に**新しい復旧専用volume**へ更新前バックアップを復元し、`feeder:before-update-YYYYMMDD` で別ポート起動して確認します。元のvolumeを削除・上書きせず、Composeのvolume指定とイメージを復旧用に切り替えたうえで起動します。旧版が新スキーマのDBを直接開かないようにしてください。更新後に作成された記事や状態は更新前バックアップに含まれないため、切り戻し前に保全・確認が必要です。

通常の `docker compose down` はvolumeを保持します。運用データに `down --volumes` を実行しないでください。復旧前には [データ移行計画](data-cutover.md) も確認してください。

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

## キャッチアップと記事履歴（schema v10）

v10 は記事の観測した版・関連索引・関心設定・確認状態と、収集の継続位置／再試行期限を追加します。認識済み v9 は更新前に `feeder.v9-backup.sqlite3` へ退避します。過去の本文履歴は捏造せず、既存記事の既読・スター・訪問・順序・設定を保持します。

配備前には Web/worker を停止し、`/data` 全体の整合バックアップと旧イメージを保持してください。v9 バイナリへ戻す場合は、v10 DB を開かせず更新直前のバックアップを別の復旧用 volume に復元します。v10 で追加された関心・確認・履歴も、通常の完全DB／volumeバックアップに含まれます。設定JSON／OPMLのエクスポートだけでは復旧できません。

[合成データによる検証記録](verification-reader-catchup.md)と[キャッチアップの仕様](reader-catchup.md)を参照してください。
