use super::*;
use axum::extract::RawQuery;
use rusqlite::types::Value;
use std::collections::HashSet;

pub const VISITS: &str = "CREATE TABLE article_visits (article_id INTEGER PRIMARY KEY REFERENCES articles(id) ON DELETE CASCADE, visited_at TEXT NOT NULL)";
pub fn recognized_v4(objects: &[(String, String, Option<String>)]) -> bool {
    let Ok(mut db) = Connection::open_in_memory() else {
        return false;
    };
    if db.execute(SCHEMA, []).is_err() {
        return false;
    }
    let Ok(tx) = db.transaction() else {
        return false;
    };
    if collection::migrate(&tx).is_err() || tx.execute(VISITS, []).is_err() || tx.commit().is_err()
    {
        return false;
    }
    let Ok(mut stmt) = db.prepare(
        "SELECT type, name, sql FROM sqlite_master WHERE name NOT LIKE 'sqlite_%' ORDER BY name",
    ) else {
        return false;
    };
    let Ok(rows) = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))) else {
        return false;
    };
    rows.collect::<Result<Vec<_>, _>>()
        .is_ok_and(|v| v == objects)
}
#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Filters {
    unread: Option<bool>,
    starred: Option<bool>,
    q: Option<String>,
    feed_id: Option<i64>,
    folder_id: Option<i64>,
    history: Option<bool>,
    order: Option<String>,
    scope: Option<String>,
    any: Option<String>,
    exclude: Option<String>,
    published_from: Option<String>,
    published_to: Option<String>,
    feed_ids: Option<String>,
}
#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ListQuery {
    limit: Option<u32>,
    offset: Option<u32>,
    #[serde(skip)]
    filters: Filters,
}
fn parse_query(raw: &str) -> Result<ListQuery, StatusCode> {
    if raw.len() > 8192 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut q = ListQuery::default();
    let mut values = serde_json::Map::new();
    let mut seen = HashSet::new();
    for (key, value) in url::form_urlencoded::parse(raw.as_bytes()) {
        if !seen.insert(key.to_string()) || seen.len() > 32 {
            return Err(StatusCode::BAD_REQUEST);
        }
        match key.as_ref() {
            "limit" => q.limit = Some(value.parse().map_err(|_| StatusCode::BAD_REQUEST)?),
            "offset" => q.offset = Some(value.parse().map_err(|_| StatusCode::BAD_REQUEST)?),
            "unread" | "starred" | "history" => {
                let v = match value.as_ref() {
                    "true" => true,
                    "false" => false,
                    _ => return Err(StatusCode::BAD_REQUEST),
                };
                values.insert(key.into_owned(), serde_json::Value::Bool(v));
            }
            "feed_id" | "folder_id" => {
                let id: i64 = value.parse().map_err(|_| StatusCode::BAD_REQUEST)?;
                values.insert(key.into_owned(), id.into());
            }
            _ => {
                values.insert(key.into_owned(), value.into_owned().into());
            }
        }
    }
    q.filters = serde_json::from_value(serde_json::Value::Object(values))
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    Ok(q)
}
#[derive(Serialize)]
pub struct Listing {
    items: Vec<Article>,
    total: i64,
}
#[derive(Serialize)]
pub struct Updated {
    updated: usize,
}
fn csv(s: &str, max: usize) -> Result<Vec<String>, StatusCode> {
    if s.len() > 4096 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut it = s.chars().peekable();
    let mut out = Vec::new();
    loop {
        let mut v = String::new();
        if it.peek() == Some(&'"') {
            it.next();
            let mut closed = false;
            while let Some(c) = it.next() {
                if c == '"' {
                    if it.peek() == Some(&'"') {
                        it.next();
                        v.push('"')
                    } else {
                        closed = true;
                        break;
                    }
                } else {
                    v.push(c)
                }
            }
            if !closed || !matches!(it.peek(), None | Some(',')) {
                return Err(StatusCode::BAD_REQUEST);
            }
        } else {
            while let Some(&c) = it.peek() {
                if c == ',' {
                    break;
                }
                if c == '"' {
                    return Err(StatusCode::BAD_REQUEST);
                }
                v.push(c);
                it.next();
            }
        }
        let v = v.trim();
        if v.is_empty() || v.len() > 256 {
            return Err(StatusCode::BAD_REQUEST);
        }
        out.push(v.to_owned());
        if out.len() > max {
            return Err(StatusCode::BAD_REQUEST);
        }
        if it.next().is_none() {
            break;
        }
    }
    Ok(out)
}
fn date(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 10
        || b[4] != b'-'
        || b[7] != b'-'
        || !b
            .iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
    {
        return false;
    }
    let y: u32 = s[0..4].parse().unwrap_or(0);
    let m: u32 = s[5..7].parse().unwrap_or(0);
    let d: u32 = s[8..10].parse().unwrap_or(0);
    let n = match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if y.is_multiple_of(4) && (!y.is_multiple_of(100) || y.is_multiple_of(400)) => 29,
        2 => 28,
        _ => 0,
    };
    y > 0 && d > 0 && d <= n
}
struct Scope {
    sql: String,
    values: Vec<Value>,
    order: &'static str,
}
fn scope(db: &Connection, f: &Filters) -> Result<Scope, StatusCode> {
    let mut c = Vec::<String>::new();
    let mut v = Vec::<Value>::new();
    if f.unread.unwrap_or(false) {
        c.push("is_read=0".into())
    }
    if f.starred.unwrap_or(false) {
        c.push("is_starred=1".into())
    }
    if f.history.unwrap_or(false) {
        c.push("EXISTS (SELECT 1 FROM article_visits WHERE article_id=articles.id)".into())
    }
    for (table, id) in [("feeds", f.feed_id), ("folders", f.folder_id)] {
        if let Some(id) = id {
            if id <= 0
                || !db
                    .query_row(
                        &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE id=?)"),
                        [id],
                        |r| r.get::<_, bool>(0),
                    )
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            {
                return Err(StatusCode::BAD_REQUEST);
            }
            c.push(
                if table == "feeds" {
                    "feed_id=?"
                } else {
                    "feed_id IN (SELECT id FROM feeds WHERE folder_id=?)"
                }
                .into(),
            );
            v.push(id.into())
        }
    }
    if let Some(ids) = &f.feed_ids {
        let mut seen = HashSet::new();
        let ids = csv(ids, 50)?;
        let mut parsed = Vec::new();
        for s in ids {
            if !s.bytes().all(|x| x.is_ascii_digit()) {
                return Err(StatusCode::BAD_REQUEST);
            }
            let id: i64 = s.parse().map_err(|_| StatusCode::BAD_REQUEST)?;
            if id <= 0
                || !seen.insert(id)
                || !db
                    .query_row("SELECT EXISTS(SELECT 1 FROM feeds WHERE id=?)", [id], |r| {
                        r.get::<_, bool>(0)
                    })
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            {
                return Err(StatusCode::BAD_REQUEST);
            }
            parsed.push(id)
        }
        c.push(format!(
            "feed_id IN ({})",
            vec!["?"; parsed.len()].join(",")
        ));
        v.extend(parsed.into_iter().map(Value::from));
    }
    let field = match f.scope.as_deref().unwrap_or("both") {
        "both" => "(title || ' ' || content)",
        "title" => "title",
        "body" => "content",
        _ => return Err(StatusCode::BAD_REQUEST),
    };
    if let Some(q) = &f.q {
        if q.len() > 256 {
            return Err(StatusCode::BAD_REQUEST);
        }
        if !q.is_empty() {
            c.push(format!("instr(lower({field}),lower(?))>0"));
            v.push(q.clone().into())
        }
    }
    if let Some(s) = &f.any {
        let terms = csv(s, 20)?;
        c.push(format!(
            "({})",
            vec![format!("instr(lower({field}),lower(?))>0"); terms.len()].join(" OR ")
        ));
        v.extend(terms.into_iter().map(Value::from))
    }
    if let Some(s) = &f.exclude {
        for term in csv(s, 20)? {
            c.push(format!("instr(lower({field}),lower(?))=0"));
            v.push(term.into())
        }
    }
    for (d, op) in [(&f.published_from, ">="), (&f.published_to, "<=")] {
        if let Some(d) = d {
            if !date(d) {
                return Err(StatusCode::BAD_REQUEST);
            }
            c.push(format!("date(published) {op} date(?)"));
            v.push(d.clone().into())
        }
    }
    if f.published_from
        .as_ref()
        .zip(f.published_to.as_ref())
        .is_some_and(|(a, b)| a > b)
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    let order = match f.order.as_deref().unwrap_or(if f.history.unwrap_or(false) {
        "recent"
    } else {
        "new"
    }) {
        "new" => "julianday(published) IS NULL,julianday(published) DESC,id DESC",
        "old" => "julianday(published) IS NULL,julianday(published) ASC,id ASC",
        "recent" => {
            "(SELECT visited_at FROM article_visits WHERE article_id=articles.id) DESC,id DESC"
        }
        _ => return Err(StatusCode::BAD_REQUEST),
    };
    Ok(Scope {
        sql: if c.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", c.join(" AND "))
        },
        values: v,
        order,
    })
}
pub async fn list(
    State(db): State<Db>,
    RawQuery(raw): RawQuery,
) -> Result<Json<Listing>, StatusCode> {
    let q = parse_query(raw.as_deref().unwrap_or(""))?;
    let db = db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if q.limit.is_some_and(|n| n == 0 || n > 100) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let s = scope(&db, &q.filters)?;
    let total = db
        .query_row(
            &format!("SELECT count(*) FROM articles{}", s.sql),
            rusqlite::params_from_iter(s.values.iter()),
            |r| r.get(0),
        )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut v = s.values;
    v.push(i64::from(q.limit.unwrap_or(20)).into());
    v.push(i64::from(q.offset.unwrap_or(0)).into());
    let mut st=db.prepare(&format!("SELECT id,title,content,url,published,is_read,is_starred,(SELECT visited_at FROM article_visits WHERE article_id=articles.id) FROM articles{} ORDER BY {} LIMIT ? OFFSET ?",s.sql,s.order)).map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
    let items = st
        .query_map(rusqlite::params_from_iter(v.iter()), article)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .collect::<Result<_, _>>()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(Listing { items, total }))
}
pub async fn visit(
    State(db): State<Db>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<Json<Article>, StatusCode> {
    if !origin_allowed(&headers) {
        return Err(StatusCode::FORBIDDEN);
    }
    let mut db = db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let tx = db
        .transaction()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if tx
        .execute("UPDATE articles SET is_read=1 WHERE id=?", [id])
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        == 0
    {
        return Err(StatusCode::NOT_FOUND);
    }
    tx.execute("INSERT INTO article_visits(article_id,visited_at) VALUES (?1, strftime('%Y-%m-%dT%H:%M:%fZ', max(julianday('now'), coalesce((SELECT max(julianday(visited_at)) + 1.0/86400000 FROM article_visits),0)))) ON CONFLICT(article_id) DO UPDATE SET visited_at=excluded.visited_at",[id]).map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
    let item = lookup(&tx, id)?;
    tx.commit().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(item))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadAll {
    filters: Filters,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bulk {
    ids: Vec<i64>,
    read: Option<bool>,
    starred: Option<bool>,
    filters: Filters,
}
pub async fn read_all(
    State(db): State<Db>,
    headers: HeaderMap,
    Json(input): Json<ReadAll>,
) -> Result<Json<Updated>, StatusCode> {
    if !origin_allowed(&headers) {
        return Err(StatusCode::FORBIDDEN);
    }
    let mut db = db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let tx = db
        .transaction()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let s = scope(&tx, &input.filters)?;
    let updated = tx
        .execute(
            &format!("UPDATE articles SET is_read=1{}", s.sql),
            rusqlite::params_from_iter(s.values.iter()),
        )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    tx.commit().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(Updated { updated }))
}
pub async fn bulk_state(
    State(db): State<Db>,
    headers: HeaderMap,
    Json(input): Json<Bulk>,
) -> Result<Json<Updated>, StatusCode> {
    if !origin_allowed(&headers) {
        return Err(StatusCode::FORBIDDEN);
    }
    if input.ids.is_empty()
        || input.ids.len() > 100
        || input.read.is_none() && input.starred.is_none()
        || input.ids.iter().any(|id| *id <= 0)
        || input.ids.iter().collect::<HashSet<_>>().len() != input.ids.len()
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut db = db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let tx = db
        .transaction()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let s = scope(&tx, &input.filters)?;
    let w = format!(
        "{}{} id IN ({})",
        s.sql,
        if s.sql.is_empty() { " WHERE" } else { " AND" },
        vec!["?"; input.ids.len()].join(",")
    );
    let mut v = s.values;
    v.extend(input.ids.iter().copied().map(Value::from));
    let count: i64 = tx
        .query_row(
            &format!("SELECT count(*) FROM articles{w}"),
            rusqlite::params_from_iter(v.iter()),
            |r| r.get(0),
        )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if count != input.ids.len() as i64 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut p = vec![
        input
            .read
            .map_or(Value::Null, |x| Value::Integer(i64::from(x))),
        input
            .starred
            .map_or(Value::Null, |x| Value::Integer(i64::from(x))),
    ];
    p.extend(v);
    let updated=tx.execute(&format!("UPDATE articles SET is_read=COALESCE(?,is_read),is_starred=COALESCE(?,is_starred){w}"),rusqlite::params_from_iter(p.iter())).map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
    tx.commit().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(Updated { updated }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use tower::ServiceExt;
    fn fixture() -> (tempfile::TempDir, Db) {
        let dir = tempfile::tempdir().unwrap();
        let db = Arc::new(Mutex::new(
            open_db(&dir.path().join("reader.db"), false).unwrap(),
        ));
        {
            let c = db.lock().unwrap();
            c.execute("INSERT INTO folders(id,name) VALUES (1,'folder')", [])
                .unwrap();
            c.execute("INSERT INTO feeds(id,url,title,folder_id) VALUES (1,'https://example.org/a','A',1),(2,'https://example.org/b','B',NULL)",[]).unwrap();
            for (id, title, content, published, feed) in [
                (1, "Alpha", "rust, daily", "2026-11-06T00:30:00+09:00", 1),
                (2, "Beta", "Rust news", "2026-11-05T23:00:00Z", 2),
                (3, "Gamma", "rust daily skip", "", 1),
                (4, "Alpha quoted", "safe", "2026-11-06T01:00:00Z", 1),
                (5, "Tie", "safe", "2026-11-06T01:00:00Z", 1),
            ] {
                c.execute("INSERT INTO articles(id,title,content,url,published,feed_id) VALUES (?,?,?,'https://example.org',?,?)",params![id,title,content,published,feed]).unwrap();
            }
        }
        (dir, db)
    }
    async fn call(
        db: Db,
        method: &str,
        path: &str,
        body: serde_json::Value,
    ) -> (StatusCode, serde_json::Value) {
        let req = Request::builder()
            .method(method)
            .uri(path)
            .header("host", "127.0.0.1:8080")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap();
        let r = app(db, "127.0.0.1:8080".parse().unwrap())
            .oneshot(req)
            .await
            .unwrap();
        let status = r.status();
        let bytes = to_bytes(r.into_body(), 1_000_000).await.unwrap();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null),
        )
    }
    fn ids(v: &serde_json::Value) -> Vec<i64> {
        v["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a["id"].as_i64().unwrap())
            .collect()
    }
    #[tokio::test]
    async fn detailed_filters_dates_order_and_invalid_scopes() {
        let (_dir, db) = fixture();
        let q = url::form_urlencoded::Serializer::new(String::new())
            .append_pair("scope", "body")
            .append_pair("any", "\"rust, daily\",news")
            .append_pair("exclude", "skip")
            .finish();
        let (status, v) = call(
            db.clone(),
            "GET",
            &format!("/api/articles?{q}"),
            serde_json::Value::Null,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(ids(&v), vec![2, 1]);
        let (_, v) = call(
            db.clone(),
            "GET",
            "/api/articles?published_from=2026-11-05&published_to=2026-11-05&order=old",
            serde_json::Value::Null,
        )
        .await;
        assert_eq!(ids(&v), vec![1, 2]);
        let (_, v) = call(
            db.clone(),
            "GET",
            "/api/articles?order=new&limit=2",
            serde_json::Value::Null,
        )
        .await;
        assert_eq!(ids(&v), vec![5, 4]);
        let (_, v) = call(
            db.clone(),
            "GET",
            "/api/articles?feed_ids=1,2&folder_id=1",
            serde_json::Value::Null,
        )
        .await;
        assert_eq!(v["total"], 4);
        for query in [
            "published_from=2026-02-30",
            "published_from=2026-11-07&published_to=2026-11-06",
            "scope=sql",
            "order=unknown",
            "feed_id=999",
            "feed_ids=1,1",
            "any=%22unclosed",
            "q=a&q=b",
            "unknown=1",
            "unread=maybe",
        ] {
            assert_eq!(
                call(
                    db.clone(),
                    "GET",
                    &format!("/api/articles?{query}"),
                    serde_json::Value::Null
                )
                .await
                .0,
                StatusCode::BAD_REQUEST,
                "{query}"
            );
        }
    }
    #[tokio::test]
    async fn bulk_rejects_hostile_origin_and_oversized_selection() {
        let (_dir, db) = fixture();
        let req = Request::builder()
            .method("POST")
            .uri("/api/articles/read-all")
            .header("host", "127.0.0.1:8080")
            .header("origin", "https://attacker.invalid")
            .header("content-type", "application/json")
            .body(Body::from("{\"filters\":{}}"))
            .unwrap();
        let r = app(db.clone(), "127.0.0.1:8080".parse().unwrap())
            .oneshot(req)
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::FORBIDDEN);
        assert_eq!(
            call(
                db.clone(),
                "POST",
                "/api/articles/bulk-state",
                serde_json::json!({"ids":(1..=101).collect::<Vec<_>>(),"read":true,"filters":{}})
            )
            .await
            .0,
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            db.lock()
                .unwrap()
                .query_row("SELECT sum(is_read) FROM articles", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
    #[tokio::test]
    async fn bulk_atomicity_scope_and_no_history_side_effects() {
        let (_dir, db) = fixture();
        for body in [
            serde_json::json!({"ids":[1,999],"read":true,"filters":{}}),
            serde_json::json!({"ids":[1,2],"read":true,"filters":{"feed_id":1}}),
            serde_json::json!({"ids":[1,1],"read":true,"filters":{}}),
            serde_json::json!({"ids":[],"read":true,"filters":{}}),
        ] {
            assert_eq!(
                call(db.clone(), "POST", "/api/articles/bulk-state", body)
                    .await
                    .0,
                StatusCode::BAD_REQUEST
            );
            assert_eq!(
                db.lock()
                    .unwrap()
                    .query_row("SELECT sum(is_read) FROM articles", [], |r| r
                        .get::<_, i64>(0))
                    .unwrap(),
                0
            );
        }
        let (status, _) = call(
            db.clone(),
            "POST",
            "/api/articles/read-all",
            serde_json::json!({"filters":{"limit":1}}),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        let (status, v) = call(
            db.clone(),
            "POST",
            "/api/articles/read-all",
            serde_json::json!({"filters":{"folder_id":1}}),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(v["updated"], 4);
        assert!(!lookup(&db.lock().unwrap(), 2).unwrap().is_read);
        assert_eq!(
            db.lock()
                .unwrap()
                .query_row("SELECT count(*) FROM article_visits", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
    #[tokio::test]
    async fn visits_persist_and_revisit_reorders_without_star_loss() {
        let (dir, db) = fixture();
        call(
            db.clone(),
            "POST",
            "/api/articles/1/state",
            serde_json::json!({"starred":true}),
        )
        .await;
        call(
            db.clone(),
            "GET",
            "/api/articles/1",
            serde_json::Value::Null,
        )
        .await;
        assert_eq!(
            db.lock()
                .unwrap()
                .query_row("SELECT count(*) FROM article_visits", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
        for id in [1, 2, 1] {
            assert_eq!(
                call(
                    db.clone(),
                    "POST",
                    &format!("/api/articles/{id}/visit"),
                    serde_json::Value::Null
                )
                .await
                .0,
                StatusCode::OK
            );
        }
        let (_, v) = call(
            db.clone(),
            "GET",
            "/api/articles?history=true&order=recent",
            serde_json::Value::Null,
        )
        .await;
        assert_eq!(ids(&v), vec![1, 2]);
        assert_eq!(v["items"][0]["is_starred"], true);
        drop(db);
        let c = open_db(&dir.path().join("reader.db"), false).unwrap();
        assert_eq!(
            c.query_row("SELECT count(*) FROM article_visits", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            2
        );
    }
    #[test]
    fn v3_migration_keeps_data_and_a_backup() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v3.db");
        let c = open_db(&path, true).unwrap();
        c.execute("UPDATE articles SET is_read=1,is_starred=1 WHERE id=1", [])
            .unwrap();
        for sql in [
            "DROP TRIGGER folder_order_insert",
            "DROP TRIGGER feed_order_insert",
            "DROP TRIGGER feed_order_move",
            "DROP TABLE feed_order",
            "DROP TABLE folder_order",
        ] {
            c.execute(sql, []).unwrap();
        }
        c.execute("DROP TABLE article_visits", []).unwrap();
        c.pragma_update(None, "user_version", 3).unwrap();
        drop(c);
        let migrated = open_db(&path, false).unwrap();
        let a = lookup(&migrated, 1).unwrap();
        assert!(a.is_read && a.is_starred);
        assert!(a.last_visited.is_none());
        drop(migrated);
        let backup = Connection::open(path.with_extension("v3-backup.sqlite3")).unwrap();
        assert_eq!(
            backup
                .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
                .unwrap(),
            3
        );
    }
}
