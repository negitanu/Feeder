use super::*;
use axum::routing::{get, patch};

pub fn migrate(tx: &rusqlite::Transaction<'_>) -> rusqlite::Result<()> {
    tx.execute_batch("CREATE TABLE mute_rules (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, term TEXT NOT NULL, field TEXT NOT NULL CHECK(field IN ('title','body','url','both')), feed_id INTEGER REFERENCES feeds(id) ON DELETE CASCADE, enabled INTEGER NOT NULL CHECK(enabled IN (0,1)), revision INTEGER NOT NULL CHECK(revision > 0));")
}

/// SQL boolean expression; the outer articles table must be named `articles`.
pub const HIDDEN_SQL: &str = "EXISTS (SELECT 1 FROM mute_rules AS rule WHERE rule.enabled = 1 AND (rule.feed_id IS NULL OR rule.feed_id = articles.feed_id) AND ((rule.field IN ('title','both') AND instr(lower(articles.title), lower(rule.term)) > 0) OR (rule.field IN ('body','both') AND instr(lower(articles.content), lower(rule.term)) > 0) OR (rule.field = 'url' AND instr(lower(articles.url), lower(rule.term)) > 0)))";

#[derive(Clone, Serialize)]
struct Rule {
    id: i64,
    name: String,
    term: String,
    field: String,
    feed_id: Option<i64>,
    enabled: bool,
    revision: i64,
}
fn row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Rule> {
    Ok(Rule {
        id: r.get(0)?,
        name: r.get(1)?,
        term: r.get(2)?,
        field: r.get(3)?,
        feed_id: r.get(4)?,
        enabled: r.get(5)?,
        revision: r.get(6)?,
    })
}
const SELECT: &str = "SELECT id,name,term,field,feed_id,enabled,revision FROM mute_rules";
fn valid(name: &str, term: &str, field: &str, feed: Option<i64>) -> bool {
    !name.trim().is_empty()
        && name.encode_utf16().count() <= 80
        && !term.trim().is_empty()
        && term.encode_utf16().count() <= 200
        && !term.contains('\0')
        && !name.contains('\0')
        && matches!(field, "title" | "body" | "url" | "both")
        && feed.is_none_or(|id| id > 0 && id <= 9_007_199_254_740_991)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Create {
    name: String,
    term: String,
    field: String,
    feed_id: Option<i64>,
    enabled: bool,
}
fn nullable_feed<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Option<i64>>, D::Error> {
    Option::<i64>::deserialize(d).map(Some)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Update {
    revision: i64,
    name: Option<String>,
    term: Option<String>,
    field: Option<String>,
    #[serde(default, deserialize_with = "nullable_feed")]
    feed_id: Option<Option<i64>>,
    enabled: Option<bool>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Revision {
    revision: i64,
}
#[derive(Serialize)]
struct Rules {
    rules: Vec<Rule>,
}
#[derive(Serialize)]
struct Reason {
    id: i64,
    name: String,
}
#[derive(Serialize)]
struct Reasons {
    reasons: Vec<Reason>,
}
fn err(e: rusqlite::Error) -> StatusCode {
    if matches!(&e, rusqlite::Error::SqliteFailure(x, _) if x.code == rusqlite::ErrorCode::ConstraintViolation)
    {
        StatusCode::CONFLICT
    } else {
        collection::err(e)
    }
}
async fn list(State(db): State<Db>) -> Result<Json<Rules>, StatusCode> {
    let db = db.lock().map_err(collection::err)?;
    let mut stmt = db.prepare(&format!("{SELECT} ORDER BY id")).map_err(err)?;
    let rules = stmt
        .query_map([], row)
        .map_err(err)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(err)?;
    Ok(Json(Rules { rules }))
}
async fn create(
    State(db): State<Db>,
    headers: HeaderMap,
    Json(input): Json<Create>,
) -> Result<(StatusCode, Json<Rule>), StatusCode> {
    if !origin_allowed(&headers) {
        return Err(StatusCode::FORBIDDEN);
    }
    if !valid(&input.name, &input.term, &input.field, input.feed_id) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut db = db.lock().map_err(collection::err)?;
    let tx = db.transaction().map_err(err)?;
    if tx
        .query_row("SELECT count(*) FROM mute_rules", [], |r| {
            r.get::<_, i64>(0)
        })
        .map_err(err)?
        >= 100
    {
        return Err(StatusCode::CONFLICT);
    }
    if let Some(id) = input.feed_id
        && !tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM feeds WHERE id=?1)",
                [id],
                |r| r.get::<_, bool>(0),
            )
            .map_err(err)?
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    tx.execute("INSERT INTO mute_rules(name,term,field,feed_id,enabled,revision) VALUES (?1,?2,?3,?4,?5,1)",params![input.name,input.term,input.field,input.feed_id,input.enabled]).map_err(err)?;
    let rule = tx
        .query_row(
            &format!("{SELECT} WHERE id=?1"),
            [tx.last_insert_rowid()],
            row,
        )
        .map_err(err)?;
    tx.commit().map_err(err)?;
    Ok((StatusCode::CREATED, Json(rule)))
}
async fn update(
    State(db): State<Db>,
    Path(id): Path<i64>,
    headers: HeaderMap,
    Json(input): Json<Update>,
) -> Result<Json<Rule>, StatusCode> {
    if !origin_allowed(&headers) {
        return Err(StatusCode::FORBIDDEN);
    }
    if id < 1
        || input.revision < 1
        || (input.name.is_none()
            && input.term.is_none()
            && input.field.is_none()
            && input.feed_id.is_none()
            && input.enabled.is_none())
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut db = db.lock().map_err(collection::err)?;
    let tx = db.transaction().map_err(err)?;
    let old = tx
        .query_row(&format!("{SELECT} WHERE id=?1"), [id], row)
        .optional()
        .map_err(err)?
        .ok_or(StatusCode::NOT_FOUND)?;
    if old.revision != input.revision || old.revision == i64::MAX {
        return Err(StatusCode::CONFLICT);
    }
    let name = input.name.unwrap_or(old.name);
    let term = input.term.unwrap_or(old.term);
    let field = input.field.unwrap_or(old.field);
    let feed = input.feed_id.unwrap_or(old.feed_id);
    let enabled = input.enabled.unwrap_or(old.enabled);
    if !valid(&name, &term, &field, feed) {
        return Err(StatusCode::BAD_REQUEST);
    }
    if let Some(id) = feed
        && !tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM feeds WHERE id=?1)",
                [id],
                |r| r.get::<_, bool>(0),
            )
            .map_err(err)?
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    tx.execute("UPDATE mute_rules SET name=?1,term=?2,field=?3,feed_id=?4,enabled=?5,revision=revision+1 WHERE id=?6",params![name,term,field,feed,enabled,id]).map_err(err)?;
    let rule = tx
        .query_row(&format!("{SELECT} WHERE id=?1"), [id], row)
        .map_err(err)?;
    tx.commit().map_err(err)?;
    Ok(Json(rule))
}
async fn remove(
    State(db): State<Db>,
    Path(id): Path<i64>,
    headers: HeaderMap,
    Json(input): Json<Revision>,
) -> Result<StatusCode, StatusCode> {
    if !origin_allowed(&headers) {
        return Err(StatusCode::FORBIDDEN);
    }
    if id < 1 || input.revision < 1 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let db = db.lock().map_err(collection::err)?;
    if db
        .execute(
            "DELETE FROM mute_rules WHERE id=?1 AND revision=?2",
            params![id, input.revision],
        )
        .map_err(err)?
        == 0
    {
        return Err(
            if db
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM mute_rules WHERE id=?1)",
                    [id],
                    |r| r.get::<_, bool>(0),
                )
                .map_err(err)?
            {
                StatusCode::CONFLICT
            } else {
                StatusCode::NOT_FOUND
            },
        );
    }
    Ok(StatusCode::NO_CONTENT)
}
async fn reasons(State(db): State<Db>, Path(id): Path<i64>) -> Result<Json<Reasons>, StatusCode> {
    if id < 1 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let db = db.lock().map_err(collection::err)?;
    if !db
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM articles WHERE id=?1)",
            [id],
            |r| r.get::<_, bool>(0),
        )
        .map_err(err)?
    {
        return Err(StatusCode::NOT_FOUND);
    }
    let predicate = HIDDEN_SQL.replace(
        "WHERE rule.enabled",
        "WHERE rule.id = target.id AND rule.enabled",
    );
    let sql = format!(
        "SELECT target.id,target.name FROM mute_rules AS target JOIN articles ON articles.id=?1 WHERE {predicate} ORDER BY target.id"
    );
    let mut stmt = db.prepare(&sql).map_err(err)?;
    let reasons = stmt
        .query_map([id], |r| {
            Ok(Reason {
                id: r.get(0)?,
                name: r.get(1)?,
            })
        })
        .map_err(err)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(err)?;
    Ok(Json(Reasons { reasons }))
}
pub fn routes() -> Router<Db> {
    Router::new()
        .route("/api/mute-rules", get(list).post(create))
        .route("/api/mute-rules/{id}", patch(update).delete(remove))
        .route("/api/articles/{id}/mute-reasons", get(reasons))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn matching_is_literal_scoped_and_non_destructive() {
        let mut db = Connection::open_in_memory().unwrap();
        db.execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE feeds(id INTEGER PRIMARY KEY); INSERT INTO feeds VALUES(1),(2); CREATE TABLE articles(id INTEGER PRIMARY KEY,title TEXT,content TEXT,url TEXT,feed_id INTEGER,is_read INTEGER,is_starred INTEGER); INSERT INTO articles VALUES(1,'100% NEWS','hello [x]','https://a.test/',1,0,1),(2,'100x news','hello x','https://b.test/',2,1,0);").unwrap();
        let tx = db.transaction().unwrap();
        migrate(&tx).unwrap();
        tx.commit().unwrap();
        db.execute_batch("INSERT INTO mute_rules(name,term,field,feed_id,enabled,revision) VALUES('literal','100%','title',1,1,1),('paused','hello','body',NULL,0,1),('wrong feed','news','title',2,0,1);").unwrap();
        let query = format!("SELECT id FROM articles WHERE {HIDDEN_SQL} ORDER BY id");
        let ids = |db: &Connection| {
            db.prepare(&query)
                .unwrap()
                .query_map([], |r| r.get::<_, i64>(0))
                .unwrap()
                .collect::<rusqlite::Result<Vec<_>>>()
                .unwrap()
        };
        assert_eq!(ids(&db), vec![1]);
        db.execute("UPDATE mute_rules SET enabled=0 WHERE name='literal'", [])
            .unwrap();
        assert!(ids(&db).is_empty());
        db.execute(
            "UPDATE mute_rules SET enabled=1 WHERE name='wrong feed'",
            [],
        )
        .unwrap();
        assert_eq!(ids(&db), vec![2]);
        db.execute("DELETE FROM mute_rules", []).unwrap();
        let states: Vec<(i64, bool, bool)> = db
            .prepare("SELECT id,is_read,is_starred FROM articles ORDER BY id")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(states, vec![(1, false, true), (2, true, false)]);
        assert!(!valid("x", &"x".repeat(201), "title", None));
    }
    #[tokio::test]
    async fn reasons_and_revision_conflicts() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE feeds(id INTEGER PRIMARY KEY); INSERT INTO feeds VALUES(1); CREATE TABLE articles(id INTEGER PRIMARY KEY,title TEXT,content TEXT,url TEXT,feed_id INTEGER); INSERT INTO articles VALUES(1,'HELLO','body','url',1);").unwrap();
        let tx = conn.transaction().unwrap();
        migrate(&tx).unwrap();
        tx.commit().unwrap();
        let db = Arc::new(Mutex::new(conn));
        let (_, Json(rule)) = create(
            State(db.clone()),
            HeaderMap::new(),
            Json(Create {
                name: "test".into(),
                term: "hello".into(),
                field: "title".into(),
                feed_id: Some(1),
                enabled: true,
            }),
        )
        .await
        .ok()
        .unwrap();
        let result = reasons(State(db.clone()), Path(1)).await.ok().unwrap().0;
        assert_eq!(result.reasons.len(), 1);
        assert_eq!(result.reasons[0].id, rule.id);
        let changed = update(
            State(db.clone()),
            Path(rule.id),
            HeaderMap::new(),
            Json(Update {
                revision: rule.revision,
                name: None,
                term: None,
                field: None,
                feed_id: None,
                enabled: Some(false),
            }),
        )
        .await
        .ok()
        .unwrap()
        .0;
        assert_eq!(changed.revision, 2);
        assert!(
            reasons(State(db.clone()), Path(1))
                .await
                .ok()
                .unwrap()
                .0
                .reasons
                .is_empty()
        );
        assert_eq!(
            remove(
                State(db),
                Path(rule.id),
                HeaderMap::new(),
                Json(Revision {
                    revision: rule.revision
                })
            )
            .await
            .err(),
            Some(StatusCode::CONFLICT)
        );
    }
}
