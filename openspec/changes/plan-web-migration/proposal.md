## Why

2026-11-06 の方針変更: 旧 FeedAstrolabe データの移行・変換は対象外。以下の旧データ切替に関する内容は過去の計画とし、Feeder 自身のデータ保全・復旧は維持する。

FeedAstrolabe の Web 機能を維持しつつ、単一の Web 製品 Feeder として Rust と React/TypeScript に移植する。段階的な受入と安全な切替の基準を先に確定する。

## What Changes

- Web 版の現行仕様と実装を棚卸しし、段階、依存関係、各段階の受入条件を定義する。
- ローカルデスクトップ版、その移植計画、Web/local 同期を Feeder の対象から除外する。
- Web データ、設定、API、セキュリティ、運用の継続と切替・復旧の手順を計画する。
- **BREAKING** 旧デスクトップ配布と同期 API は Feeder に引き継がない。

## Capabilities

### New Capabilities
- `web-migration`: Web 機能の段階的移植、検証、データ保全、切替と復旧。

### Modified Capabilities

なし。Feeder に既存の本仕様はまだない。

## Impact

新リポジトリの計画・参照資料・将来の Rust/React 実装。稼働環境や既存リポジトリは変更しない。API や DB の互換方式は調査して決める。
