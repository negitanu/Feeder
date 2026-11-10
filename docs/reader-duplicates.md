# 重複記事

重複表示は ID の新しい順に最大 1,000 記事ずつ調査し、古い範囲と新しい範囲を移動できます。範囲内のURLに一致する記事はライブラリ全体から検出します。同じグループが複数範囲に現れる場合があります。各範囲では、HTTP(S) URL のフラグメントと `utm_*`、`fbclid`、`gclid`（名前は大文字小文字を区別しない）を除いた URL が一致する記事をまとめます。その他のクエリは順序を含め保持します。元 URL、既読、スターは変更しません。調査上限に達してさらに記事がある場合は画面に明示します。件数は範囲に含まれるURLに一致するライブラリ全体の記事数です。1グループの表示は100件ずつ追加できます。データが変化するとページ間で結果が変わる場合があります。

`GET /api/duplicates?offset=0&limit=20&before_id=...`（before_idは任意） は `groups`、`group_count`、`article_count`、`scanned`、`truncated`、`offset`、`limit`、`has_more`、次の範囲の `next_before_id` を返します。`groups` の各要素は `canonical_url` と `articles` を持ち、グループは全体の `match_count` と表示省略を示す `members_truncated` を含み、各記事は `id`, `title`, `url`, `feed_id`, `feed_title`, `is_read`, `is_starred` を持ちます。offset は 0–1000、limit は 1–50 です。グループは最新記事 ID の降順、記事も ID の降順です。入力不正は 400、DB エラーは詳細を含まない 500 です。

統合担当は `duplicates::migrate` を v9 トランザクションで呼び、`duplicates::routes()` を既存ルーターに追加し、既存の認証・認可・strict schema 検証の対象にしてください。移行はスキーマを変更しません。`DuplicatePanel` は `{onOpen: (id: number) => void, onRead: (ids: number[]) => Promise<void>}` を受け取ります。`onRead` は指定された ID のみを既読にし、失敗時は reject してください。パネル内の切り替えで通常画面に戻れます。App 側では表示モードを連携し、重複表示中の通常リストの表示を調整してください。開く操作は既存詳細画面の動作に従います。他のコピーを自動操作しません。

`GET /api/duplicates/members?article_id=...&before_id=...` は基点記事と同じ正規化URLの記事を新しい順に最大100件返し、続きがあれば `next_before_id` を返します。取得処理は元記事を変更しません。
