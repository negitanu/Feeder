# Cursor 引き継ぎ入口

2026-11-06。アプリ実装の基準コミットは `f942dad`。この引き継ぎ文書・次期OpenSpecの追加は、それ以降の準備作業であり、実装済み機能の追加ではない。

## 最初に読むもの

1. この文書とリポジトリ直下の [AGENTS.md](../../AGENTS.md)。
2. [優先順位と完了条件](priority-backlog.md)。
3. [最初の変更の提案](../../openspec/changes/reader-stable-continuation/proposal.md)、同じディレクトリのdesign/specs/tasks。
4. 作業対象のFeeder本仕様 `openspec/specs/` と `docs/verification-*.md`。旧仕様は `reference/feedastrolabe/specs/` にある参照資料。

## リポジトリと継続方針

- GitHub: `git@github.com:negitanu/Feeder.git`、作業ブランチ **dev**。アプリ実装は `f942dad` までpush済み。mainの変更・merge/releaseは行っていない。
- この環境の実体は `/Users/tkobayashi/src/Feeder`。会話の既定cwd `/Users/tkobayashi/Documents/ChatGPT/Feeder` とは別なので注意。別PCではclone先を使う。
- 移行元のコード参照は `/Users/tkobayashi/src/FeedAstrolabe`。別PCでは参照元を別ディレクトリで用意する。旧作業ツリーには同期関連の変更があるため、全てを取り込まない。参照基準のcommit/静的索引は `reference/feedastrolabe/` に記録済み。
- **Webのみ・Rust + React/TypeScript・Linuxコンテナのみ**。ローカル/desktopアプリ、Web/local同期、Mac/Windows native配布を復活させない。
- X新規収集も復活させず、既存Webの保存済み記事は保全する。既存Webの本番データ・稼働サービスは未変更で、切替前に停止/削除しない。
- 既存方針: OpenSpec先行、合成fixture、受入結果を記録、devへ目的別commit。コミット作者はローカル本人設定、AI共同著者は自動追加しない。履歴改変/force-pushを使わない。

## 実装済みの基準線

Feederの8変更（reader-foundation、subscription-collection、discovery-worker-opml、linux-container-runtime、reader-productivity、sidebar-organization、reader-settings-export、api-access-control）は、その変更範囲の実装・検証を完了。旧Web全体の移植完了ではない。

- 読む: 一覧/詳細、追加読込、既読/スター、最近の履歴、詳細条件、全件/最大100件の選択一括操作。現在の継続はoffsetなので、並行挿入/削除の課題が残る。
- 集める: RSS/Atom、HTML候補発見、確認プレビュー、フォルダー、手動/独立worker収集、停止/再開/削除、OPMLの確認後読み込みと書き出し。
- 表示: sourceの配置/所属をDBへ保存、キーボード/ポインター/タッチ、幅/開閉、標準/1行/タイル、テーマ、共有UTC/JSTと初期表示。
- 設定/API: 非秘密設定JSONの書き出し、ブラウザsession/CSRF、ハッシュ保存のAPIキー/期限/read-write/失効/上限、公開v1のRSS/記事サブセット。
- 運用: 非root/read-only/権限削減のWeb/worker、volume、health、停止、整合バックアップ/別volume復元の生成データ検証。Tunnel用overrideはportを非公開にする構成例で、実接続は未検証。

現在のDBは **Feeder schema v7**。既知のFeeder v1〜v6は旧バージョンのバックアップを作って段階更新し、未知のschemaと既存バックアップの上書きを拒否する。これは旧FeedAstrolabe DBを変換する仕組みではない。旧DBの直接mount、無条件schema stamp、列/行の切捨てで起動を通す修正をしない。

## 主要コードの入口

