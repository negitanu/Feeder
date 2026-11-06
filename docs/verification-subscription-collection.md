# Subscription collection 検証記録

2026-11-06。以下の検証に成功。移植全体は継続中。

対象: Feeder専用DBの購読・フォルダー・手動収集、旧WebDBの読取専用検査。

受入項目: v1からの既知スキーマ移行と状態保持、RSS/Atom解析、重複防止、conditional GET/304、失敗時保持、pause/delete、記事フィルター、SSRF対策、UI保存と復帰、PC/狭い画面とテーマ、旧DBを変更しない検査。

未実施・後続: 旧Webの完全DB変換、本番切替、サイトからのRSS発見と署名付きプレビュー、定期worker、OPML、D&D/表示順保存、外部Accessと有料API。これらを完了として扱わない。

## 成功した検証

- Rust: cargo fmt、cargo test 9件、Clippy -D warnings。RSS/Atom、記事更新時の状態保持、304、pause/delete再確認、CRUDとフィルター、禁止アドレス、未知DB拒否、v1バックアップ・移行を検証。成功HTTP応答の処理はfixtureで検証し、DNS/redirect接続の全経路を模擬サーバーで網羅したわけではない。
- React: TypeScriptとVite本番ビルド成功。
- Playwright: 読書回帰と購読操作。フォルダー作成・移動・開閉保存、フォルダー名選択で展開しない、停止再開、停止時収集拒否と記事保持、私有アドレス拒否、削除時の他記事保持、390px幅とpageerror無し。収集画面のモバイル画像を目視確認。
- 実公開接続: https://go.dev/blog/feed.atom を一時DBへ登録し10件取得。状態変更後の再取得で件数と既読・スターを保持。利用者のデータ・キーは使用していない。
- Python検査ツール: unittest 4件。既存ファイルの不変、未存在ファイルの非作成、破損入力、秘密値非出力、WAL確定データを検証。実旧DBへの実行はしていない。
- CI定義にブラウザ購読テストと検査ツールテストを追加。GitHub上の実行は未実施。
