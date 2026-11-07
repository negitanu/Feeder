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

- Docker Hubのトークン取得が401となり、Linuxイメージの再ビルドとComposeでの今回の受入は未完了。一時的な空の認証設定でも同じ結果。既存のログイン情報と稼働コンテナは変更していない。今回の実装を稼働中環境へは反映していない。
- Mastodon／Blueskyの実サービス取得は未確認。取得・解析・SSRF拒否・公開状態・作者・リポスト除外は合成fixtureで検証。1回40件であり、継続ページングと429待機最適化は後続。
- タッチ端末実機、全ブラウザー、全テーマと全入力方式の組合せは未確認。今回のモバイル確認はChromiumの幅・配色エミュレーション。
- 重複判定は最新1,000件のみで意味的な本文類似判定を行わない。ダイジェストは全体100件・情報源あたり10件まで。履歴キャッシュは最大30件、1件500記事までで、それ以上は再取得する。
- 共有検索の名称・条件編集はAPIのみ。画面では保存・コピー・適用・削除を提供する。
- 設定JSONは新機能のデータを含む完全バックアップではない。新機能の保全にはSQLite／volume全体の整合バックアップを用いる。
