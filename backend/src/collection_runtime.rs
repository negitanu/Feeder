//! Additional v10 collection state. The v9 migration must not call this module.
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::{Deserialize, Serialize};

pub fn migrate(tx: &Transaction<'_>) -> rusqlite::Result<()> {
    tx.execute_batch("CREATE TABLE collection_runtime (feed_id INTEGER PRIMARY KEY REFERENCES feeds(id) ON DELETE CASCADE, cursor TEXT, high_water TEXT, cycle_head TEXT, next_retry INTEGER, more_pending INTEGER NOT NULL DEFAULT 0 CHECK(more_pending IN (0,1)), gap_possible INTEGER NOT NULL DEFAULT 0 CHECK(gap_possible IN (0,1)), recovery INTEGER NOT NULL DEFAULT 0 CHECK(recovery IN (0,1)), cursor_guard TEXT);\
    CREATE TRIGGER collection_runtime_feed_url_reset AFTER UPDATE OF url ON feeds WHEN OLD.url IS NOT NEW.url BEGIN DELETE FROM collection_runtime WHERE feed_id=NEW.id; DELETE FROM collection_health WHERE feed_id=NEW.id; END;\
    CREATE TRIGGER collection_runtime_social_source_reset AFTER UPDATE OF provider,server,remote_id ON social_sources WHEN OLD.provider IS NOT NEW.provider OR OLD.server IS NOT NEW.server OR OLD.remote_id IS NOT NEW.remote_id BEGIN DELETE FROM collection_runtime WHERE feed_id=NEW.feed_id; DELETE FROM collection_health WHERE feed_id=NEW.feed_id; UPDATE feeds SET etag=NULL,modified=NULL,last_checked=NULL,last_error=NULL WHERE id=NEW.feed_id; END;")
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Runtime {
    pub cursor: Option<String>,
    pub high_water: Option<String>,
    pub cycle_head: Option<String>,
    pub next_retry: Option<i64>,
    pub more_pending: bool,
    pub gap_possible: bool,
    pub recovery: bool,
    pub cursor_guard: Option<CursorGuard>,
}

/// Brent's cycle detector uses constant space and survives page-budget boundaries.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CursorGuard {
    anchor: String,
    power: u64,
    distance: u64,
}

impl Runtime {
    pub fn advance_cursor(&mut self, next: String) -> bool {
        if self.cursor.as_ref() == Some(&next) {
            return false;
        }
        let Some(mut guard) = self.cursor_guard.clone().or_else(|| {
            self.cursor.as_ref().map(|current| CursorGuard {
                anchor: current.clone(),
                power: 1,
                distance: 0,
            })
        }) else {
            self.cursor_guard = Some(CursorGuard {
                anchor: next.clone(),
                power: 1,
                distance: 0,
            });
            self.cursor = Some(next);
            return true;
        };
        if guard.anchor == next {
            return false;
        }
        guard.distance = guard.distance.saturating_add(1);
        if guard.distance >= guard.power {
            guard.anchor = next.clone();
            guard.power = guard.power.saturating_mul(2);
            guard.distance = 0;
        }
        self.cursor_guard = Some(guard);
        self.cursor = Some(next);
        true
    }
}

pub fn load(db: &Connection, feed_id: i64) -> rusqlite::Result<Runtime> {
    Ok(db.query_row("SELECT cursor,high_water,cycle_head,next_retry,more_pending,gap_possible,recovery,cursor_guard FROM collection_runtime WHERE feed_id=?", [feed_id], |r| {
        let guard: Option<String> = r.get(7)?;
        let cursor_guard = guard.map(|json| serde_json::from_str(&json).map_err(|e| rusqlite::Error::FromSqlConversionFailure(7,rusqlite::types::Type::Text,Box::new(e)))).transpose()?;
        Ok(Runtime {cursor:r.get(0)?,high_water:r.get(1)?,cycle_head:r.get(2)?,next_retry:r.get(3)?,more_pending:r.get(4)?,gap_possible:r.get(5)?,recovery:r.get(6)?,cursor_guard})
    }).optional()?.unwrap_or_default())
}

/// Call only in the same transaction that verifies the live lease and stores entries.
pub fn save(tx: &Transaction<'_>, feed_id: i64, state: &Runtime) -> rusqlite::Result<()> {
    let guard = state
        .cursor_guard
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
    tx.execute("INSERT INTO collection_runtime(feed_id,cursor,high_water,cycle_head,next_retry,more_pending,gap_possible,recovery,cursor_guard) VALUES (?,?,?,?,?,?,?,?,?) ON CONFLICT(feed_id) DO UPDATE SET cursor=excluded.cursor,high_water=excluded.high_water,cycle_head=excluded.cycle_head,next_retry=excluded.next_retry,more_pending=excluded.more_pending,gap_possible=excluded.gap_possible,recovery=excluded.recovery,cursor_guard=excluded.cursor_guard", params![feed_id,state.cursor,state.high_water,state.cycle_head,state.next_retry,state.more_pending,state.gap_possible,state.recovery,guard])?;
    Ok(())
}

