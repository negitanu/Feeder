## Context

移行元は FeedAstrolabe Web の Flask/SQLite、現行ソース schema version 19。実稼働 DB のバージョンや使用状況は未確認。Web の未コミット同期差分は新製品のベースに含めない。ローカルアプリと同期は廃止する。

## Goals / Non-Goals

**Goals:** Web 機能の可観測な同等性、既存データの検証可能な移行、Rust と React/TypeScript への段階的移植、安全な切替と復旧。

**Non-Goals:** デスクトップアプリ、端末同期、旧ローカルアプリのデータ移行、今回のアプリ実装や本番変更。

## Decisions

- Web の UI は React、業務処理と API は Rust とし、配信と worker の境界は縦切り検証で確定する。特定フレームワークやライブラリは評価後に固定する。
- `reference/` は移行元の歴史的証拠であり Feeder 本仕様ではない。各実装変更で Feeder の canonical spec を新たに作り、互換性・除外・検証状態を対応表へ記録する。
- 同じ DB を新旧で同時書込みしない。複製と合成 fixture で検証し、SQLite 直接互換か変換かを決定する。稼働前に一貫した DB＋blob バックアップ、復元試験、切替中の書込み停止と照合を行う。
- 認証、公開 API、収集 worker、renderer 隔離、CSRF と SSRF の保護を機能実装と一緒に再現する。renderer の実装言語は安全な代替の検証まで未確定で、Rust 化のために隔離を弱めない。
- 各段階の終了は実行可能な受入テストと記録を条件とする。全機能と運用検証まで旧 Web を本番から撤去しない。

## Risks / Trade-offs

- スキーマ19だけでは稼働状態や blob の整合を保証しない → 起動前調査と整合バックアップ・差分照合。
- 共有 DB に異なる実装が同時書込み → 切替時に旧 writer を停止し、rollback 時の新規書込みを失わない運用を事前検証。
- 新 UI の操作や API が劣化 → 旧仕様・現行コード・合成 fixture を対照し、画面と契約の回帰を実施。
- デスクトップ・同期廃止が Web データ削除と混同される → Feeder の対象外と定義し、既存環境やデータの削除は計画に含めない。

## Open Questions

Rust HTTP/DB ライブラリ、React のビルド、DB 運用方式、renderer の継続・置換、API 互換範囲、段階的切替方法は検証後に決める。既存ユーザーの JS 無効時の利用と外部 Access 構成は受入条件を調査する。
