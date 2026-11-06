# Runtime configuration

## `.env` とAPIキー

`cp .env.example .env` で作成し、`chmod 600 .env` で権限を制限してから、ポートや必要なキーを編集します。リポジトリのルートで実行する Docker Compose は `.env` を自動的に読み、同名のシェル環境変数が優先されます。別のファイルを使う場合は `docker compose --env-file /path/to/file ...` を指定します。値に `$` などが含まれる場合は Compose の `.env` の単一引用符で囲み、意図しない変数展開を避けてください。変更は、対象コンテナを `docker compose up -d` で再作成して反映します。Rust のプロセスが直接 `.env` を読み込む方式ではありません。

- `OPENROUTER_API_KEY` と `ORCAROUTER_API_KEY` は移植元と同じ名前です。Web と worker へ渡されますが、AI プロバイダー機能はまだ未実装で、この値だけでは有効になりません。
- `TUNNEL_TOKEN` は `compose.tunnel-env.yml` を使う場合の Cloudflare Tunnel トークンです。既存の `compose.tunnel.yml` では代わりに `FEEDER_TUNNEL_TOKEN_FILE` を使います。
- Feeder の公開 API にアクセスするための Bearer キーは「集める」画面で発行します。その値をクライアント側の `.env` や環境変数に保管できます。外部 AI のキーや `TUNNEL_TOKEN` を Feeder API の認証に使うことはできません。

`.env` は Git とイメージのビルド対象から除外されていますが、暗号化されません。利用者のみが読める権限で保管してください。環境変数方式の値は権限を持つ利用者がコンテナ設定や `docker compose config` で参照できるため、出力を共有しないでください。より限定したトークンの受け渡しには既存のファイル方式を使えます。設定エクスポートはこれらのキーを含みません。

## アプリの設定

- `FEEDER_DB` selects the SQLite file (default `data/feeder.sqlite3`). Web and worker must use the same persistent file.
- `FEEDER_BIND` selects the Web listening address (default `127.0.0.1:8080`).
- `FEEDER_ALLOWED_HOSTS` optionally lists trusted HTTP Host authorities, separated by commas. For the container setup use `localhost:8080,127.0.0.1:8080,web:8080`. Include the port used by clients. Wildcards, URLs, credentials, and malformed entries are rejected at startup. When unset, the existing bind-address and loopback checks apply. Forwarded headers do not change the trusted Host or Origin.
- `FEEDER_DEMO=1` opts into sample articles when the database is empty.

Run `feeder` for Web, `feeder worker` for continuous collection, or `feeder worker --once` for one pass. SIGTERM and SIGINT stop new worker jobs and let the current fetch finish; Web drains active requests. SQLite connections wait up to five seconds for database locks. Container publication should remain loopback-only unless an authenticated access boundary is added.

- `FEEDER_ACCESS_MODE`: Webは `local`（既定）または `tunnel`。localはloopback/localhost/webのHostへ制限します。tunnelは未公開originとCloudflare Access/Protect with Accessを前提とします。
- `FEEDER_PUBLIC_ORIGIN`: tunnelで必須の正確なHTTPS origin（末尾スラッシュなし）。転送ヘッダーから推測しません。

詳細と公開構成の検証は[アクセス制御](api-access-control.md)を参照してください。workerはブラウザ認証の設定を必要としません。
