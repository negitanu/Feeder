# Runtime configuration

- `FEEDER_DB` selects the SQLite file (default `data/feeder.sqlite3`). Web and worker must use the same persistent file.
- `FEEDER_BIND` selects the Web listening address (default `127.0.0.1:8080`).
- `FEEDER_ALLOWED_HOSTS` optionally lists trusted HTTP Host authorities, separated by commas. For the container setup use `localhost:8080,127.0.0.1:8080,web:8080`. Include the port used by clients. Wildcards, URLs, credentials, and malformed entries are rejected at startup. When unset, the existing bind-address and loopback checks apply. Forwarded headers do not change the trusted Host or Origin.
- `FEEDER_DEMO=1` opts into sample articles when the database is empty.

Run `feeder` for Web, `feeder worker` for continuous collection, or `feeder worker --once` for one pass. SIGTERM and SIGINT stop new worker jobs and let the current fetch finish; Web drains active requests. SQLite connections wait up to five seconds for database locks. Container publication should remain loopback-only unless an authenticated access boundary is added.

- `FEEDER_ACCESS_MODE`: Webは `local`（既定）または `tunnel`。localはloopback/localhost/webのHostへ制限します。tunnelは未公開originとCloudflare Access/Protect with Accessを前提とします。
- `FEEDER_PUBLIC_ORIGIN`: tunnelで必須の正確なHTTPS origin（末尾スラッシュなし）。転送ヘッダーから推測しません。

詳細と公開構成の検証は[アクセス制御](api-access-control.md)を参照してください。workerはブラウザ認証の設定を必要としません。