/// Backoff begins at one minute and is capped at one day.
pub fn backoff_seconds(failures: u32) -> i64 {
    (60_i64 * (1_i64 << failures.saturating_sub(1).min(11))).min(86400)
}

/// Conservative scope-level coverage: a missing successful collection, paused or
/// disabled collection, pending continuation, active retry, errors, possible gaps,
/// or an overdue success means the reader cannot assume complete coverage.
pub fn is_incomplete(
    db: &Connection,
    feed_id: Option<i64>,
    folder_id: Option<i64>,
) -> rusqlite::Result<bool> {
    db.query_row(r#"SELECT EXISTS(
      SELECT 1 FROM feeds f JOIN collection_settings settings ON settings.id=1
      LEFT JOIN collection_health h ON h.feed_id=f.id
      LEFT JOIN collection_runtime runtime ON runtime.feed_id=f.id
      LEFT JOIN feed_poll_intervals intervals ON intervals.feed_id=f.id
      LEFT JOIN social_sources social ON social.feed_id=f.id
      WHERE (?1 IS NULL OR f.id=?1) AND (?2 IS NULL OR f.folder_id=?2)
        AND (f.paused=1 OR settings.enabled=0 OR f.last_error IS NOT NULL
          OR COALESCE(h.consecutive_failures,0)>0
          OR COALESCE(runtime.next_retry>unixepoch(),0)
          OR COALESCE(runtime.more_pending,0) OR COALESCE(runtime.gap_possible,0)
          OR (social.feed_id IS NOT NULL AND runtime.feed_id IS NULL)
          OR COALESCE(h.last_success, CASE WHEN h.feed_id IS NULL AND f.last_error IS NULL THEN f.last_checked END) IS NULL
          OR COALESCE(unixepoch(COALESCE(h.last_success,CASE WHEN h.feed_id IS NULL AND f.last_error IS NULL THEN f.last_checked END)),0)
              <= unixepoch()-COALESCE(intervals.interval_seconds,settings.interval_seconds))
    )"#, params![feed_id,folder_id], |r| r.get(0))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectionContext {
    InitialBackfill,
    Normal,
    RecoveryBackfill,
}

