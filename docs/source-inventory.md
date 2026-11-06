# 移行元の仕様と機能の対応表

参照基準は FeedAstrolabe commit `b9592aa641f254a29a626937a6c02d7fcae54a6c` の Web 本仕様。移行状況は各見出しと末尾の受入状況に記録する。段階は暫定で、実装時にシナリオ単位へ分解する。参照スナップショットは [reference](../reference/feedastrolabe/README.md) に置く。

## ai-research-tracking（段階 4、未着手）

移行元: [ai-research-tracking](../reference/feedastrolabe/specs/ai-research-tracking/spec.md)。要件: Versioned research tracking、Attributed model cards、Qualified evaluations。

## article-reader（段階 1–2、部分移植）

移行元: [article-reader](../reference/feedastrolabe/specs/article-reader/spec.md)。要件: Article browsing、Persistent reading state。

## configuration-export（段階 2–3、未着手）

移行元: [configuration-export](../reference/feedastrolabe/specs/configuration-export/spec.md)。要件: Versioned configuration download、Explicit allowlist and secret exclusion、Read-only authorized export。

## contextual-source-management（段階 2、部分移植）

移行元: [contextual-source-management](../reference/feedastrolabe/specs/contextual-source-management/spec.md)。要件: Source management belongs to the library。

## entities-relations（段階 4、未着手）

移行元: [entities-relations](../reference/feedastrolabe/specs/entities-relations/spec.md)。要件: Normalized mentions、Evidence-backed relations、Reversible exploration。

## evidence-provenance（段階 3、未着手）

移行元: [evidence-provenance](../reference/feedastrolabe/specs/evidence-provenance/spec.md)。要件: Immutable evidence、Independent retention、Safe attachment lifecycle。

## feed-collection（段階 2、部分移植）

移行元: [feed-collection](../reference/feedastrolabe/specs/feed-collection/spec.md)。要件: Subscription management、Idempotent refresh、Safe website discovery and subscription preview。

## grounded-assistant（段階 4、未着手）

移行元: [grounded-assistant](../reference/feedastrolabe/specs/grounded-assistant/spec.md)。要件: Grounded assistance、Controlled external processing、Safe reproducible jobs。

## local-operation（段階 0–5、部分移植）

移行元: [local-operation](../reference/feedastrolabe/specs/local-operation/spec.md)。要件: Local runtime and data、Input and content protection。

local は既存 Web サーバーの localhost 運用を指し、廃止対象の Electron アプリではない。

## network-intelligence（段階 4、未着手）

移行元: [network-intelligence](../reference/feedastrolabe/specs/network-intelligence/spec.md)。要件: Temporal network observations、Uncertainty-aware interpretation、Passive bounded monitoring。

## reader-interactions（段階 1–2、部分移植）

移行元: [reader-interactions](../reference/feedastrolabe/specs/reader-interactions/spec.md)。要件: Persistent disclosure without reset control、Pause and resume subscriptions in context、Prominent scoped bulk read、Floating article detail and original access、Content-first article detail、Optional bounded hero image、Detail bounded by the article area、Mark displayed article as read、Global unread library entry。

## reader-panel-polish（段階 1–2、未着手）

移行元: [reader-panel-polish](../reference/feedastrolabe/specs/reader-panel-polish/spec.md)。要件: Dismiss detail without losing the reader、Smooth accessible detail transitions、Consistent selected action controls、Shared brand favicon。

## reader-productivity（段階 1–2、部分移植）

移行元: [reader-productivity](../reference/feedastrolabe/specs/reader-productivity/spec.md)。要件: Independent recent reading history、Shared deterministic ordering、Detailed literal search form、Explicit selected-article bulk state。

## reports-operations（段階 3、未着手）

移行元: [reports-operations](../reference/feedastrolabe/specs/reports-operations/spec.md)。要件: Portable sourced outputs、Controlled notification delivery、Recoverable operations、Bounded management APIs。

## research-workspaces（段階 3、未着手）

