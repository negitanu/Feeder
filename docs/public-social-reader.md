# 公開 SNS 購読

Mastodon の公開サーバー origin とアカウント、または Bluesky のハンドル/DID を登録する。Bluesky は公開 AppView を使用する。認証情報は使用せず、Mastodon の public 投稿だけを保存する。公開 API や匿名 lookup を提供しないサーバーでは登録できない。ブースト・リポストは取り込まない。初回は次回の通常更新で記事を取得する。

購読は通常のフォルダ、既読、スター、検索、更新間隔、停止と削除に従う。削除すると保存記事も削除される。SNS 購読の URL は編集できない。外部取得は通常の SSRF 対策とサイズ・時間制限に従う。秘密や実データを fixture に使わない。

API: `GET /api/social/sources` は `sources` 配列、`POST /api/social/sources` は `{provider,actor,server?}`、`DELETE /api/social/sources/{feed_id}` は 204。Mastodon の `server` は HTTP(S) origin、Bluesky では省略する。書き込みは既存の認証・CSRF 保護が必要。

サービス仕様: https://docs.joinmastodon.org/methods/accounts/ および https://docs.bsky.app/docs/api/app-bsky-feed-get-author-feed

## 実サービス接続確認（2026-11-09）

独立した一時コンテナと空のDBで、Mastodon公式公開アカウント（mastodon.social）とBluesky公式公開アカウント（bsky.app）を匿名登録し、それぞれ手動収集を2回実行した。両方ともHTTP 200で、Mastodonは80件から157件、Blueskyは25件から89件に増えた。両者の継続cursorと継続待ちが保存され、Blueskyは取得漏れの可能性を表示する状態が保存された。テスト環境と取得データは終了後に削除した。初回サイクルの完走・新着高水位への到達、実サービスによる429/503とその待機、長期運用はこの確認に含まれない。429/503の待機や高水位の遷移は合成テストで検証する。

## 継続サイクルの追加確認（2026-11-09）

実データと分離した一時コンテナで匿名の公式公開アカウントを登録し、各回最大2ページで検証した。Blueskyは7回で初回収集を完了（338件）、次の収集も成功し、保存された高水位を確認した。取得漏れの可能性フラグは引き続き表示される。Mastodonは2回で157件を取得後、3回目に502となり、記事と継続位置・再試行時刻を保持した。上流原因と待機後の完走は未確認。取得したデータを含む一時コンテナ・DBは検証後に削除した。実サービスの429/503や長期的な完全性を保証するものではない。

同日の再検証では、投稿本文の2048バイト制限がMastodonのページ全体を拒否する原因と判明したため、本文の上限を256 KiBに変更し、空本文も許容した。修正済み候補を空の一時DBで実行し、Mastodonは7回で初回収集438件を完了して次サイクルも成功。検証用DBは削除した。実際の429/503応答と長期の継続性は未確認。

公開v1 APIのフィード・記事は情報源種別として `rss`、`mastodon`、`bluesky` を返し、記事一覧は同じ種別で絞り込める。公開v1 APIの `POST /api/v1/social/sources` はwriteキーで公開アカウントを登録する。
