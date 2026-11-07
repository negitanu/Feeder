use super::*;
use axum::{
    extract::Path,
    routing::{get, post},
};
use rusqlite::{OptionalExtension, params};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Saved {
    saved: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Settings {
    auto_remove: bool,
}
#[derive(Serialize)]
struct Item {
    id: i64,
    title: String,
    url: String,
    published: String,
    saved_at: String,
}
#[derive(Serialize)]
struct List {
    items: Vec<Item>,
}

pub fn migrate(tx: &rusqlite::Transaction<'_>) -> rusqlite::Result<()> {
    tx.execute_batch("CREATE TABLE IF NOT EXISTS article_read_later (article_id INTEGER PRIMARY KEY REFERENCES articles(id) ON DELETE CASCADE, saved_at TEXT NOT NULL); CREATE TABLE IF NOT EXISTS read_later_settings (id INTEGER PRIMARY KEY CHECK(id=1), auto_remove INTEGER NOT NULL DEFAULT 0 CHECK(auto_remove IN (0,1))); INSERT OR IGNORE INTO read_later_settings(id, auto_remove) VALUES(1,0);")
}

pub fn on_visit(tx: &Connection, id: i64) -> rusqlite::Result<()> {
    tx.execute("DELETE FROM article_read_later WHERE article_id=?1 AND (SELECT auto_remove FROM read_later_settings WHERE id=1)=1", [id])?;
    Ok(())
}

async fn list(State(db): State<Db>) -> Result<Json<List>, StatusCode> {
    let db = db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut stmt = db.prepare("SELECT a.id,a.title,a.url,a.published,r.saved_at FROM article_read_later r JOIN articles a ON a.id=r.article_id ORDER BY r.saved_at DESC,r.article_id DESC LIMIT 100").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let items = stmt
        .query_map([], |r| {
            Ok(Item {
                id: r.get(0)?,
                title: r.get(1)?,
                url: r.get(2)?,
                published: r.get(3)?,
                saved_at: r.get(4)?,
            })
        })
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(List { items }))
}

async fn set_saved(
    State(db): State<Db>,
    Path(id): Path<i64>,
    Json(input): Json<Saved>,
) -> Result<Json<Saved>, StatusCode> {
    let db = db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let exists: bool = db
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM articles WHERE id=?)",
            [id],
            |r| r.get(0),
        )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if !exists {
        return Err(StatusCode::NOT_FOUND);
    }
    if input.saved {
        db.execute("INSERT OR IGNORE INTO article_read_later(article_id,saved_at) VALUES(?,strftime('%Y-%m-%dT%H:%M:%fZ','now'))", [id])
    } else { db.execute("DELETE FROM article_read_later WHERE article_id=?", [id]) }.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(input))
}
async fn get_settings(State(db): State<Db>) -> Result<Json<Settings>, StatusCode> {
    let db = db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let auto_remove = db
        .query_row(
            "SELECT auto_remove FROM read_later_settings WHERE id=1",
            [],
            |r| r.get(0),
        )
        .optional()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .unwrap_or(false);
    Ok(Json(Settings { auto_remove }))
}
async fn patch_settings(
    State(db): State<Db>,
    Json(input): Json<Settings>,
) -> Result<Json<Settings>, StatusCode> {
    let db = db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    db.execute("INSERT INTO read_later_settings(id,auto_remove) VALUES(1,?) ON CONFLICT(id) DO UPDATE SET auto_remove=excluded.auto_remove", params![input.auto_remove]).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(input))
}
pub fn routes() -> Router<Db> {
    Router::new()
        .route("/api/read-later", get(list))
        .route("/api/articles/{id}/read-later", post(set_saved))
        .route(
            "/api/read-later/settings",
            get(get_settings).patch(patch_settings),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn queue_is_independent_and_visit_is_opt_in() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE articles(id INTEGER PRIMARY KEY,title TEXT NOT NULL,url TEXT NOT NULL,published TEXT NOT NULL,is_read INTEGER NOT NULL,is_starred INTEGER NOT NULL); INSERT INTO articles VALUES(1,'Synthetic','https://example.invalid','2026-01-01',0,1);").unwrap();
        let tx = conn.transaction().unwrap();
        migrate(&tx).unwrap();
        tx.commit().unwrap();
        let db = Arc::new(Mutex::new(conn));
        assert!(matches!(
            set_saved(State(db.clone()), Path(999), Json(Saved { saved: true })).await,
            Err(StatusCode::NOT_FOUND)
        ));
        let _ = set_saved(State(db.clone()), Path(1), Json(Saved { saved: true }))
            .await
            .unwrap();
        let _ = set_saved(State(db.clone()), Path(1), Json(Saved { saved: true }))
            .await
            .unwrap();
        {
            let c = db.lock().unwrap();
            c.execute("UPDATE articles SET is_read=1 WHERE id=1", [])
                .unwrap();
            on_visit(&c, 1).unwrap();
            assert_eq!(
                c.query_row("SELECT COUNT(*) FROM article_read_later", [], |r| r
                    .get::<_, i64>(0))
                    .unwrap(),
                1
            );
            assert_eq!(
                c.query_row("SELECT is_starred FROM articles WHERE id=1", [], |r| r
                    .get::<_, i64>(0))
                    .unwrap(),
                1
            );
        }
        let _ = patch_settings(State(db.clone()), Json(Settings { auto_remove: true }))
            .await
            .unwrap();
        {
            let c = db.lock().unwrap();
            on_visit(&c, 1).unwrap();
            assert_eq!(
                c.query_row("SELECT COUNT(*) FROM article_read_later", [], |r| r
                    .get::<_, i64>(0))
                    .unwrap(),
                0
            );
        }
        let _ = set_saved(State(db.clone()), Path(1), Json(Saved { saved: true }))
            .await
            .unwrap();
        {
            let c = db.lock().unwrap();
            c.execute("DELETE FROM articles WHERE id=1", []).unwrap();
            assert_eq!(
                c.query_row("SELECT COUNT(*) FROM article_read_later", [], |r| r
                    .get::<_, i64>(0))
                    .unwrap(),
                0
            );
        }
    }
}
