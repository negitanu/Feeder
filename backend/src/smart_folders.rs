// Retain the legacy table so existing databases and backups remain compatible.
pub fn migrate(tx: &rusqlite::Transaction<'_>) -> rusqlite::Result<()> {
    tx.execute("CREATE TABLE smart_folders (id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE, snapshot TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision > 0))", [])?;
    Ok(())
}
