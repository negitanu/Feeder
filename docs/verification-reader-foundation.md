# Reader foundation の検証

2026-11-06。以下の検証が成功。全面移植と本番切替は未完了。

## 対象

生成データを使う読書の最小実装。既存 Web の実 DB、秘密情報、API キー、有料サービスを使用しない。外部配備や全機能移植の検証とは区別する。

## 実行予定

- Rust: format、単体・HTTP 契約・永続化・未知DB拒否・Origin防御のテスト、Clippy。
- React: TypeScript 検査と本番ビルド。
- 組合せ: health、記事取得、既読とスター、再起動後の状態、ブラウザの主要操作と狭い画面・テーマ。
- OpenSpec: 全件 strict と文書リンク。

## 未実施の受入

既存DBの完全移行、全業務機能、Cloudflare Access、収集・renderer隔離、実 AI と通知、PC／モバイルの全操作回帰。本番切替のゲートは継続して未達。

## 成功した検証

- Rust 1.99.1: cargo fmt、cargo test（2テスト、API/永続化/DB保護/OriginとHost防御を含む）、Clippy -D warnings。
- Node 22.16.0、React 19.3.1、TypeScript 7.0.2、Vite 8.3.6: 型検査と本番ビルド成功。npm audit 0 vulnerabilities（依存導入時）。Cargo.lock/package-lock.json を保存。
- Rust実HTTP: health、36件一覧、正当な状態変更、外部Origin403。
- Playwright 1.65.0 / Chromium: 詳細と既読、スター、追加読み込み36件、テーマ再読込、390px検索、横はみ出し無し、pageerror無し。画面の目視確認も実施。
- GitHub CI 定義を追加。リモート未設定のためGitHubでの実行は未実施。
