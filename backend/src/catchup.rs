// Keep the legacy tables for existing databases and backups.
pub fn migrate_catchup(tx: &rusqlite::Transaction<'_>) -> rusqlite::Result<()> {
    tx.execute_batch("CREATE TABLE catchup_preferences (id INTEGER PRIMARY KEY CHECK(id=1), revision INTEGER NOT NULL CHECK(revision>0), terms TEXT NOT NULL);\
        INSERT INTO catchup_preferences VALUES(1,1,'[]');\
        CREATE TABLE catchup_priority_feeds (feed_id INTEGER PRIMARY KEY REFERENCES feeds(id) ON DELETE CASCADE, position INTEGER NOT NULL UNIQUE);\
        CREATE TRIGGER catchup_priority_feed_removed AFTER DELETE ON catchup_priority_feeds WHEN NOT EXISTS(SELECT 1 FROM feeds WHERE id=OLD.feed_id) BEGIN UPDATE catchup_preferences SET revision=revision+1 WHERE id=1; END;\
        CREATE TABLE catchup_acknowledgements (article_id INTEGER NOT NULL REFERENCES articles(id) ON DELETE CASCADE, revision INTEGER NOT NULL CHECK(revision>0), fingerprint TEXT NOT NULL, action TEXT NOT NULL CHECK(action IN ('confirmed','skipped')), PRIMARY KEY(article_id,revision,fingerprint));")
}
