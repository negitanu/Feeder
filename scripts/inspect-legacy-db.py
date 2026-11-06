#!/usr/bin/env python3
"""Read-only, metadata-only inspection of a legacy Feeder SQLite database."""

import argparse
import json
import sqlite3
import sys
from pathlib import Path
from urllib.parse import quote


def inspect_database(path: Path) -> dict:
    if not path.exists():
        raise ValueError("database path does not exist")
    if not path.is_file():
        raise ValueError("database path is not a regular file")

    # mode=ro prevents accidental creation or writes. Do not set immutable=1:
    # SQLite must see committed WAL content for a consistent inspection.
    uri = "file:" + quote(str(path.resolve()), safe="/") + "?mode=ro"
    connection = sqlite3.connect(uri, uri=True, timeout=5)
    try:
        connection.execute("PRAGMA query_only=ON")
        connection.execute("PRAGMA trusted_schema=OFF")
        connection.execute("BEGIN")
        user_version = connection.execute("PRAGMA user_version").fetchone()[0]
        application_id = connection.execute("PRAGMA application_id").fetchone()[0]
        table_names = [
            row[0]
            for row in connection.execute(
                "SELECT name FROM sqlite_master "
                "WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name"
            )
        ]
        table_counts = {}
        for name in table_names:
            # Names came from sqlite_master; quoting protects unusual identifiers.
            quoted_name = '"' + name.replace('"', '""') + '"'
            table_counts[name] = connection.execute(
                f"SELECT COUNT(*) FROM {quoted_name}"
            ).fetchone()[0]

        integrity_issue_count = sum(
            1 for row in connection.execute("PRAGMA integrity_check") if row[0] != "ok"
        )
        foreign_key_violation_count = sum(
            1 for _ in connection.execute("PRAGMA foreign_key_check")
        )
        return {
            "user_version": user_version,
            "application_id": application_id,
            "tables": table_counts,
            "integrity_check": {
                "ok": integrity_issue_count == 0,
                "issue_count": integrity_issue_count,
            },
            "foreign_key_check": {
                "ok": foreign_key_violation_count == 0,
                "violation_count": foreign_key_violation_count,
            },
        }
    finally:
        connection.close()


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(
        description="Inspect SQLite schema and row counts without exposing row contents."
    )
    parser.add_argument("database", type=Path, help="existing SQLite database file")
    args = parser.parse_args(argv)
    try:
        result = inspect_database(args.database)
    except (OSError, ValueError, sqlite3.Error) as exc:
        # SQLite exception text can include database details; emit only a safe category.
        if isinstance(exc, ValueError):
            message = str(exc)
        elif isinstance(exc, sqlite3.DatabaseError):
            message = "unable to inspect database (invalid or corrupt SQLite file)"
        else:
            message = "unable to access database file"
        print(json.dumps({"error": message}), file=sys.stderr)
        return 2
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
