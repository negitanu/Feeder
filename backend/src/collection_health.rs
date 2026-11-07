use axum::{Json, Router, extract::State, http::StatusCode, routing::get};
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;

const TABLE: &str = "CREATE TABLE collection_health (feed_id INTEGER PRIMARY KEY REFERENCES feeds(id) ON DELETE CASCADE, last_success TEXT, consecutive_failures INTEGER NOT NULL DEFAULT 0)";

pub fn migrate(tx: &rusqlite::Transaction<'_>) -> rusqlite::Result<()> {
    tx.execute(TABLE, [])?;
    // Preserve known historical successes before the new attempt writer starts.
    tx.execute(
        "INSERT INTO collection_health(feed_id,last_success,consecutive_failures) SELECT id,last_checked,0 FROM feeds WHERE last_checked IS NOT NULL AND last_error IS NULL",
        [],
    )?;
    Ok(())
}

/// Record one accepted, lease-owned collection completion.
pub fn record_attempt(db: &Connection, feed_id: i64, success: bool) -> rusqlite::Result<()> {
    if success {
        db.execute(
            "INSERT INTO collection_health(feed_id,last_success,consecutive_failures) VALUES (?1,datetime('now'),0) ON CONFLICT(feed_id) DO UPDATE SET last_success=datetime('now'),consecutive_failures=0",
            [feed_id],
        )?;
    } else {
        db.execute(
            "INSERT INTO collection_health(feed_id,last_success,consecutive_failures) VALUES (?1,NULL,1) ON CONFLICT(feed_id) DO UPDATE SET consecutive_failures=collection_health.consecutive_failures+1",
            [feed_id],
        )?;
    }
    Ok(())
}

#[derive(Debug, Serialize)]
struct FeedHealth {
    id: i64,
    name: String,
    url: String,
    paused: bool,
    last_checked: Option<String>,
    last_success: Option<String>,
    error: Option<String>,
    next_due: Option<String>,
    consecutive_failures: i64,
    stale: bool,
    individual_override: bool,
    poll_interval_seconds: Option<i64>,
    collection_state: &'static str,
    next_retry: Option<String>,
    more_pending: bool,
    gap_possible: bool,
}

#[derive(Debug, Serialize)]
pub(super) struct HealthResponse {
    global_enabled: bool,
    global_interval_seconds: i64,
    feeds: Vec<FeedHealth>,
    truncated: bool,
}

