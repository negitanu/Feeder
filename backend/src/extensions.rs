//! Additive Reader storage. Build the expected schema independently before trusting a file.
use super::*;

pub fn migrate(tx: &rusqlite::Transaction<'_>) -> rusqlite::Result<()> {
    digest::migrate(tx)?;
    duplicates::migrate(tx)?;
    smart_folders::migrate(tx)?;
    social::migrate(tx)?;
    read_later::migrate(tx)?;
    mute_rules::migrate(tx)?;
    collection_health::migrate(tx)?;
    Ok(())
}

pub fn recognized_v9(objects: &[(String, String, Option<String>)]) -> bool {
    let expected = || -> rusqlite::Result<Vec<(String, String, Option<String>)>> {
        let mut db = Connection::open_in_memory()?;
        db.execute(SCHEMA, [])?;
        let tx = db.transaction()?;
        collection::migrate(&tx)?;
        tx.execute(reader::VISITS, [])?;
        collection::migrate_order(&tx)?;
        settings::migrate(&tx)?;
        auth::migrate(&tx)?;
        collection::migrate_intervals(&tx)?;
        migrate(&tx)?;
        tx.commit()?;
        db.prepare(
            "SELECT type,name,sql FROM sqlite_master WHERE name NOT LIKE 'sqlite_%' ORDER BY name",
        )?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect()
    };
    expected().is_ok_and(|expected| expected == objects)
}

