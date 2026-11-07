# 公開 SNS 購読

Mastodon の公開サーバー origin とアカウント、または Bluesky のハンドル/DID を登録する。Bluesky は公開 AppView を使用する。認証情報は使用せず、Mastodon の public 投稿だけを保存する。公開 API や匿名 lookup を提供しないサーバーでは登録できない。ブースト・リポストは取り込まない。初回は次回の通常更新で記事を取得する。

購読は通常のフォルダ、既読、スター、検索、更新間隔、停止と削除に従う。削除すると保存記事も削除される。SNS 購読の URL は編集できない。外部取得は通常の SSRF 対策とサイズ・時間制限に従う。秘密や実データを fixture に使わない。

API: `GET /api/social/sources` は `sources` 配列、`POST /api/social/sources` は `{provider,actor,server?}`、`DELETE /api/social/sources/{feed_id}` は 204。Mastodon の `server` は HTTP(S) origin、Bluesky では省略する。書き込みは既存の認証・CSRF 保護が必要。

サービス仕様: https://docs.joinmastodon.org/methods/accounts/ および https://docs.bsky.app/docs/api/app-bsky-feed-get-author-feed
