# Reader拡張の統合検証（2026-11-07）

## 実装と対応仕様

ReactとRustの既存Web構成に次の機能を接続した。研究・調査機能、旧データ変換、デスクトップ同期は追加していない。

- [共有検索](shared-smart-folders.md): ブラウザー保存を保持した明示コピー、別セッションでの適用、競合検知。
- [重複表示](reader-duplicates.md): 最新1,000件を正規化URLでまとめ、原本・既読・スターを保持。
- [公開SNS](public-social-reader.md): Mastodon／Blueskyの購読を通常workerと共通の記事ストレージへ接続。
- [あとで読む](read-later.md): 独立した保存状態、一覧・詳細の操作、任意の訪問時削除。
- [ミュート](reader-mute-rules.md): 一覧・一括操作・ダイジェストの共通除外、明示再表示、詳細での理由表示。
- [収集診断](collection-health.md): 受理された取得だけを記録し、成功・失敗・次回予定・個別間隔を区別。
- [URL履歴](reader-url-history.md): 深いリンク、戻る／進む、有限の一覧・選択・スクロールキャッシュ。
- [ダイジェスト](reader-digest.md): 24時間／7日間の実件数、最新記事、明示的な表示上限。
- [記事行](feed-source-title-time.md): 発行元Feed、タイトル、絶対日時の順。UTC／JSTとモバイル幅に対応。

各機能の要件・設計・受入条件は同名のOpenSpec変更に記録した。今回の10変更と記事行の本仕様はOpenSpecのstrict検証を通過した。共通の移行・統合は `reader-expansion-foundation`。

## 実行した検証

合成データと専用の一時SQLiteを使い、実購読・秘密情報をfixtureにしていない。

- `cargo test --manifest-path backend/Cargo.toml`: 76件成功。DB v8→v9、既読・スター・履歴・購読順序・既存設定の保持、更新前バックアップ、未知スキーマの無変更拒否を含む。
- v9のSQLiteバックアップを別ファイルへ復元し、共有検索・ミュート・SNS・あとで読む・収集状態を含む各セルの一致、integrity_check、外部キーcascadeを検証。
- `cargo fmt --check`、`cargo clippy --locked --all-targets -- -D warnings`、Reactの本番ビルド成功。
- `scripts/reader-expansion-browser-smoke.mjs`: 専用プロセスと一時DBを自動作成。Feed2件・記事14件を使い、共有保存と別セッション、ミュート理由と一括既読範囲、保存ボタンとあとで読む一覧、任意の訪問時削除、原本を残す重複表示、日時不明・ミュートを除いたダイジェスト12件、収集診断、SNS入力制約を確認。PC／390px・ライト／ダーク、キーボードでの詳細表示、1行表示の列順とページの横溢れを確認。
- `scripts/reader-history-browser-smoke.mjs`: CSRFを含む合成APIで初回訪問1回、戻る／進むで余分な訪問が増えないことを確認。
- 既存の `browser-smoke`、`middle-read`、`detail-layout`、`productivity`、`saved-search`、`settings` ブラウザ回帰成功。設定テストのDB番号と時刻期待値をv9・絶対日時に更新。
- `frontend/tests/article-row-time.test.mjs`: SQLiteのUTC日時とオフセット付き日時をUTC／JSTで確認。America/Los_Angeles環境でも成功。PCのダークテーマ1行表示を撮影し、目視で列・省略・操作領域を確認。

CI設定に日時テスト、履歴とReader拡張の合成スモークを追加した。GitHub上のCIは実行・確認していない。

## 制限と未確認

- 当初のDocker Hub認証401は解消後に再検証し、Linuxビルド・受入・稼働反映を完了した。詳細は下の追補を参照。
- Mastodon／Blueskyの実サービス取得は未確認。取得・解析・SSRF拒否・公開状態・作者・リポスト除外は合成fixtureで検証。1回40件であり、継続ページングと429待機最適化は後続。
- タッチ端末実機、全ブラウザー、全テーマと全入力方式の組合せは未確認。今回のモバイル確認はChromiumの幅・配色・タッチ操作エミュレーション。
- 重複判定は最新1,000件のみで意味的な本文類似判定を行わない。ダイジェストは全体100件・情報源あたり10件まで。履歴キャッシュは最大30件、1件500記事までで、それ以上は再取得する。
- 共有検索の名称・条件編集はAPIのみ。画面では保存・コピー・適用・削除を提供する。
- 設定JSONは新機能のデータを含む完全バックアップではない。新機能の保全にはSQLite／volume全体の整合バックアップを用いる。

## Linux受入と稼働反映（同日追補）

Docker Hub認証の解消後、`docker compose build web` と `docker build --target backend-test .` が成功。Linux内でバックエンド76件すべてが通過した。

すべてのブラウザフラグを有効にした `scripts/container-smoke.py` が成功。読書・詳細・OPML・生産性・並べ替え・設定・認証・保存検索・読みやすさ・詳細レイアウト・中クリック・履歴を通しで実行した。非root／read-only／capabilities、秘密の非露出、別volumeへのアーカイブ復元、再作成後の既読・スター・履歴・購読・設定・APIキー保持、workerの正常終了も確認した。新9機能の専用合成スモークも再実行して成功した。

この受入で次を修正した。

- 詳細を閉じた後に成功した閲覧応答が届いた場合、同じ一覧の既読を反映する。詳細は再表示せず、検索・一覧世代が変わった応答は破棄する。既存の遅延応答・後続詳細優先テストで確認。
- 購読のあるモバイル設定画面で、収集診断表の非表示列ラベルがページを広げないよう、スクロール領域内に包含する。OPML取り込み後の390px検証で確認。
- 並べ替えテストを対象フォルダー内に限定し、先に取り込まれた別フォルダーの購読に依存しないようにした。

稼働環境はすでにDB v9と今回の拡張機能を使用しており、v8更新前バックアップの存在を確認した。今回のUI修正の反映前にはWeb／workerを停止し、Git管理外の `backups/reader-linux-fixes-d7sw2llb/feeder-data.tar.gz` に `/data` 全体を退避。非公開ディレクトリで展開し、SQLiteのapplication_id・schema v9・integrity_check・foreign_key_checkを検証した。アーカイブのSHA-256と検証結果は同ディレクトリの `verification.json` に保存している。

以前の稼働コードとハッシュが一致する復旧用イメージ `feeder:before-reader-linux-fixes-20261107` をGitの版から再構築して保持。その後、既存volumeを保持して `docker compose up -d --no-build --wait` を実行した。Webはhealthy、workerとTunnelはrunning、healthは200、Tunnel接続登録4件、Web／workerのpanic記録0件。稼働中の実行ファイル・画面ハッシュは検証済みイメージと一致した。Cloudflare Accessのログイン後画面とSNS実サービス取得は今回の受入では未確認。