#[cfg(test)]
pub fn drop_for_legacy_fixture(db: &Connection) {
    db.execute_batch("DROP TABLE article_read_later; DROP TABLE read_later_settings; DROP TABLE smart_folders; DROP TABLE social_sources; DROP TABLE mute_rules; DROP TABLE collection_health;").unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v8_upgrade_preserves_library_states_settings_and_backup() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("feeder.db");
        let db = open_db(&path, false).unwrap();
        drop_for_legacy_fixture(&db);
        db.pragma_update(None, "user_version", 8).unwrap();
        db.execute_batch("INSERT INTO folders(id,name) VALUES(10,'Synthetic folder'); INSERT INTO feeds(id,url,title,folder_id,paused,last_checked,last_error) VALUES(20,'https://example.invalid/rss','Synthetic Publisher',10,1,'2026-11-07 01:00:00',NULL); INSERT INTO articles(id,title,content,url,published,is_read,is_starred,feed_id,external_id) VALUES(30,'Synthetic title','Body','https://example.invalid/a','2026-11-07T00:00:00Z',1,1,20,'external'); INSERT INTO article_visits VALUES(30,'2026-11-07T02:00:00Z'); INSERT INTO feed_poll_intervals VALUES(20,120); UPDATE collection_settings SET enabled=0; UPDATE feed_order SET position=7 WHERE feed_id=20;").unwrap();
        drop(db);
        let upgraded = open_db(&path, false).unwrap();
        assert_eq!(
            upgraded
                .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
                .unwrap(),
            9
        );
        let item = lookup(&upgraded, 30).unwrap();
        assert!(item.is_read && item.is_starred && !item.is_saved_for_later);
        assert_eq!(item.feed_title.as_deref(), Some("Synthetic Publisher"));
        assert_eq!(item.last_visited.as_deref(), Some("2026-11-07T02:00:00Z"));
        assert_eq!(upgraded.query_row("SELECT paused,position,interval_seconds FROM feeds JOIN feed_order ON feed_order.feed_id=feeds.id JOIN feed_poll_intervals ON feed_poll_intervals.feed_id=feeds.id WHERE feeds.id=20",[],|r|Ok((r.get::<_,bool>(0)?,r.get::<_,i64>(1)?,r.get::<_,i64>(2)?))).unwrap(),(true,7,120));
        assert!(
            !upgraded
                .query_row("SELECT enabled FROM collection_settings", [], |r| r
                    .get::<_, bool>(0))
                .unwrap()
        );
        assert_eq!(
            upgraded
                .query_row("PRAGMA foreign_key_check", [], |_| Ok(()))
                .optional()
                .unwrap(),
            None
        );
        drop(upgraded);
        let backup = Connection::open_with_flags(
            path.with_extension("v8-backup.sqlite3"),
            OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        assert_eq!(
            backup
                .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
                .unwrap(),
            8
        );
        assert_eq!(
            backup
                .query_row(
                    "SELECT is_read,is_starred FROM articles WHERE id=30",
                    [],
                    |r| Ok((r.get::<_, bool>(0)?, r.get::<_, bool>(1)?))
                )
                .unwrap(),
            (true, true)
        );
        assert!(open_db(&path, false).is_ok());
    }

    #[test]
    fn v9_backup_restore_retains_reader_extensions_and_constraints() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("source.db");
        let source = open_db(&path, false).unwrap();
        source.execute_batch("INSERT INTO feeds(id,url,title,paused) VALUES(1,'https://example.invalid/@synthetic','Synthetic social',1); INSERT INTO articles(id,title,content,url,published,feed_id,is_read,is_starred) VALUES(2,'Synthetic','Body','https://example.invalid/post','2026-11-07T00:00:00Z',1,1,1); INSERT INTO article_visits VALUES(2,'2026-11-07T01:00:00Z'); INSERT INTO article_read_later VALUES(2,'2026-11-07T02:00:00Z'); UPDATE read_later_settings SET auto_remove=1; INSERT INTO mute_rules(name,term,field,feed_id,enabled,revision) VALUES('Synthetic rule','Body','body',1,1,3); INSERT INTO social_sources VALUES(1,'mastodon','synthetic','https://example.invalid','123'); INSERT INTO collection_health VALUES(1,'2026-11-06 01:00:00',2); INSERT INTO smart_folders(name,snapshot,revision) VALUES('Synthetic shared','{\"filter\":\"later\",\"query\":\"\",\"source\":null,\"applied\":{\"order\":\"new\",\"scope\":\"both\"}}',4);").unwrap();
        let restored_path = dir.path().join("restore.db");
        {
            let mut target = Connection::open(&restored_path).unwrap();
            rusqlite::backup::Backup::new(&source, &mut target)
                .unwrap()
                .run_to_completion(100, std::time::Duration::from_millis(1), None)
                .unwrap();
        }
        let restored = open_db(&restored_path, false).unwrap();
        let a = lookup(&restored, 2).unwrap();
        assert!(a.is_read && a.is_starred && a.is_saved_for_later);
        for table in [
            "article_visits",
            "article_read_later",
            "read_later_settings",
            "smart_folders",
            "social_sources",
            "mute_rules",
            "collection_health",
            "feed_order",
            "collection_settings",
            "reader_settings",
        ] {
            // Compare all typed cell values rather than a row count alone.
            let snapshot = |db: &Connection| {
                let mut stmt = db
                    .prepare(&format!("SELECT * FROM {table} ORDER BY 1"))
                    .unwrap();
                let columns = stmt.column_count();
                stmt.query_map([], |row| {
                    (0..columns)
                        .map(|i| row.get::<_, rusqlite::types::Value>(i))
                        .collect::<rusqlite::Result<Vec<_>>>()
                })
                .unwrap()
                .collect::<rusqlite::Result<Vec<_>>>()
                .unwrap()
            };
            assert_eq!(snapshot(&source), snapshot(&restored), "{table}");
        }
        assert_eq!(
            restored
                .query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
                .unwrap(),
            "ok"
        );
        restored
            .execute("DELETE FROM feeds WHERE id=1", [])
            .unwrap();
        for table in [
            "articles",
            "article_read_later",
            "article_visits",
            "social_sources",
            "mute_rules",
            "collection_health",
        ] {
            assert_eq!(
                restored
                    .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r
                        .get::<_, i64>(0))
                    .unwrap(),
                0
            );
        }
    }

    #[test]
    fn unknown_v9_schema_is_refused_without_file_mutation() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("unknown.db");
        let db = open_db(&path, false).unwrap();
        db.execute("CREATE TABLE unexpected(secret TEXT)", [])
            .unwrap();
        drop(db);
        let before = std::fs::read(&path).unwrap();
        assert!(open_db(&path, false).is_err());
        assert_eq!(before, std::fs::read(&path).unwrap());
        assert!(!path.with_extension("v9-backup.sqlite3").exists());
    }
}
