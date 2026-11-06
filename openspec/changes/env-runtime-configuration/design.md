## Decisions

- Compose uses its normal project `.env` interpolation; shell variables take precedence, and `--env-file` permits an explicit file. The Rust process does not independently parse dotenv files. Restart/recreate the relevant containers to apply edits.
- Existing `compose.tunnel.yml` remains the protected secret-file option. A separate `compose.tunnel-env.yml`, used with the base compose only, supplies the connector `TUNNEL_TOKEN` via its environment. Both use an unpublished web port, pinned connector image, read-only filesystem and existing Access boundary. Never enable both overrides together.
- Base Compose passes optional `OPENROUTER_API_KEY` and `ORCAROUTER_API_KEY` to Web and worker to match legacy provisioning. They are not consumed until a separately specified provider is implemented; blank/unset credentials do not enable a provider. They must not reach the browser, renderer, database, exports, application logs, or build context.
- Environment values are visible to privileged users via container inspection and Compose configuration output; documentation must say this and avoid suggesting that `.env` is encrypted. The existing file-based tunnel option remains available. No implicit real `.env` is used by automated tests.
- Feeder's own API bearer keys continue to be generated and managed through its existing interface, with hashes in SQLite. A client may store its issued bearer key in its own environment; no implicit bootstrap or bypass is added to the server.
- Validation checks both tunnel variants from synthetic env files, never print resolved configurations or secret values, and keep the existing security checks. The public tunnel still depends on Cloudflare Access and actual external acceptance.

## Validation and rollback

Use synthetic canaries and isolated Compose configuration; inspect outputs only programmatically and never print them. Run production-router/browser export and Linux container regression, test both token modes and precedence, and check that `.env` remains ignored and excluded from image context. Revert Compose/docs changes to return to file-only tunnel mode; no database migration is needed.
