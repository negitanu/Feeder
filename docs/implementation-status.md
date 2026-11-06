# 実装と移植の進捗

2026-11-06。`reader-foundation` の実装・検証が完了。全体移行は継続中。

## 今回の実装対象

Rust HTTP/SQLite と React/TypeScript の読書経路。生成サンプルで記事一覧、詳細、検索、既読／スターと再起動後の永続化を検証した。旧 DB を直接変更しないため、既知の Feeder 以外のスキーマは起動時に拒否する。

## 基準線

移行元のソース91モジュール、285ルートデコレーターを静的解析した結果を `reference/feedastrolabe/source-inventory.json` に保存。パスは Blueprint の prefix を結合する前の断片であり、公開 API の完全な一覧を意味しない。再生成: `python3 scripts/inventory-source.py ../FeedAstrolabe --output reference/feedastrolabe/source-inventory.json`。

Web/local 同期は移植対象外。元ソースの作業ツリーには同期関連の未コミット変更が含まれるため、静的解析結果には混在する。既存コードの SHA-256 を記録し、今後の契約比較では同期を除外する。実 DB、秘密情報、記事本文をこの調査に使用していない。

## 今後の受入ゲート

reader-foundation の成功だけで全面移植や本番切替を完了としない。購読・収集・設定・認証、調査・根拠、外部連携、運用とデータ移行は `plan-web-migration` のタスクで継続する。既存 API の互換性、DB変換、PC／モバイル全テーマ、実外部接続は領域ごとに検証結果を記録する。

結果: [検証記録](verification-reader-foundation.md)。この段階以降の実装は以下の履歴へ記載する。

## 購読・収集の実装

`subscription-collection` の最初の段階を実装・検証。直接RSS/Atom URL、フォルダー、手動更新、停止再開、削除を優先し、旧仕様のサイト発見・プレビュー・定期worker・高度な並べ替えを後続として維持する。旧DBの検査は構造・件数・整合性のみ、明示指定されたファイルを読取専用で扱う。

購読段階の結果: [検証記録](verification-subscription-collection.md)。この段階では旧Webデータ移行の完成、サイト発見・プレビュー・定期worker・高度な並べ替えは未完了だった。

## フィード発見・定期収集・OPML

`discovery-worker-opml` を実装・検証。プレビュー時の非登録と明示確認、独立workerの停止制御と重複取得の排除、OPMLの重複処理・安全なXML処理を今回の受入対象とする。OPMLからの新規購読は一時停止状態とし、初回ネットワーク取得を自動実行しない。

現在の検証結果: [フィード発見・worker・OPML](verification-discovery-worker-opml.md)。次の領域は読書履歴・詳細検索・条件全体の一括既読、表示順・幅、設定の持ち出しと認証・コンテナ基盤。全体計画の段階2は部分完了。

## Linuxコンテナ運用

Linuxコンテナを正式な実行環境として `linux-container-runtime` を実装・検証。Mac/Windowsネイティブ実行や配布は対応対象に含めない。Webとworker、永続volume、health、非root/read-only/権限削減、Host許可、終了処理を検証する。

結果: [Linuxコンテナ検証](verification-linux-container-runtime.md)。Web/worker の起動、health、永続化、権限制御と終了を検証済み。外部認証/tunnel、renderer/observer、本番切替は未完了。

## 読書履歴・詳細条件・一括操作

`reader-productivity` を実装・検証。明示的な詳細閲覧の履歴と、共通の条件解釈を使う一覧・全件既読・選択状態変更を実装する。初期の日付境界はUTC。表示タイムゾーン設定と署名付き継続カーソル等の旧仕様差分は残る。

結果: [読書機能の検証](verification-reader-productivity.md)。次はフォルダー/フィードの表示順保存・幅変更、表示設定と秘密を除く設定持ち出し、外部認証・公開APIの移植。

## ライブラリの配置と幅

`sidebar-organization` でフォルダー／フィード順序、フォルダー移動、ポインター・タッチ・キーボード操作とブラウザ別の幅保存を移植。DB v5 は既知の v4 をバックアップした後、既存の表示順を保って更新する。別セッションとの配置競合は上書きせず最新状態を読み直す。

結果: [配置の検証](verification-sidebar-organization.md)。次は表示設定、秘密を除く設定持ち出し、外部認証・公開API。旧DBの空名フォルダー等を含む完全変換は引き続き未完了。
