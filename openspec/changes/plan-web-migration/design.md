## Goals

Web専用のRSS/SNS Readerとして、Rust APIとReact UIで収集・読書・整理を実現する。FeedAstrolabeは参照資料とし、対象機能を選んで移植する。

## Decisions

- 優先順位は docs/remaining-port-tasks.md に集約する。
- 調査・研究・根拠管理・研究向けAI分析を除外し、SNS収集をそれらへ依存させない。
- 既存Feederデータを合成fixtureで保全・復旧検証する。旧データ変換は行わない。
- 認証・権限・CSRF・SSRF・非root・read-onlyを維持する。将来レンダラーを追加する場合は別途隔離設計を必須とするが、現計画には含めない。
- 互換性はReader用途のAPIと設定について定義し、全旧APIとの同等性は求めない。

## Acceptance

各機能の仕様、PC／モバイル・各テーマ・入力方法、永続化・復旧と外部接続の検証を記録する。実装候補のアイデアは個別OpenSpecで採用後に着手する。
