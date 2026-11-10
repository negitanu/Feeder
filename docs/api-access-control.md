# アクセス制御とAPIキー

「設定」の下部にある「APIキー」から、名前・権限・有効期間を指定して発行します。既定は読み取りのみ・90日、有効期間は1〜365日です。全文は発行した画面に一度だけ表示し、「キーを隠す」や画面移動で消えます。再表示できないため、必要な場合は安全な場所へ保存してください。ブラウザの保存領域や設定エクスポートへは記録しません。

発行済み一覧は名前、先頭部分、権限、期限などの情報だけを表示します。「失効」を実行すると以後の呼び出しを拒否します。DBにはSHA-256ハッシュだけを保存します。キーごとの1分間の上限は既定120回、1〜1000回へ変更できます。DBバックアップにはハッシュや利用記録が入るため、公開しないでください。

## ブラウザと公開API

ブラウザ内部の `/api/*` の変更操作は、HttpOnly/SameSite=StrictのセッションCookieと `X-Feeder-CSRF` を組み合わせて保護します。セッションは最大12時間で、サーバー再起動でも無効になります。拒否された操作を自動で再実行せず、次の明示的な操作で確認情報を再取得します。

外部クライアントは `/api/v1/*` へ `Authorization: Bearer <キー>` を送ります。ブラウザCookieでは認証できず、writeキーもキーの発行・管理には使えません。キーをURLへ入れる方式はありません。読み取りキーによる変更は403、期限切れ・失効・無効キーは401、回数上限は429です。401には `WWW-Authenticate: Bearer`、429には `Retry-After` が付きます。

実装済みの範囲:

- `GET /api/v1/feeds` と `GET /api/v1/feeds/{id}`: RSS購読の一覧・詳細。
- `POST /api/v1/feeds`: `{url, folder?, source_type?: "rss"}`。安全な初回取得に成功してから購読・フォルダー・記事を一括保存し201を返します。
- `PATCH /api/v1/feeds/{id}`: `title`、`folder`、`enabled`、RSSの `url`、`poll_interval` の部分更新。収集間隔は60〜86400秒の整数、`null` は全体設定の継承です。
- `DELETE /api/v1/feeds/{id}` と `POST /api/v1/feeds/{id}/refresh`: 削除と安全な手動取得。停止中の購読の手動取得は409です。
- `POST /api/v1/social/sources`: writeキーで `{"provider":"mastodon","actor":"name","server":"https://example.org"}` または `{"provider":"bluesky","actor":"name.bsky.social"}` を指定し、公開SNS購読を作成します。正常時は201と購読情報を返し、収集は通常workerが実行します。
- `GET /api/v1/articles` と `GET /api/v1/articles/{id}`: 記事の一覧・詳細。`PATCH /api/v1/articles/{id}` は `{is_read?, is_starred?}` のJSON booleanだけを受け付けます。閲覧履歴は作りません。

記事一覧はID降順で、`limit` 1〜100（既定30）、`feed`、`is_read`、`is_starred`、`q`（最大200文字の文字列検索）、`source_type=rss`／`mastodon`／`bluesky`、`before_id` または `after_id` を使えます。`after_id=0` からの取得はID昇順で、`has_more`、`next_before_id`、`next_after_id` により継続できます。重複・未知・矛盾する条件は拒否します。応答は `data`、エラーは `error: {code, message}` にまとめます。

URLは公開HTTP(S)形式で、認証情報やローカルアドレスを含められません。変更時にはネットワーク取得をせず、既存のフィード・記事IDと状態を維持し、前URLのETag・Last-Modified・最終確認日時・収集エラーを消します。次の収集時にDNSを検証します。正規化後のURLが同じ場合は既存の収集状態を維持します。重複URLまたは収集中の編集・削除は409で拒否し、同時に指定した変更も保存しません。

本文はFeederのプレーンテキストです。公開日時は現段階ではFeederに保存した日時表現です。フォルダーなしはnullです。SNSの情報源と記事は `source_type` に `mastodon` または `bluesky` を返し、記事一覧を種別で絞れます。ページ観測・研究系API、旧版の全フィールドは対応していません。旧版API全体の互換性を完了したとは扱いません。旧データ・既存キーの移行は対象外です。

## 公開構成

通常のComposeは `local` モードで、ホストのループバックだけへポートを公開します。Host制限はDNS再バインド等の保護であり、利用者の認証ではありません。ポートを外部向けに変更してローカルUIを公開しないでください。

外部向けは `.env` に公開設定と `TUNNEL_TOKEN` を設定し、`docker compose up -d --build` で起動します。`FEEDER_ACCESS_MODE=tunnel` を設定します。Webポートはlocalhostにのみ公開されます。公開ホストのAccess許可ポリシーと **Protect with Access** によるJWT検証を事前に設定します。Tunnel tokenだけで閲覧者が認証されるわけではありません。アプリはJWTを独自検証せず、認証済みのTunnel接続と信頼されたコンテナネットワークを境界とします。

`.env` の `FEEDER_PUBLIC_ORIGIN` に末尾スラッシュのない正確なHTTPS origin、`FEEDER_PUBLIC_HOST` に同じauthority、`FEEDER_TUNNEL_IMAGE` に固定バージョンまたはdigestの公式イメージ、`TUNNEL_TOKEN` にTunnelトークンを指定します。`FEEDER_ACCESS_MODE=tunnel` も指定してください。.envは権限を制限し、値をリポジトリへ入れないでください。

公開前に、設定を構文検査します。この検査はtoken本文を表示しません。通常のローカル環境と同じvolumeを使うため、切替前にWeb/workerを停止しバックアップしてください。

```sh
python3 scripts/check-tunnel-config.py
docker compose up -d --build
```

トークンはコネクターの環境変数になり、権限を持つ利用者にはコンテナ設定や `docker compose config` で見えます。`.env` の権限を制限し、その出力を共有しないでください。`.env` 変更後は対象コンテナを再作成してください。

公開後の受入は実ドメインで、未ログイン・非許可利用者の拒否、許可利用者の画面とCSRF操作、Service Auth policyを通したサービス認証＋アプリBearer、originへ直接入れないこと、healthと終了処理を確認します。この実装作業では実トークンを用意せず、Tunnel接続・外部公開を実行していません。

参考: [Cloudflareの公開アプリ保護](https://developers.cloudflare.com/cloudflare-one/access-controls/applications/http-apps/self-hosted-public-app/)、[token-file実行パラメーター](https://developers.cloudflare.com/cloudflare-one/networks/connectors/cloudflare-tunnel/configure-tunnels/run-parameters/)、[Composeのマージとreset](https://docs.docker.com/reference/compose-file/merge/)、[RustCrypto SHA-2](https://docs.rs/sha2/latest/sha2/)。

Cloudflareの公開ホストのサービスURLには `http://web:8080` を設定します。接続ログにTCP 7844の警告が出る場合、送信先へのTCP 7844をネットワーク側で許可するとHTTP/2へのフォールバックを利用できます。UDP 7844のQUIC接続が成功していればTunnelはQUICで動作します。
