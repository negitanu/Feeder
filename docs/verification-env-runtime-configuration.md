# `.env` 設定の検証

2026-11-06。移行元の `.env.example` と Compose 定義を参照し、Feeder の Web・worker・Tunnel 構成を合成データで確認した。移行元や本番の `.env` と秘密情報は使用していない。

- 一時ディレクトリーの合成 `.env` と明示的な `--env-file` により、ポート、外部 AI 用キー、シェル変数の優先、単一引用符内の `$`、両方の Tunnel token 方式、token 欠落時の拒否を検証した。`.env` の Git 除外とビルドコンテキストの除外も確認した。
- Linux の専用 Compose プロジェクトに合成キーを渡し、Web・workerへの設定、Feeder APIでの拒否、画面と JavaScript、設定エクスポート、SQLite、ログへの非露出を確認した。読書・生産性・配置・設定・APIアクセスのブラウザ回帰、非root/read-only、バックアップ・別volume復元にも成功した。
- GitHub CIの実行結果は確認していない。実際の Cloudflare Tunnel 接続、外部ドメインの Access ポリシー、OpenRouter/OrcaRouter との通信は未実施。AIプロバイダー自体は未実装。

環境変数方式はコンテナの設定を参照できる利用者に値が見える。トークンファイル方式も残し、運用上の選択肢としている。`.env` は暗号化されないため、所有者のみ読める権限で保管する。
