"""Attach generated demo articles to a synthetic feed for browser tests only."""
import sqlite3
import sys
from pathlib import Path
path=Path(sys.argv[1])
if not path.is_file(): raise SystemExit('Test database must already exist')
with sqlite3.connect(path) as db:
    if db.execute('PRAGMA application_id').fetchone()[0] != 0x46454544:
        raise SystemExit('Not a Feeder test database')
    if db.execute('SELECT count(*) FROM articles').fetchone()[0] != 36 or db.execute("SELECT count(*) FROM articles WHERE title LIKE 'サンプル記事 %'").fetchone()[0] != 36:
        raise SystemExit('Only a dedicated generated demo database is accepted')
    db.execute("INSERT INTO feeds(url,title,paused) VALUES ('https://example.com/feeder-test.xml','テスト購読',0)")
    feed_id=db.execute('SELECT last_insert_rowid()').fetchone()[0]
    db.execute("UPDATE articles SET feed_id=?,external_id='fixture-'||id WHERE id<=18",(feed_id,))