- `backend/src/main.rs`: DB認識/移行・起動・production router。productionではauth wrapperを通る。unit helperのappは内部ハンドラー検証用で、認証を証明する経路ではない。
- `reader.rs`: 内部フィルター/一覧/訪問/一括状態。`public_api.rs`: 独立した公開v1のID順API。両者の順序と継続を混同しない。
- `auth.rs`: session/CSRF/キー/hash/期限/権限/回数とlocal/tunnel境界。`settings.rs`: 表示設定とallowlistによる読取専用export。
- `collection.rs` / `discovery.rs` / `opml.rs`: 安全なネットワーク取得・保存・worker・確認・購読入出力。
- `frontend/src/App.tsx`: 現状のreader状態/追加読込。`apiFetch.ts`: CSRF取得/送信。403時はcacheを捨てるが変更要求を自動再送しない。
- `LibrarySidebar.tsx` / `SidebarWidth.tsx` / `ReaderSettingsPanel.tsx` / `ApiKeysPanel.tsx`: source操作・表示・設定・キー管理。
- `scripts/container-smoke.py`: UUIDの専用Compose/volumeだけを作成・削除する統合検証。実環境のvolumeに`down --volumes`を向けない。

## 確認済みと未確認

実装基準 `f942dad` で、Linux内Rust39テスト、fmt/Clippy、本番Reactビルド、5ブラウザ検証（読書/生産性/配置/設定/APIアクセス）、復元/再作成、OpenSpec17項目が通過。旧DB検査ツール4テストも成功。この引き継ぎで追加する提案は未実装で、以前の検証に含まれない。

GitHub Actionsは定義・push済みだが成功結果は未取得。匿名のGitHub APIは404を返した。これはCI失敗やrepo不存在の証明ではない。Cursorでは認証済みの [Actions](https://github.com/negitanu/Feeder/actions?query=branch%3Adev) で対象SHAを確認し、読めなければ未確認と記録してローカル検証を行う。実Cloudflare、有料API、旧Webの実DB変換、研究/根拠/外部連携、本番切替は未実施。

履歴の検証文書には、その段階では未実装だったtimezone/順序/認証などが残っている。現在の優先順位はこの入口とbacklog、各最新の本仕様/後続検証を参照する。全体計画 `plan-web-migration/tasks.md` の大項目は完全互換/切替まで含むため未完了を維持している。完了率は推定しない。

## 再現する確認

正式な受入はLinuxコンテナ。Docker Engine/Compose v2、OpenSpec CLIを用意する。ブラウザ検証用にはNode22とPlaywrightのChromiumが必要（実アプリのruntimeイメージにNodeは含まない）。

```sh
git status --short --branch
openspec list --json
openspec validate --all --strict
npm --prefix frontend ci
npx --prefix frontend playwright install chromium
docker build --target backend-test .
docker compose --env-file /dev/null build
FEEDER_BROWSER_SMOKE=1 FEEDER_PRODUCTIVITY_SMOKE=1 FEEDER_SIDEBAR_SMOKE=1 FEEDER_SETTINGS_SMOKE=1 FEEDER_ACCESS_SMOKE=1 python3 scripts/container-smoke.py
python3 -m unittest scripts/test_inspect_legacy_db.py
git diff --check
```

CIのLinuxランナーではChromium導入に`--with-deps`を使っている。ホストにRustを入れて確認する場合はcargo fmt/test/clippyを `--manifest-path backend/Cargo.toml` で実行する。この環境のcargoは `/Users/tkobayashi/.cargo/bin/cargo`、別環境へこの絶対パスを強制しない。

## Cursorの最初の作業

P0-01の基準線を確認後、`openspec status --change reader-stable-continuation --json` と `openspec instructions apply --change reader-stable-continuation --json` を読み、9件の未実装タスクから進める。設計に残した初回cursorパラメーター等を具体化し、失敗を再現するfixtureから始める。研究機能やUI全面刷新を同じ変更へ混ぜない。

引き継ぎ用の貼り付け文は [Cursorへの開始指示](cursor-start-prompt.md)。この準備作業ではCursorのアプリ起動・チャット作成・他エージェントへの送信は行っていない。