移行元: [research-workspaces](../reference/feedastrolabe/specs/research-workspaces/spec.md)。要件: Research organization、Versioned notes、Scoped workspace access、Shared tags and bounded filtering。

## runtime-refresh（段階 0–5、部分移植）

移行元: [runtime-refresh](../reference/feedastrolabe/specs/runtime-refresh/spec.md)。要件: Latest compatible runtime images、Python 3.15 and current packages、Preserve container behavior and data。

Python のバージョン要件は Rust へ読み替え、renderer の実装言語は別途決定する。非 root 等の保護とデータ保全は維持する。

## security-intelligence（段階 4、未着手）

移行元: [security-intelligence](../reference/feedastrolabe/specs/security-intelligence/spec.md)。要件: Attributed vulnerability cards、Qualified applicability、Bounded connectors。

## selectable-ai-providers（段階 4、未着手）

移行元: [selectable-ai-providers](../reference/feedastrolabe/specs/selectable-ai-providers/spec.md)。要件: One provider selection interface、Provider-specific request and result contracts、Preserve budgets and provider lineage、Compatible OpenRouter migration。

## sidebar-organization（段階 2、部分移植）

移行元: [sidebar-organization](../reference/feedastrolabe/specs/sidebar-organization/spec.md)。要件: Persistent folder and feed direct ordering、Move feeds between folders、Touch and keyboard parity、Resizable sidebar。

Feeder `sidebar-organization` で DB による順序・所属保存、直接ドラッグ、キーボード、操作メニューからの移動、幅変更を移植。旧空名フォルダーのデータ変換は本移行の検証まで未完了。フォルダーなし(NULL)と文字列「未分類」のフォルダーは区別する。検証は[配置の検証](verification-sidebar-organization.md)を参照。

## watchlists-triage（段階 4、未着手）

移行元: [watchlists-triage](../reference/feedastrolabe/specs/watchlists-triage/spec.md)。要件: Repeatable triage、Idempotent matching、Provenance-preserving groups。

## その他の追跡

- SNS、ページ観測、既存 API の細部は README・操作文書・実装・旧変更仕様を調査し、各 Feeder 本仕様へ紐づける。上記20本だけで全シナリオが網羅されたとは扱わない。
- 未コミットの Web/local 同期仕様と実装、desktop-distribution は明示的に除外。既存 Web データや通常の API は除外しない。
- 実動作、テスト結果、既知の制約と外部受入状況を今後各要件に追記し、参照仕様を完了と誤認しない。

## ソースの静的棚卸し

[source-inventory.json](../reference/feedastrolabe/source-inventory.json) にソースモジュールとルートデコレーターの位置・ハッシュを記録した。これは実装調査の索引であり、公開APIのprefix、認証・副作用・レスポンス互換性は未照合。同期の未コミットコードも含まれるため対象外を識別して扱う。

## 読書・収集の受入状況（2026-11-06）

`reader-foundation`、`subscription-collection`、`discovery-worker-opml` を検証済み。記事状態、直接購読、候補発見、非破壊プレビュー、期限・セッション・再利用防止の確認、停止再開、手動・定期更新、OPMLが対象。受入は Feeder 本仕様と各検証記録を参照。旧API完全互換、既存WebDB変換、JS無効時の画面、外部認証、D&D/順序保存/幅変更、条件全体の「全て既読」、調査・外部分析・高度な運用は未完了。

## Linux実行基盤（2026-11-06）

`linux-container-runtime` でWeb/worker、永続volume、health、非root/read-only、Host境界、正常終了、生成データのバックアップ/別volume復元を検証。旧runtime-refreshのPython固有要件をRust/ReactのLinuxビルドへ読み替えた。renderer/observerと外部認証は未移植。

## 読書履歴・詳細条件・一括操作（2026-11-06）

`reader-productivity` で実閲覧履歴、UTCの詳細条件、安定した並び、未描画を含む全件既読と選択状態の原子更新を実装・検証。署名付きcontinuationや表示timezoneなどの差分は [検証記録](verification-reader-productivity.md) で追跡する。旧仕様全体の完了とは区別する。
