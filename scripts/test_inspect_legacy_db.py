import importlib.util
import json
import sqlite3
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("inspect-legacy-db.py")
SPEC = importlib.util.spec_from_file_location("inspect_legacy_db", SCRIPT)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class InspectLegacyDatabaseTests(unittest.TestCase):
    def setUp(self):
        self.temp_dir = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp_dir.cleanup)
        self.root = Path(self.temp_dir.name)

    def create_legacy_db(self, path):
        with sqlite3.connect(path) as db:
            db.executescript("""
                PRAGMA application_id=1179407187;
                PRAGMA user_version=19;
                CREATE TABLE feeds (id INTEGER PRIMARY KEY, url TEXT NOT NULL);
                CREATE TABLE articles (
                    id INTEGER PRIMARY KEY,
                    feed_id INTEGER NOT NULL REFERENCES feeds(id),
                    is_read INTEGER NOT NULL DEFAULT 0,
                    is_starred INTEGER NOT NULL DEFAULT 0,
                    payload BLOB
                );
            """)
            db.execute("INSERT INTO feeds(url) VALUES(?)", ("https://secret.invalid/private",))
            db.execute("INSERT INTO articles(feed_id,payload) VALUES(1,?)", (b"secret blob bytes",))

    def test_reports_metadata_without_changing_database(self):
        path = self.root / "legacy.sqlite3"
        self.create_legacy_db(path)
        before = path.read_bytes()
        result = MODULE.inspect_database(path)
        self.assertEqual(result["user_version"], 19)
        self.assertEqual(result["application_id"], 1179407187)
        self.assertEqual(result["tables"], {"articles": 1, "feeds": 1})
        self.assertEqual(result["integrity_check"], {"ok": True, "issue_count": 0})
        self.assertEqual(result["foreign_key_check"], {"ok": True, "violation_count": 0})
        self.assertEqual(path.read_bytes(), before)

        completed = subprocess.run(
            [sys.executable, str(SCRIPT), str(path)], capture_output=True, text=True
        )
        self.assertEqual(completed.returncode, 0)
        self.assertNotIn("secret", completed.stdout)
        self.assertNotIn("private", completed.stdout)
        self.assertNotIn("payload", completed.stdout)
        self.assertEqual(json.loads(completed.stdout)["tables"]["articles"], 1)

    def test_missing_path_is_not_created(self):
        path = self.root / "missing.sqlite3"
        with self.assertRaisesRegex(ValueError, "does not exist"):
            MODULE.inspect_database(path)
        self.assertFalse(path.exists())
        completed = subprocess.run(
            [sys.executable, str(SCRIPT), str(path)], capture_output=True, text=True
        )
        self.assertEqual(completed.returncode, 2)
        self.assertEqual(json.loads(completed.stderr), {"error": "database path does not exist"})
        self.assertFalse(path.exists())

    def test_corrupt_database_returns_clean_error(self):
        path = self.root / "corrupt.sqlite3"
        path.write_bytes(b"this is not sqlite content")
        completed = subprocess.run(
            [sys.executable, str(SCRIPT), str(path)], capture_output=True, text=True
        )
        self.assertEqual(completed.returncode, 2)
        self.assertEqual(json.loads(completed.stderr)["error"],
                         "unable to inspect database (invalid or corrupt SQLite file)")
        self.assertEqual(path.read_bytes(), b"this is not sqlite content")

    def test_reads_committed_wal_content_without_mutation(self):
        path = self.root / "wal.sqlite3"
        db = sqlite3.connect(path)
        db.execute("PRAGMA journal_mode=WAL")
        db.execute("CREATE TABLE articles (id INTEGER PRIMARY KEY)")
        db.commit()
        db.execute("INSERT INTO articles DEFAULT VALUES")
        db.commit()
        wal_path = Path(str(path) + "-wal")
        shm_path = Path(str(path) + "-shm")
        self.assertTrue(wal_path.exists())
        before_db = path.read_bytes()
        before_wal = wal_path.read_bytes()
        result = MODULE.inspect_database(path)
        self.assertEqual(result["tables"], {"articles": 1})
        self.assertEqual(path.read_bytes(), before_db)
        self.assertEqual(wal_path.read_bytes(), before_wal)
        self.assertTrue(shm_path.exists())
        db.close()


if __name__ == "__main__":
    unittest.main()
