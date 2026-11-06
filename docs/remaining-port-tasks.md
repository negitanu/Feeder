# Feeder 残りの Web 移植タスク

監査時点: 2026-11-07。これは FeedAstrolabe 参照スナップショットと Feeder の現行 OpenSpec・実装・検証記録を突き合わせた、次の作業を選ぶための一覧である。参照資料は移行元の履歴であり Feeder の仕様・検証結果ではない。既存Feederデータの保全と復旧は必須。

## 現在の範囲と状態

- **実装済み（記録済み）— 読む／RSS収集／配置／一部設定・アクセス制御:** 読書状態・検索・履歴・段階読込、購読発見と確認、OPML、定期収集、フォルダー／フィード順序、表示設定・設定エクスポート、APIキーと公開RSS/記事API。根拠: `openspec/specs/reader-foundation/spec.md`、`subscription-collection/spec.md`、`discovery-worker-opml/spec.md`、`sidebar-organization/spec.md`、`reader-settings-export/spec.md`、`api-access-control/spec.md` と各変更の完了済み `tasks.md`、`docs/implementation-status.md`。
- **実装済み（今回の監査報告）— 設定の統合:** 「集める」行き先を除去し、設定を統一済み。メイン実装で読書・設定・OPMLプレビュー・APIキーのブラウザー検証とバックアップ復旧を確認。記録: `docs/verification-unified-settings.md`。統合作業の OpenSpec は `openspec/changes/unified-settings/`。
- **部分実装— 読書継続と残る操作:** 署名付き継続、失敗時再試行、浮動詳細・前後移動は実装済み。一方、`openspec/changes/reader-stable-continuation/tasks.md` の受入・記録 3.1–3.3 は未完了。参照 `article-reader/spec.md` のブラウザ履歴復元、旧URL、JavaScript 無効時の結果到達性も残件候補で、個別 OpenSpec で範囲を定める。
- **部分実装— 公開 API 契約:** RSS/記事の v1 と認証・権限は実装済み。旧 API 全体との互換性は証明されていない。`docs/priority-backlog.md` P0-03 と `openspec/specs/api-access-control/spec.md` を参照。
- **未実装— 調査・研究データ領域:** research workspace／メモ版管理／タグ、immutable evidence・引用・添付と blob 保全、entity／relations は Feeder の Rust/React 実装・canonical spec が確認できない。参照 `research-workspaces/spec.md`、`evidence-provenance/spec.md`、`entities-relations/spec.md`。既存の Feeder 記事等の保全要件と混同せず、新規機能として合成 fixture で扱う。
- **未実装— 運用・通知:** 出典付きレポート、通知配信、再現可能なジョブ制御、定期バックアップと別環境復元は未完了。参照 `reports-operations/spec.md`、`docs/priority-backlog.md` P1-04、`openspec/changes/plan-web-migration/tasks.md` 3.2 / 4.2。
- **未実装— 外部ソース・分析:** SNS収集、AI provider／grounded assistant／watchlist、network/security intelligence、ページ観測と隔離 renderer は未実装。参照 `feed-collection/spec.md`、`selectable-ai-providers/spec.md`、`grounded-assistant/spec.md`、`watchlists-triage/spec.md`、`network-intelligence/spec.md`、`security-intelligence/spec.md`。Feeder に環境変数の受け口があっても provider 機能の実装とは数えない（`docs/implementation-status.md`）。

- **部分実装— UI/UX:** 配色・カード・ブランド・設定統合は実装済み。モバイルのフィード開閉、設定カテゴリの分割、URL・履歴連携、詳細の開閉アニメーションが残る。根拠: `docs/verification-reader-visual-parity.md`。

## 次の順序

1. **P0 — 継続読込の残る受入を閉じる:** `openspec/changes/reader-stable-continuation/tasks.md` 3.1–3.3。改ざん・期限・条件不一致・挿入削除・2ページ目失敗と再試行・observerなし・モバイル/テーマ・復元を合成データで確認し、未確認項目を記録する。既存変更の受入を完了する作業。
2. **P0 — 公開 API の互換性契約:** `docs/priority-backlog.md` P0-03 に従い、旧 API 仕様と `backend/src/public_api.rs` の RSS/記事について互換・意図的差分・未対応を確定。fixture による契約を OpenSpec で先に定める。SNS/研究 API は各機能の変更に含める。
3. **P1 — Research workspace 最小垂直スライス:** 案件、版管理メモ、タグ、記事との関連、認可・競合処理を対象とする OpenSpec を作成し、React画面・Rust API/SQLite・合成 fixture を一緒に実装する。受入後に evidence を追加する。
4. **P1 — Evidence と添付:** P1-3 完了後、immutable capture/quotation、添付の認証・上限・隔離処理、DBと blob の一貫バックアップ／別環境復元を OpenSpec 化して実装する。
5. **P1 — 復旧運用とレポート／通知:** ジョブの再試行・中断・重複防止、出典付きレポート、模擬通知、Feeder データの別 volume 復元を分割して進める。
6. **P2 — 外部連携を個別変更で追加:** 依存する workspace/evidence/復旧基盤後に、SNS、AI/assistant/watchlist、intelligence、隔離 observer を別々の OpenSpec・模擬接続・上限付きで着手する。

## 対象外と全体ゲート

- FeedAstrolabe の記事・既読・スター・設定・研究・根拠・添付の移行／変換は対象外。Feeder に保存されたデータのバックアップ・復元・更新失敗時の保全は対象（合成 fixture を使う）。
- 旧デスクトップアプリ、Web/local 同期、旧同期コード、X の新規収集は対象外。運用仕様のローカル volume は Web サーバーの永続化であり、desktop/local 同期ではない。
- 公開・切替前には `docs/priority-backlog.md` の G1 外部 Access、G2 Feeder DB と添付の復旧、G3 全体受入を完了する。未実施の外部接続・依存監査・CI は成功扱いしない。

## 根拠の案内

- Feeder 実装の進捗と検証リンク: `docs/implementation-status.md`。
- 優先順位・受入条件: `docs/priority-backlog.md`。
- 網羅的な移行計画と全体ゲート: `openspec/changes/plan-web-migration/tasks.md`。
- 旧仕様スナップショットの由来・注意: `reference/feedastrolabe/README.md`。