fn read_health(db: &Connection) -> rusqlite::Result<HealthResponse> {
    let (global_enabled, global_interval_seconds): (bool, i64) = db.query_row(
        "SELECT enabled,interval_seconds FROM collection_settings WHERE id=1",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let mut stmt = db.prepare(
        r#"SELECT f.id,f.title,f.url,f.paused,f.last_checked,
           COALESCE(h.last_success,CASE WHEN h.feed_id IS NULL AND f.last_error IS NULL THEN f.last_checked END),
           f.last_error,COALESCE(h.consecutive_failures,0),
           CASE WHEN i.feed_id IS NULL THEN 0 ELSE 1 END,i.interval_seconds,
           CASE WHEN f.paused=1 OR ?1=0 THEN NULL
                WHEN f.last_checked IS NULL THEN datetime('now')
                ELSE datetime(f.last_checked, '+' || COALESCE(i.interval_seconds,?2) || ' seconds') END,
           CASE WHEN f.paused=1 OR ?1=0 THEN 0
                WHEN COALESCE(h.last_success,CASE WHEN h.feed_id IS NULL AND f.last_error IS NULL THEN f.last_checked END) IS NULL THEN 1
                WHEN unixepoch('now')-unixepoch(COALESCE(h.last_success,CASE WHEN h.feed_id IS NULL AND f.last_error IS NULL THEN f.last_checked END)) >= COALESCE(i.interval_seconds,?2) THEN 1
                ELSE 0 END
         FROM feeds f LEFT JOIN collection_health h ON h.feed_id=f.id LEFT JOIN feed_poll_intervals i ON i.feed_id=f.id
         ORDER BY f.id LIMIT 1001"#,
    )?;
    let mut rows = stmt.query(params![global_enabled, global_interval_seconds])?;
    let runtime_exists: bool = db.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='collection_runtime')", [], |r| r.get(0))?;
    let mut feeds = Vec::new();
    while let Some(row) = rows.next()? {
        let id: i64 = row.get(0)?;
        let paused: bool = row.get(3)?;
        let (next_retry, waiting, more_pending, gap_possible): (Option<String>, bool, bool, bool) =
            if runtime_exists {
                db.query_row("SELECT datetime(next_retry,'unixepoch'),COALESCE(next_retry>unixepoch(),0),more_pending,gap_possible FROM collection_runtime WHERE feed_id=?", [id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional()?.unwrap_or((None,false,false,false))
            } else {
                (None, false, false, false)
            };
        let error: Option<String> = row.get(6)?;
        let last_success: Option<String> = row.get(5)?;
        let mut next_due: Option<String> = row.get(10)?;
        if next_due.is_some() && more_pending {
            next_due = Some(db.query_row("SELECT datetime('now')", [], |r| r.get(0))?);
        }
        if waiting && next_due.is_some() && next_retry > next_due {
            next_due = next_retry.clone();
        }
        let state = if paused {
            "paused"
        } else if waiting {
            "waiting"
        } else if error.is_some() {
            "error"
        } else if more_pending {
            "catching_up"
        } else if last_success.is_none() {
            "uncollected"
        } else {
            "idle"
        };
        feeds.push(FeedHealth {
            id: row.get(0)?,
            name: row.get(1)?,
            url: row.get(2)?,
            paused: row.get(3)?,
            last_checked: row.get(4)?,
            last_success: row.get(5)?,
            error: row.get(6)?,
            consecutive_failures: row.get(7)?,
            individual_override: row.get(8)?,
            poll_interval_seconds: row.get(9)?,
            next_due,
            stale: row.get(11)?,
            collection_state: state,
            next_retry,
            more_pending,
            gap_possible,
        });
    }
    let truncated = feeds.len() > 1000;
    feeds.truncate(1000);
    Ok(HealthResponse {
        global_enabled,
        global_interval_seconds,
        feeds,
        truncated,
    })
}

pub(super) async fn handler(
    State(db): State<super::Db>,
) -> Result<Json<HealthResponse>, StatusCode> {
    let db = db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    read_health(&db)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub(super) fn routes() -> Router<super::Db> {
    Router::new().route("/api/collection/health", get(handler))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Connection {
        let db = Connection::open_in_memory().unwrap();
        db.pragma_update(None, "foreign_keys", true).unwrap();
        db.execute_batch(
            "CREATE TABLE feeds(id INTEGER PRIMARY KEY,title TEXT NOT NULL,url TEXT NOT NULL,paused INTEGER NOT NULL DEFAULT 0,last_checked TEXT,last_error TEXT);\
             CREATE TABLE collection_settings(id INTEGER PRIMARY KEY,enabled INTEGER NOT NULL,interval_seconds INTEGER NOT NULL);\
             CREATE TABLE feed_poll_intervals(feed_id INTEGER PRIMARY KEY REFERENCES feeds(id) ON DELETE CASCADE,interval_seconds INTEGER NOT NULL);\
             INSERT INTO collection_settings VALUES(1,1,900);\
             INSERT INTO feeds VALUES(1,'Recent','https://example.invalid/recent',0,datetime('now','-10 minutes'),NULL);\
             INSERT INTO feeds VALUES(2,'Legacy error','https://example.invalid/error',0,datetime('now'), 'old error');\
             INSERT INTO feeds VALUES(3,'Paused','https://example.invalid/paused',1,NULL,NULL);\
             INSERT INTO feeds VALUES(4,'Override','https://example.invalid/override',0,datetime('now','-2 minutes'),NULL);\
             INSERT INTO feed_poll_intervals VALUES(4,60);",
        ).unwrap();
        let tx = db.unchecked_transaction().unwrap();
        migrate(&tx).unwrap();
        tx.commit().unwrap();
        db
    }

    #[test]
    fn attempts_reset_or_increment_and_preserve_known_success() {
        let db = fixture();
        record_attempt(&db, 1, false).unwrap();
        record_attempt(&db, 1, false).unwrap();
        let row: (Option<String>, i64) = db
            .query_row(
                "SELECT last_success,consecutive_failures FROM collection_health WHERE feed_id=1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert!(row.0.is_some());
        assert_eq!(row.1, 2);
        record_attempt(&db, 1, true).unwrap();
        let row: (Option<String>, i64) = db
            .query_row(
                "SELECT last_success,consecutive_failures FROM collection_health WHERE feed_id=1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert!(row.0.is_some());
        assert_eq!(row.1, 0);
    }

    #[test]
    fn first_failure_without_success_keeps_success_unknown() {
        let db = fixture();
        record_attempt(&db, 2, false).unwrap();
        let row: (Option<String>, i64) = db
            .query_row(
                "SELECT last_success,consecutive_failures FROM collection_health WHERE feed_id=2",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(row, (None, 1));
    }

    #[test]
    fn read_contract_uses_legacy_fallback_and_respects_disabled_due_state() {
        let db = fixture();
        let health = read_health(&db).unwrap();
        assert_eq!(health.feeds[0].last_success, health.feeds[0].last_checked);
        assert_eq!(health.feeds[1].last_success, None);
        assert_eq!(health.feeds[1].consecutive_failures, 0);
        assert_eq!(health.feeds[2].next_due, None);
        assert!(health.feeds[3].individual_override);
        assert_eq!(health.feeds[3].poll_interval_seconds, Some(60));
        db.execute("UPDATE collection_settings SET enabled=0", [])
            .unwrap();
        assert!(
            read_health(&db)
                .unwrap()
                .feeds
                .iter()
                .all(|feed| feed.next_due.is_none())
        );
    }

    #[test]
    fn delete_cascades_and_results_are_explicitly_bounded() {
        let db = fixture();
        for id in 5..=1002 {
            db.execute(
                "INSERT INTO feeds(id,title,url) VALUES(?1,?2,?3)",
                params![
                    id,
                    format!("Feed {id}"),
                    format!("https://example.invalid/{id}")
                ],
            )
            .unwrap();
        }
        assert_eq!(read_health(&db).unwrap().feeds.len(), 1000);
        assert!(read_health(&db).unwrap().truncated);
        record_attempt(&db, 1, true).unwrap();
        db.execute("DELETE FROM feeds WHERE id=1", []).unwrap();
        let count: i64 = db
            .query_row(
                "SELECT count(*) FROM collection_health WHERE feed_id=1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }
}