/// Classify before committing the current response; new RSS feed creation uses
/// InitialBackfill directly. A social initial cycle stays initial across pages.
pub fn collection_context(db: &Connection, feed_id: i64) -> rusqlite::Result<CollectionContext> {
    db.query_row(
        r#"SELECT h.last_success, h.consecutive_failures, f.last_error,
       s.feed_id IS NOT NULL, r.high_water, r.recovery, f.last_checked,
       COALESCE(unixepoch(COALESCE(h.last_success,f.last_checked)) < unixepoch()-2*COALESCE(i.interval_seconds,settings.interval_seconds),0)
       FROM feeds f JOIN collection_settings settings ON settings.id=1
       LEFT JOIN feed_poll_intervals i ON i.feed_id=f.id LEFT JOIN collection_health h ON h.feed_id=f.id
       LEFT JOIN social_sources s ON s.feed_id=f.id
       LEFT JOIN collection_runtime r ON r.feed_id=f.id WHERE f.id=?"#,
        [feed_id],
        |r| {
            let success: Option<String> = r.get(0)?;
            let failures: Option<i64> = r.get(1)?;
            let error: Option<String> = r.get(2)?;
            let social: bool = r.get(3)?;
            let high_water: Option<String> = r.get(4)?;
            let recovery: Option<bool> = r.get(5)?;
            let last_checked: Option<String> = r.get(6)?;
            let overdue: bool = r.get(7)?;
            Ok(
                if (social && high_water.is_none())
                    || (!social && success.is_none() && last_checked.is_none())
                {
                    CollectionContext::InitialBackfill
                } else if failures.unwrap_or(0) > 0 || error.is_some() || recovery.unwrap_or(false) || overdue
                {
                    CollectionContext::RecoveryBackfill
                } else {
                    CollectionContext::Normal
                },
            )
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn additive_state_roundtrip_and_source_reset() {
        let mut db = Connection::open_in_memory().unwrap();
        db.execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE feeds(id INTEGER PRIMARY KEY,url TEXT,etag TEXT,modified TEXT,last_checked TEXT,last_error TEXT); CREATE TABLE collection_health(feed_id INTEGER PRIMARY KEY REFERENCES feeds(id) ON DELETE CASCADE); CREATE TABLE social_sources(feed_id INTEGER PRIMARY KEY REFERENCES feeds(id) ON DELETE CASCADE,provider TEXT,server TEXT,remote_id TEXT); INSERT INTO feeds VALUES(1,'https://example.invalid/a','tag','date','then',NULL); INSERT INTO social_sources VALUES(1,'mastodon','https://example.invalid','1'); INSERT INTO collection_health VALUES(1);").unwrap();
        let tx = db.transaction().unwrap();
        migrate(&tx).unwrap();
        let state = Runtime {
            cursor: Some("opaque".into()),
            high_water: Some("head".into()),
            cycle_head: Some("next".into()),
            next_retry: Some(123),
            more_pending: true,
            gap_possible: true,
            recovery: true,
            ..Default::default()
        };
        save(&tx, 1, &state).unwrap();
        assert_eq!(load(&tx, 1).unwrap(), state);
        tx.execute(
            "UPDATE social_sources SET remote_id='2' WHERE feed_id=1",
            [],
        )
        .unwrap();
        assert_eq!(load(&tx, 1).unwrap(), Runtime::default());
        assert_eq!(
            tx.query_row("SELECT count(*) FROM collection_health", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert_eq!(
            tx.query_row("SELECT etag FROM feeds", [], |r| r
                .get::<_, Option<String>>(0))
                .unwrap(),
            None
        );
        save(&tx, 1, &state).unwrap();
        tx.execute(
            "UPDATE feeds SET url='https://example.invalid/b' WHERE id=1",
            [],
        )
        .unwrap();
        assert_eq!(load(&tx, 1).unwrap(), Runtime::default());
        save(&tx, 1, &state).unwrap();
        tx.execute("DELETE FROM feeds WHERE id=1", []).unwrap();
        assert_eq!(
            tx.query_row("SELECT count(*) FROM collection_runtime", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
    #[test]
    fn bounded_backoff() {
        assert_eq!(backoff_seconds(1), 60);
        assert_eq!(backoff_seconds(2), 120);
        assert_eq!(backoff_seconds(u32::MAX), 86400);
    }
    #[test]
    fn scope_coverage_and_context_follow_real_state() {
        let dir = tempfile::tempdir().unwrap();
        let db = super::super::open_db(&dir.path().join("db"), false).unwrap();
        db.execute_batch("INSERT INTO folders(id,name) VALUES(1,'One'),(2,'Two'); INSERT INTO feeds(id,url,title,folder_id) VALUES(1,'https://example.invalid/1','One',1),(2,'https://example.invalid/2','Two',2); UPDATE collection_settings SET enabled=1;").unwrap();
        assert!(is_incomplete(&db, None, Some(1)).unwrap());
        assert_eq!(
            collection_context(&db, 1).unwrap(),
            CollectionContext::InitialBackfill
        );
        db.execute_batch("UPDATE feeds SET last_checked=datetime('now') WHERE id=1; INSERT INTO collection_health VALUES(1,datetime('now'),0);").unwrap();
        assert!(!is_incomplete(&db, Some(1), None).unwrap());
        assert!(is_incomplete(&db, None, Some(2)).unwrap());
        assert_eq!(
            collection_context(&db, 1).unwrap(),
            CollectionContext::Normal
        );
        db.execute(
            "INSERT INTO collection_runtime(feed_id,more_pending) VALUES(1,1)",
            [],
        )
        .unwrap();
        assert!(is_incomplete(&db, Some(1), None).unwrap());
        db.execute_batch(
            "UPDATE collection_runtime SET more_pending=0,recovery=1 WHERE feed_id=1;",
        )
        .unwrap();
        assert_eq!(
            collection_context(&db, 1).unwrap(),
            CollectionContext::RecoveryBackfill
        );
        db.execute_batch(
            "UPDATE collection_runtime SET recovery=0,gap_possible=1 WHERE feed_id=1;",
        )
        .unwrap();
        assert!(is_incomplete(&db, Some(1), None).unwrap());
        assert_eq!(
            collection_context(&db, 1).unwrap(),
            CollectionContext::Normal
        );
        db.execute_batch("UPDATE collection_runtime SET gap_possible=0 WHERE feed_id=1; UPDATE collection_health SET last_success=datetime('now','-2 days') WHERE feed_id=1;").unwrap();
        assert_eq!(
            collection_context(&db, 1).unwrap(),
            CollectionContext::RecoveryBackfill
        );
        db.execute_batch("UPDATE feeds SET paused=1 WHERE id=1;")
            .unwrap();
        assert!(is_incomplete(&db, Some(1), None).unwrap());
    }
    #[test]
    fn long_cursor_cycle_is_detected_after_persisted_refresh_boundaries() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("db");
        let db = super::super::open_db(&path, false).unwrap();
        db.execute("INSERT INTO feeds(id,url,title) VALUES(1,'https://example.invalid/social','Synthetic')",[]).unwrap();
        drop(db);
        let mut detected = false;
        for cursor in ["a", "b", "c"].into_iter().cycle().take(24) {
            let mut db = super::super::open_db(&path, false).unwrap();
            let mut runtime = load(&db, 1).unwrap();
            if !runtime.advance_cursor(cursor.into()) {
                detected = true;
                break;
            }
            let tx = db.transaction().unwrap();
            save(&tx, 1, &runtime).unwrap();
            tx.commit().unwrap();
        }
        assert!(
            detected,
            "three-step cycle must not run forever across restarts"
        );
    }
}
