# Inspecting a legacy SQLite database

Use the inspection script to collect schema and record-count metadata before planning a migration. It does not migrate data and never prints row contents, API keys, or blob bytes.

```sh
python3 scripts/inspect-legacy-db.py /path/to/legacy.sqlite3
```

The path must already exist and be a regular file. The tool opens SQLite in read-only mode, enables `query_only`, and reads through the normal SQLite WAL handling so committed WAL transactions are included. It does not use immutable mode, checkpoint, or create a database. Standard output is JSON containing `user_version`, `application_id`, per-table row counts, and only the pass/fail and issue counts for SQLite integrity and foreign-key checks. Errors are returned as a short JSON object on standard error with a nonzero exit status.
