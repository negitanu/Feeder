use super::*;
use crate::collection_runtime::CollectionContext;
use axum::extract::RawQuery;
use sha2::{Digest, Sha256};

fn err<T>(_: T) -> StatusCode {
    StatusCode::INTERNAL_SERVER_ERROR
}
const KEEP: i64 = 20;
pub fn migrate_catchup(tx: &rusqlite::Transaction<'_>) -> rusqlite::Result<()> {
    tx.execute_batch("CREATE TABLE article_revision_state (article_id INTEGER PRIMARY KEY REFERENCES articles(id) ON DELETE CASCADE, current_revision INTEGER NOT NULL, fingerprint TEXT NOT NULL, first_seen_at TEXT NOT NULL, ingestion_kind TEXT NOT NULL CHECK(ingestion_kind IN ('initial','normal','recovery','legacy')), changed_at TEXT, source_updated_at TEXT, history_complete INTEGER NOT NULL DEFAULT 0 CHECK(history_complete IN (0,1))); CREATE TABLE article_revisions (article_id INTEGER NOT NULL REFERENCES articles(id) ON DELETE CASCADE, revision INTEGER NOT NULL, fingerprint TEXT NOT NULL, title TEXT NOT NULL, content TEXT NOT NULL, url TEXT NOT NULL, published TEXT NOT NULL, source_updated_at TEXT, observed_at TEXT NOT NULL, baseline INTEGER NOT NULL CHECK(baseline IN (0,1)), PRIMARY KEY(article_id,revision));")
}
#[derive(Debug, Clone)]
pub struct RevisionState {
    pub current_revision: i64,
    pub fingerprint: String,
    pub history_complete: bool,
}
pub fn current_state(db: &Connection, id: i64) -> rusqlite::Result<Option<RevisionState>> {
    db.query_row("SELECT current_revision,fingerprint,history_complete FROM article_revision_state WHERE article_id=?", [id], |r| Ok(RevisionState{current_revision:r.get(0)?,fingerprint:r.get(1)?,history_complete:r.get(2)?})).optional()
}
fn fingerprint(title: &str, content: &str, url: &str) -> String {
    let mut h = Sha256::new();
    for s in [title, content, url] {
        h.update((s.len() as u64).to_be_bytes());
        h.update(s.as_bytes());
    }
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}
#[allow(clippy::too_many_arguments)]
pub fn store_entry(
    tx: &rusqlite::Transaction<'_>,
    feed_id: i64,
    external_id: &str,
    title: &str,
    content: &str,
    url: &str,
    published: &str,
    source_updated_at: Option<&str>,
    kind: CollectionContext,
) -> rusqlite::Result<i64> {
    let old: Option<(i64, String, String, String, String)> = tx
        .query_row(
            "SELECT id,title,content,url,published FROM articles WHERE feed_id=? AND external_id=?",
            params![feed_id, external_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .optional()?;
    let now: String = tx.query_row("SELECT strftime('%Y-%m-%dT%H:%M:%fZ','now')", [], |r| {
        r.get(0)
    })?;
    let source: Option<String> = tx.query_row(
        "SELECT strftime('%Y-%m-%dT%H:%M:%fZ',?1)",
        [source_updated_at],
        |r| r.get(0),
    )?;
    let fp = fingerprint(title, content, url);
    tx.execute("INSERT INTO articles(feed_id,external_id,title,content,url,published) VALUES (?,?,?,?,?,?) ON CONFLICT(feed_id,external_id) DO UPDATE SET title=excluded.title,content=excluded.content,url=excluded.url,published=excluded.published",params![feed_id,external_id,title,content,url,published])?;
    let id: i64 = tx.query_row(
        "SELECT id FROM articles WHERE feed_id=? AND external_id=?",
        params![feed_id, external_id],
        |r| r.get(0),
    )?;
    let mut state = current_state(tx, id)?;
    if state.is_none() {
        let (base_title, base_content, base_url, base_published, initial_kind) =
            if let Some((_, ref t, ref c, ref u, ref p)) = old {
                (t.as_str(), c.as_str(), u.as_str(), p.as_str(), "legacy")
            } else {
                (
                    title,
                    content,
                    url,
                    published,
                    match kind {
                        CollectionContext::InitialBackfill => "initial",
                        CollectionContext::Normal => "normal",
                        CollectionContext::RecoveryBackfill => "recovery",
                    },
                )
            };
        let base_fp = fingerprint(base_title, base_content, base_url);
        tx.execute(
            "INSERT INTO article_revision_state VALUES (?,1,?,?,?,NULL,?,0)",
            params![id, base_fp, now, initial_kind, source],
        )?;
        tx.execute(
            "INSERT INTO article_revisions VALUES (?,1,?,?,?,?,?,?,?,1)",
            params![
                id,
                base_fp,
                base_title,
                base_content,
                base_url,
                base_published,
                if old.is_some() {
                    None
                } else {
                    source.as_deref()
                },
                now
            ],
        )?;
        state = current_state(tx, id)?;
    }
    let s = state.unwrap();
    if s.fingerprint != fp {
        let revision = s.current_revision + 1;
        tx.execute(
            "INSERT INTO article_revisions VALUES (?,?,?,?,?,?,?,?,?,0)",
            params![
                id, revision, fp, title, content, url, published, source, now
            ],
        )?;
        tx.execute("UPDATE article_revision_state SET current_revision=?,fingerprint=?,changed_at=?,source_updated_at=? WHERE article_id=?",params![revision,fp,now,source,id])?;
        tx.execute(
            "DELETE FROM article_revisions WHERE article_id=? AND revision<=?",
            params![id, revision - KEEP],
        )?;
    } else {
        tx.execute(
            "UPDATE article_revision_state SET source_updated_at=? WHERE article_id=?",
            params![source, id],
        )?;
    }
    crate::story_groups::index_article(tx, id, title, content, url, published)?;
    Ok(id)
}
#[derive(Serialize)]
struct Snapshot {
    revision: i64,
    fingerprint: String,
    title: String,
    content: String,
    url: String,
    published: String,
    source_updated_at: Option<String>,
    observed_at: String,
    baseline: bool,
}
#[derive(Serialize)]
struct History {
    items: Vec<Snapshot>,
    has_more: bool,
    next_before_revision: Option<i64>,
    history_complete: bool,
    history_start: Option<i64>,
}
async fn history(
    State(db): State<Db>,
    Path(id): Path<i64>,
    RawQuery(raw): RawQuery,
) -> Result<Json<History>, StatusCode> {
    let mut limit = 10usize;
    let mut before = None;
    let mut seen = std::collections::HashSet::new();
    if let Some(raw) = raw {
        if raw.len() > 256 {
            return Err(StatusCode::BAD_REQUEST);
        }
        for (k, v) in url::form_urlencoded::parse(raw.as_bytes()) {
            if !seen.insert(k.to_string()) {
                return Err(StatusCode::BAD_REQUEST);
            }
            match k.as_ref() {
                "limit" => {
                    limit = v.parse().map_err(|_| StatusCode::BAD_REQUEST)?;
                    if !(1..=20).contains(&limit) {
                        return Err(StatusCode::BAD_REQUEST);
                    }
                }
                "before_revision" => {
                    let n: i64 = v.parse().map_err(|_| StatusCode::BAD_REQUEST)?;
                    if n < 1 {
                        return Err(StatusCode::BAD_REQUEST);
                    }
                    before = Some(n)
                }
                _ => return Err(StatusCode::BAD_REQUEST),
            }
        }
    }
    let db = db.lock().map_err(err)?;
    let exists: bool = db
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM articles WHERE id=?)",
            [id],
            |r| r.get(0),
        )
        .map_err(err)?;
    if !exists {
        return Err(StatusCode::NOT_FOUND);
    }
    let state = current_state(&db, id).map_err(err)?;
    let start: Option<i64> = db
        .query_row(
            "SELECT min(revision) FROM article_revisions WHERE article_id=?",
            [id],
            |r| r.get(0),
        )
        .map_err(err)?;
    let mut stmt=db.prepare("SELECT revision,fingerprint,title,content,url,published,source_updated_at,observed_at,baseline FROM article_revisions WHERE article_id=? AND (? IS NULL OR revision<?) ORDER BY revision DESC LIMIT ?").map_err(err)?;
    let mut items: Vec<Snapshot> = stmt
        .query_map(params![id, before, before, (limit + 1) as i64], |r| {
            Ok(Snapshot {
                revision: r.get(0)?,
                fingerprint: r.get(1)?,
                title: r.get(2)?,
                content: r.get(3)?,
                url: r.get(4)?,
                published: r.get(5)?,
                source_updated_at: r.get(6)?,
                observed_at: r.get(7)?,
                baseline: r.get(8)?,
            })
        })
        .map_err(err)?
        .collect::<Result<_, _>>()
        .map_err(err)?;
    let has_more = items.len() > limit;
    if has_more {
        items.pop();
    }
    let next = if has_more {
        items.last().map(|x| x.revision)
    } else {
        None
    };
    Ok(Json(History {
        items,
        has_more,
        next_before_revision: next,
        history_complete: state.is_some_and(|x| x.history_complete),
        history_start: start,
    }))
}
pub fn routes() -> Router<Db> {
    Router::new().route("/api/articles/{id}/revisions", get(history))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Connection {
        let mut db = Connection::open_in_memory().unwrap();
        db.execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE feeds(id INTEGER PRIMARY KEY); INSERT INTO feeds VALUES(1); CREATE TABLE articles(id INTEGER PRIMARY KEY,feed_id INTEGER REFERENCES feeds(id),external_id TEXT,title TEXT NOT NULL,content TEXT NOT NULL,url TEXT NOT NULL,published TEXT NOT NULL,is_read INTEGER NOT NULL DEFAULT 0,is_starred INTEGER NOT NULL DEFAULT 0,UNIQUE(feed_id,external_id)); CREATE TABLE mute_rules(enabled INTEGER,feed_id INTEGER,field TEXT,term TEXT);").unwrap();
        let tx = db.transaction().unwrap();
        migrate_catchup(&tx).unwrap();
        crate::story_groups::migrate_catchup(&tx).unwrap();
        tx.commit().unwrap();
        db
    }
    #[test]
    fn legacy_baseline_content_changes_and_retention_preserve_state() {
        let mut db = fixture();
        db.execute("INSERT INTO articles VALUES(4,1,'x','old','body','https://example.invalid/a','yesterday',1,1)",[]).unwrap();
        let tx = db.transaction().unwrap();
        let id = store_entry(
            &tx,
            1,
            "x",
            "new",
            "body",
            "https://example.invalid/a",
            "today",
            Some("2026-11-07T09:00:00+09:00"),
            CollectionContext::Normal,
        )
        .unwrap();
        let state = current_state(&tx, id).unwrap().unwrap();
        assert_eq!(state.current_revision, 2);
        let (kind, source): (String, Option<String>) = tx.query_row("SELECT ingestion_kind, source_updated_at FROM article_revision_state WHERE article_id=?", [id], |r| Ok((r.get(0)?,r.get(1)?))).unwrap();
        assert_eq!(kind, "legacy");
        assert_eq!(source.as_deref(), Some("2026-11-07T00:00:00.000Z"));
        assert!(!state.history_complete);
        let baseline_source: Option<String> = tx
            .query_row(
                "SELECT source_updated_at FROM article_revisions WHERE article_id=4 AND revision=1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(baseline_source, None);
        assert_eq!(
            tx.query_row(
                "SELECT title FROM article_revisions WHERE article_id=4 AND revision=1",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            "old"
        );
        store_entry(
            &tx,
            1,
            "x",
            "new",
            "body",
            "https://example.invalid/a",
            "another date",
            None,
            CollectionContext::Normal,
        )
        .unwrap();
        assert_eq!(current_state(&tx, id).unwrap().unwrap().current_revision, 2);
        for i in 0..25 {
            store_entry(
                &tx,
                1,
                "x",
                &format!("title {i}"),
                "body",
                "https://example.invalid/a",
                "today",
                None,
                CollectionContext::Normal,
            )
            .unwrap();
        }
        assert_eq!(
            tx.query_row(
                "SELECT count(*) FROM article_revisions WHERE article_id=4",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            20
        );
        assert!(
            tx.query_row(
                "SELECT (is_read,is_starred)=(1,1) FROM articles WHERE id=4",
                [],
                |r| r.get::<_, bool>(0)
            )
            .unwrap()
        );
        tx.commit().unwrap();
    }
    #[test]
    fn initial_kind_is_stable_and_rollback_is_atomic() {
        let mut db = fixture();
        {
            let tx = db.transaction().unwrap();
            let id = store_entry(
                &tx,
                1,
                "a",
                "title",
                "body",
                "https://e.invalid/",
                "",
                None,
                CollectionContext::InitialBackfill,
            )
            .unwrap();
            store_entry(
                &tx,
                1,
                "a",
                "title",
                "body",
                "https://e.invalid/",
                "",
                None,
                CollectionContext::Normal,
            )
            .unwrap();
            let state = current_state(&tx, id).unwrap().unwrap();
            assert_eq!(state.current_revision, 1);
            let (kind, changed): (String, Option<String>) = tx.query_row("SELECT ingestion_kind,changed_at FROM article_revision_state WHERE article_id=?", [id], |r| Ok((r.get(0)?,r.get(1)?))).unwrap();
            assert_eq!(kind, "initial");
            assert!(changed.is_none());
        }
        assert_eq!(
            db.query_row("SELECT count(*) FROM articles", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
    #[tokio::test]
    async fn history_paginates_and_get_has_no_side_effects() {
        let mut db = fixture();
        let tx = db.transaction().unwrap();
        for i in 0..4 {
            store_entry(
                &tx,
                1,
                "a",
                &format!("title{i}"),
                "body",
                "https://e.invalid/a",
                "2026-11-07",
                None,
                CollectionContext::Normal,
            )
            .unwrap();
        }
        tx.commit().unwrap();
        let db = std::sync::Arc::new(std::sync::Mutex::new(db));
        let Json(first) = history(State(db.clone()), Path(1), RawQuery(Some("limit=2".into())))
            .await
            .unwrap();
        assert_eq!(
            first.items.iter().map(|x| x.revision).collect::<Vec<_>>(),
            vec![4, 3]
        );
        assert!(first.has_more);
        assert_eq!(first.next_before_revision, Some(3));
        assert_eq!(first.history_start, Some(1));
        assert!(!first.history_complete);
        let Json(second) = history(
            State(db.clone()),
            Path(1),
            RawQuery(Some("limit=2&before_revision=3".into())),
        )
        .await
        .unwrap();
        assert_eq!(
            second.items.iter().map(|x| x.revision).collect::<Vec<_>>(),
            vec![2, 1]
        );
        assert!(!second.has_more);
        assert!(matches!(
            history(
                State(db.clone()),
                Path(1),
                RawQuery(Some("limit=21".into()))
            )
            .await,
            Err(StatusCode::BAD_REQUEST)
        ));
        assert!(matches!(
            history(State(db.clone()), Path(999), RawQuery(None)).await,
            Err(StatusCode::NOT_FOUND)
        ));
        assert_eq!(
            db.lock()
                .unwrap()
                .query_row(
                    "SELECT is_read+is_starred FROM articles WHERE id=1",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
    }
}
