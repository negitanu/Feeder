# API・アクセス制御の検証

2026-11-06。変更 `api-access-control`。Linuxコンテナを正式環境とし、生成データ・使い捨てキーのみで確認した。実資格情報・旧DB・既存サービスは操作していない。

## Rustと移行

- Linux `docker build --target backend-test .`: 39テスト成功。ホスト側の同じ39テスト、fmt、Clippy `--all-targets -- -D warnings` も成功。
- v6からv7への認識済み移行・更新前バックアップ・記事/既読/スター/訪問/順序/共有設定の保持と再起動。
- production middlewareでHost、Origin、CSRF/cookie、期限切れセッション、重複ヘッダー/重複Cookie、セッション数の上限を確認。
- Tunnelモードの正確なHTTPS origin、Secure Cookie、偽の転送Hostによる迂回拒否、認証なしhealthを確認。これはローカルの模擬リクエストであり、実JWT検証のテストではない。
- キーのハッシュ保存・一度だけの全文返却・read/write・期限切れ/失効・回数上限/Retry-Afterを確認。DBを読んでもキー全文が保存されていない。
- 公開記事APIのID順、before/after継続、条件検索、重複/矛盾/未知条件、型不正を検証。状態の部分更新は原子的に未指定フラグを保持し、訪問を作らない。
- RSSの安全な初回取得の後にフォルダー・購読・記事を一括保存する経路を実装。fixture購読CRUD・無効な非公開URL拒否時の非登録・未対応設定の拒否・記事保持を検証。実外部RSSへのAPI経由取得はこの段階では未実施。

## ブラウザ・Linuxコンテナ

実行: `FEEDER_BROWSER_SMOKE=1 FEEDER_PRODUCTIVITY_SMOKE=1 FEEDER_SIDEBAR_SMOKE=1 FEEDER_SETTINGS_SMOKE=1 FEEDER_ACCESS_SMOKE=1 python3 scripts/container-smoke.py`。

- 既存の読書・追加読込・詳細/検索/一括操作・配置/幅・タッチ・共有設定/表示形式・安全な設定書き出しの全回帰検証が成功。
- CSRFなし/誤Originで変更が拒否され、ブラウザCookieだけで公開APIを呼べない。
- 画面から読み取り/書き込みキーを発行し、一度だけの表示・隠す・再読込後の非表示・失効を確認。
- 読み取りキーの変更拒否、writeキーの記事状態変更、公開APIによる閲覧履歴不変、Bearerだけでキー管理ができないことを確認。
- 回数上限1回にしたキーで200→429とRetry-Afterを確認して上限を120へ復帰。失効後は401。
- 設定JSONとキー一覧に全文/ハッシュが出ないこと、API状態更新時に記事と履歴を保つことを確認。
- コンテナ再作成と別volumeへのバックアップ復元で、キー情報/回数設定・記事/履歴・配置・表示/収集設定が一致。セッションを再取得して管理APIへアクセスできる。
- 390px画面のキー管理を目視確認。横はみ出し・ブラウザ例外なし。非root/read-only/権限削減、正常終了、専用テストvolume/コンテナの削除も成功。

## 公開構成

`check-tunnel-config.py` で、解決後のComposeにWeb/workerの公開ポートがなく、HTTPS origin/Host一致、固定connectorイメージ、tokenファイル参照のみであることを確認。公開ポート追加・latestイメージ・HTTP originを拒否する負の確認も成功。実tokenファイルを読まず、connectorは起動していない。

参考にした公式仕様は[アクセス制御の資料](api-access-control.md)に記録。sha2/subtleをCargo.lockで固定。GitHub Actionsには同じLinux/ブラウザ/構成検査を追加したが、この変更のCI実行は未確認。

## 未完了

実ドメインのCloudflare Access許可/拒否/Service Auth/Protect with Accessの受入、アプリ独自のJWT検証、旧DB/キーの移行、公開API全フィールド/日時表現/個別間隔とSNS・観測・研究APIの互換性、本番切替は未完了。localモードは利用者認証を提供せず、ループバック公開と信頼されたコンテナネットワークを前提とする。
