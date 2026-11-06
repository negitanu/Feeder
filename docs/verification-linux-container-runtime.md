# Linux container runtime 検証記録

2026-11-06。以下の検証を実施。対象はLinuxコンテナ。macOS/Windowsネイティブ配布は対象外。

## 計画した受入

- multi-stageのReact/Rustビルドと最小runtime。
- Web/workerのhealth依存、非root、read-only、cap_drop、no-new-privileges、永続volume。
- 明示Host許可と不正Origin拒否、通常HTTP/API/UI配信。
- サービス再作成後の状態・設定保持、workerの停止と終了。
- 既存FeedAstrolabeとは異なるComposeプロジェクト/volumeを使用。

実外部Access・既存Webデータの完全移行・renderer/observer・本番切替は未実施。

## 成功した検証

- Docker Engine Linux/arm64上で公式 Node 22 / Rust 1.99 / Debian trixie の各タグを取得しmulti-stageビルド成功。Reactの型検査・本番ビルドとRust release --lockedビルドがコンテナ内で成功。
- `docker build --target backend-test .` でLinux内のRustテスト19件成功。Host許可・不正Origin・forwarded header・worker停止の回帰を含む。fmt/Clippyも成功。
- Compose設定検査、Web health成功後のworker起動、HTTP/UI/API、非root UID/GID10001、read-only、capabilities削除、no-new-privilegesを実コンテナで確認。runtimeにnode/cargoがないことも確認。
- UUID付き専用プロジェクトと生成データのみを使用。サービス再作成後も既読・スター・設定を保持し、worker停止時exit0を確認。
- 全writerを止めた/dataをtarで取得し、別の新規project/volumeへ復元。復元先HTTPで既読・スターと設定を照合。既存FeedAstrolabeのvolumeは操作していない。
- Linux CIにイメージビルド、Linuxテストtarget、Compose受入を追加。GitHub上の実行は未実施。Linux amd64の実ビルドは未実施で、定義済みUbuntu CIの対象。

外部公開認証/tunnel、renderer/observer、既存Webデータの全量変換、実運用データ復旧、移植全体の本番切替は未完了。コンテナ化によってこれらが完了したとは扱わない。

- 最終イメージにPlaywrightから接続し、記事詳細・既読/未読・スター・追加読込・テーマ保存・390px検索を確認。画面エラーと横はみ出し無し。コンテナテスト用project/volumeは終了後に削除し、通常環境は起動・停止していない。
