# RSS / SNS Reader の実装計画

2026-11-07改訂。FeederはWeb専用の高機能な情報収集ツール／RSS・SNSリーダーとして、FeedAstrolabeから必要な機能を選びRust・Reactへ移植する。

## 段階

1. 既存の読書・RSS収集・OPML・配置・設定・アクセス制御の受入を仕上げる。
2. URL履歴、モバイル、購読管理、収集状況の表示を改善する。
3. Mastodon・Blueskyの公開投稿を、RSSと共通の既読・スター・検索・整理へ統合する。
4. 保存検索、重複整理、あとで読むなどから効果の高い改善を個別OpenSpecで選定する。
5. Reader向けAPIと設定の持ち運びを整え、データ復旧と実環境受入を記録する。

詳細な優先順位・未実装事項・アイデアは [残タスク一覧](remaining-port-tasks.md)、実装済みと検証結果は [実装状況](implementation-status.md) を参照。

## 共通条件

実装前にOpenSpecで要件・設計・受入条件を定め、既存仕様との対応を記録する。非root・read-only・認証・入力と通信の保護を維持し、合成fixtureを用いてFeederデータの保全と復旧を確認する。PC／モバイル・テーマ・マウス／タッチ／キーボードを検証し、目的ごとにコミットする。

## 除外

調査・研究ワークスペース、案件・根拠・引用・添付管理、調査レポート、AIアシスタント・研究向けwatchlist・entity/relations・intelligence・ページ観測と専用rendererは計画対象外。旧データ移行、デスクトップ、Web/local同期、X新規収集も対象外。参照資料と既存Feederデータを削除する指示ではない。
