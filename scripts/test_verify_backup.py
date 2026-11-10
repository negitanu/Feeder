import contextlib
import importlib.util
import io
import sqlite3
import tarfile
import tempfile
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location('verify_backup', Path(__file__).with_name('verify-backup.py'))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)

class VerifyBackupTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.db = self.root / 'fixture.sqlite3'
        with contextlib.closing(sqlite3.connect(self.db)) as conn:
            conn.executescript('''CREATE TABLE feeds(id INTEGER PRIMARY KEY);
                CREATE TABLE articles(id INTEGER PRIMARY KEY, feed_id INTEGER REFERENCES feeds(id));
                CREATE TABLE collection_settings(id INTEGER PRIMARY KEY);
                INSERT INTO feeds VALUES(1); INSERT INTO articles VALUES(1,1); PRAGMA user_version=10;''')
        self.archive = self.root / 'fixture.tar.gz'

    def tearDown(self):
        self.tmp.cleanup()

    def write_archive(self, entries):
        with tarfile.open(self.archive, 'w:gz') as tar:
            for name, body, kind in entries:
                entry = tarfile.TarInfo(name)
                entry.type = kind
                entry.size = len(body) if kind == tarfile.REGTYPE else 0
                if kind == tarfile.SYMTYPE: entry.linkname = 'elsewhere'
                tar.addfile(entry, io.BytesIO(body) if entry.isfile() else None)

    def db_entry(self):
        return ('./feeder.sqlite3', self.db.read_bytes(), tarfile.REGTYPE)

    def verify(self):
        with contextlib.redirect_stdout(io.StringIO()):
            module.verify(self.archive)

    def test_valid_archive_and_source_unchanged(self):
        self.write_archive([('.', b'', tarfile.DIRTYPE), self.db_entry(), ('attachments/note', b'fixture', tarfile.REGTYPE)])
        before = self.archive.read_bytes()
        self.verify()
        self.assertEqual(before, self.archive.read_bytes())

    def test_path_traversal_absolute_duplicate_and_links_rejected(self):
        for extra in [('../escape', b'x', tarfile.REGTYPE), ('/absolute', b'x', tarfile.REGTYPE),
                      ('feeder.sqlite3', self.db.read_bytes(), tarfile.REGTYPE), ('link', b'', tarfile.SYMTYPE),
                      ('hardlink', b'', tarfile.LNKTYPE)]:
            with self.subTest(extra=extra[0]):
                self.write_archive([self.db_entry(), extra])
                with self.assertRaises(ValueError): self.verify()

    def test_wal_missing_empty_corrupt_and_foreign_key_rejected(self):
        for entries in [[self.db_entry(), ('feeder.sqlite3-wal', b'pending', tarfile.REGTYPE)],
                        [self.db_entry(), ('feeder.sqlite3-journal', b'pending', tarfile.REGTYPE)],
                        [('other', b'fixture', tarfile.REGTYPE)], [('feeder.sqlite3', b'', tarfile.REGTYPE)],
                        [('feeder.sqlite3', b'not sqlite', tarfile.REGTYPE)]]:
            with self.subTest(entries=entries[0][0]):
                self.write_archive(entries)
                with self.assertRaises((ValueError, sqlite3.DatabaseError)): self.verify()
        with contextlib.closing(sqlite3.connect(self.db)) as conn:
            conn.execute('INSERT INTO articles VALUES(2,999)')
            conn.commit()
        self.write_archive([self.db_entry()])
        with self.assertRaisesRegex(ValueError, '外部キー'): self.verify()

    def test_duplicate_normalized_names_rejected(self):
        self.write_archive([self.db_entry(), ('./folder/../escape', b'x', tarfile.REGTYPE)])
        with self.assertRaises(ValueError): self.verify()

    def test_missing_tables_rejected(self):
        with contextlib.closing(sqlite3.connect(self.db)) as conn: conn.execute('DROP TABLE collection_settings')
        self.write_archive([self.db_entry()])
        with self.assertRaisesRegex(ValueError, 'Feeder'): self.verify()

    def test_truncated_gzip_and_crc_rejected(self):
        self.write_archive([self.db_entry()])
        original = self.archive.read_bytes()
        for broken in (original[:-5], original[:-8] + b'\x00\x00\x00\x00' + original[-4:]):
            self.archive.write_bytes(broken)
            with self.assertRaises((EOFError, OSError)): self.verify()

if __name__ == '__main__': unittest.main()
