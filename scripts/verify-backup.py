#!/usr/bin/env python3
"""Read-only verification of a stopped Feeder /data tar archive."""
import argparse
import contextlib
import gzip
import shutil
import sqlite3
import sys
import tarfile
import tempfile
import zlib
from pathlib import Path, PurePosixPath

MAX_DATABASE_BYTES = 4 * 1024**3

def verify(archive: Path) -> None:
    # tarfile may stop at the tar end marker without consuming the gzip trailer.
    # Read the compressed stream to EOF so truncated data and a bad CRC fail.
    with archive.open('rb') as source:
        is_gzip = source.read(2) == b'\x1f\x8b'
    if is_gzip:
        with gzip.open(archive, 'rb') as compressed:
            while compressed.read(1024 * 1024):
                pass
    with tempfile.TemporaryDirectory(prefix='feeder-backup-verify-') as temporary:
        database = Path(temporary) / 'feeder.sqlite3'
        seen = set()
        found = False
        with tarfile.open(archive, 'r:*') as tar:
            for entry in tar:
                path = PurePosixPath(entry.name)
                if path.is_absolute() or '..' in path.parts or not (entry.isfile() or entry.isdir()):
                    raise ValueError('危険なパスまたはリンクを含むアーカイブです')
                name = str(path)
                if name in seen:
                    raise ValueError('同じパスが複数含まれています')
                seen.add(name)
                if name in ('feeder.sqlite3-wal', 'feeder.sqlite3-shm', 'feeder.sqlite3-journal') and entry.size:
                    raise ValueError('SQLiteの状態ファイルが含まれています。Webとworkerを停止して再取得してください')
                if name == 'feeder.sqlite3':
                    if not entry.isfile() or not 0 < entry.size <= MAX_DATABASE_BYTES:
                        raise ValueError('DBが空か検証可能なサイズを超えています')
                    source = tar.extractfile(entry)
                    if source is None:
                        raise ValueError('DBを読み取れません')
                    with source, database.open('wb') as target:
                        shutil.copyfileobj(source, target)
                    found = True
        if not found:
            raise ValueError('feeder.sqlite3がありません')
        with contextlib.closing(sqlite3.connect(f'{database.as_uri()}?mode=ro', uri=True)) as db:
            if db.execute('PRAGMA integrity_check').fetchall() != [('ok',)]:
                raise ValueError('SQLite整合性チェックに失敗しました')
            if db.execute('PRAGMA foreign_key_check').fetchone() is not None:
                raise ValueError('外部キー整合性チェックに失敗しました')
            tables = {r[0] for r in db.execute("SELECT name FROM sqlite_master WHERE type='table'")}
            if not {'feeds', 'articles', 'collection_settings'}.issubset(tables):
                raise ValueError('FeederのDBではありません')
            version = db.execute('PRAGMA user_version').fetchone()[0]
        print(f'検証成功: SQLite整合性・外部キー・必須テーブル（schema {version}）')

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('archive', type=Path)
    args = parser.parse_args()
    try:
        verify(args.archive)
    except (OSError, EOFError, ValueError, sqlite3.Error, tarfile.TarError, zlib.error) as error:
        print(f'検証失敗: {error}', file=sys.stderr)
        sys.exit(1)
