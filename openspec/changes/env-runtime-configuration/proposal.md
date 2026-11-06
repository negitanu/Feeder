## Why

FeedAstrolabe operators use a local `.env` for Compose configuration, Cloudflare `TUNNEL_TOKEN`, and optional `OPENROUTER_API_KEY`/`ORCAROUTER_API_KEY`. Feeder currently documents only a separate tunnel-token file and does not pass these provider credentials to application services. The requested configuration should work without copying real secrets into the repository.

## What Changes

- Document and test Docker Compose `.env` configuration and an alternate tunnel override using `TUNNEL_TOKEN` from `.env`.
- Pass the two legacy provider environment variables to Web and worker, as preparation for later provider implementations. The variables do not enable AI or imply that a provider is implemented.
- Preserve the existing tunnel secret-file override and browser-issued, hashed, revocable Feeder API keys.
- Make tunnel configuration checks honor Compose's normal `.env` behavior or an explicitly specified env file without printing secret values.

## Legacy correspondence and scope

Reference: `../FeedAstrolabe/.env.example` and `../FeedAstrolabe/compose.yml`. The legacy `TUNNEL_TOKEN`, `OPENROUTER_API_KEY` and `ORCAROUTER_API_KEY` names are retained. Legacy Ollama and delivery files, AI providers and application-side Cloudflare JWT are not implemented by this change. No legacy `.env`, production data or token is read during tests.
