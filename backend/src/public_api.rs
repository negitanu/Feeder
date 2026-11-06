use super::*;
use axum::{
    extract::{RawQuery, rejection::JsonRejection},
    response::{IntoResponse, Response},
};
use rusqlite::types::Value;
use std::collections::HashSet;

struct ApiError(StatusCode);
impl From<StatusCode> for ApiError {
    fn from(s: StatusCode) -> Self {
        Self(s)
    }
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let s = self.0;
        (s, Json(serde_json::json!({"error":{"code":s.as_u16(),"message":s.canonical_reason().unwrap_or("Error")}}))).into_response()
    }
}
type ResultApi<T> = Result<T, ApiError>;
fn bad() -> ApiError {
    ApiError(StatusCode::BAD_REQUEST)
}
fn internal<T>(_: T) -> ApiError {
    ApiError(StatusCode::INTERNAL_SERVER_ERROR)
}
fn body<T>(v: Result<Json<T>, JsonRejection>) -> ResultApi<T> {
    v.map(|Json(v)| v).map_err(|e| {
        ApiError(if e.status() == StatusCode::UNPROCESSABLE_ENTITY {
            StatusCode::BAD_REQUEST
        } else {
            e.status()
        })
    })
}
fn envelope<T: Serialize>(data: T) -> Json<serde_json::Value> {
    Json(serde_json::json!({"data":data}))
}

#[derive(Serialize)]
struct FeedDto {
    id: i64,
    url: String,
    title: String,
    folder: Option<String>,
    enabled: bool,
    source_type: &'static str,
    poll_interval: Option<i64>,
    last_checked: Option<String>,
    error: Option<String>,
}
fn feed_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<FeedDto> {
    Ok(FeedDto {
        id: r.get(0)?,
        url: r.get(1)?,
        title: r.get(2)?,
        folder: r.get(3)?,
        enabled: !r.get::<_, bool>(4)?,
        source_type: "rss",
        poll_interval: r.get(7)?,
        last_checked: r.get(5)?,
        error: r.get(6)?,
    })
}
const FEED_SELECT: &str = "SELECT f.id,f.url,f.title,d.name,f.paused,f.last_checked,f.last_error,(SELECT interval_seconds FROM feed_poll_intervals WHERE feed_id=f.id) FROM feeds f LEFT JOIN folders d ON d.id=f.folder_id";
fn get_feed(db: &Connection, id: i64) -> ResultApi<FeedDto> {
    db.query_row(&format!("{FEED_SELECT} WHERE f.id=?"), [id], feed_row)
        .optional()
        .map_err(internal)?
        .ok_or(ApiError(StatusCode::NOT_FOUND))
}
async fn feeds(
    State(db): State<Db>,
    RawQuery(query): RawQuery,
) -> ResultApi<Json<serde_json::Value>> {
    if query.is_some_and(|q| !q.is_empty()) {
        return Err(bad());
    }
    let db = db.lock().map_err(internal)?;
    let mut stmt = db
        .prepare(&format!("{FEED_SELECT} ORDER BY f.id"))
        .map_err(internal)?;
    let rows = stmt
        .query_map([], feed_row)
        .map_err(internal)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(internal)?;
    Ok(envelope(rows))
}
async fn feed_detail(
    State(db): State<Db>,
    Path(id): Path<i64>,
) -> ResultApi<Json<serde_json::Value>> {
    let db = db.lock().map_err(internal)?;
    Ok(envelope(get_feed(&db, id)?))
}
fn folder_name(s: &str) -> ResultApi<&str> {
    let s = s.trim();
    if s.chars().count() > 80 || s.chars().any(char::is_control) {
        return Err(bad());
    }
    Ok(s)
}
fn ensure_folder(db: &Connection, name: &str) -> ResultApi<Option<i64>> {
    if name.is_empty() {
        return Ok(None);
    }
    db.execute(
        "INSERT INTO folders(name) VALUES (?) ON CONFLICT(name) DO NOTHING",
        [name],
    )
    .map_err(internal)?;
    Ok(Some(
        db.query_row("SELECT id FROM folders WHERE name=?", [name], |r| r.get(0))
            .map_err(internal)?,
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NewFeed {
    url: String,
    #[serde(default, deserialize_with = "strict_string")]
    folder: Option<String>,
    #[serde(default, deserialize_with = "strict_string")]
    source_type: Option<String>,
}
async fn create(
    State(db): State<Db>,
    input: Result<Json<NewFeed>, JsonRejection>,
) -> ResultApi<(
    StatusCode,
    [(header::HeaderName, String); 1],
    Json<serde_json::Value>,
)> {
    let input = body(input)?;
    if input.source_type.as_deref().is_some_and(|s| s != "rss") {
        return Err(bad());
    }
    let folder = folder_name(input.folder.as_deref().unwrap_or(""))?.to_owned();
    let created = collection::add_feed_in_folder(db.clone(), &input.url, &folder).await?;
    let db = db.lock().map_err(internal)?;
    Ok((
        StatusCode::CREATED,
        [(header::LOCATION, format!("/api/v1/feeds/{}", created.id))],
        envelope(get_feed(&db, created.id)?),
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EditFeed {
    #[serde(default, deserialize_with = "strict_string")]
    title: Option<String>,
    #[serde(default, deserialize_with = "strict_string")]
    url: Option<String>,
    #[serde(default, deserialize_with = "strict_string")]
    folder: Option<String>,
    #[serde(default, deserialize_with = "strict_bool")]
    enabled: Option<bool>,
    #[serde(default, deserialize_with = "collection::double_interval")]
    poll_interval: Option<Option<i64>>,
}
async fn edit(
    State(db): State<Db>,
    Path(id): Path<i64>,
    input: Result<Json<EditFeed>, JsonRejection>,
) -> ResultApi<Json<serde_json::Value>> {
    let input = body(input)?;
    if input.title.is_none()
        && input.url.is_none()
        && input.folder.is_none()
        && input.enabled.is_none()
        && input.poll_interval.is_none()
    {
        return Err(bad());
    }
    let title = input
        .title
        .as_deref()
        .map(collection::clean_name)
        .transpose()?;
    let folder = input.folder.as_deref().map(folder_name).transpose()?;
    let mut db = db.lock().map_err(internal)?;
    let tx = db
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(internal)?;
    collection::ensure_not_refreshing(&tx, id)?;
    let old = get_feed(&tx, id)?;
    collection::edit_url(&tx, id, input.url.as_deref())?;
    collection::set_poll_interval(&tx, id, input.poll_interval)?;
    let fid = match folder {
        Some(s) => ensure_folder(&tx, s)?,
        None => tx
            .query_row("SELECT folder_id FROM feeds WHERE id=?", [id], |r| r.get(0))
            .map_err(internal)?,
    };
    tx.execute(
        "UPDATE feeds SET title=?,folder_id=?,paused=? WHERE id=?",
        params![
            title.unwrap_or(&old.title),
            fid,
            !input.enabled.unwrap_or(old.enabled),
            id
        ],
    )
    .map_err(internal)?;
    let result = get_feed(&tx, id)?;
    tx.commit().map_err(internal)?;
    Ok(envelope(result))
}
async fn remove(State(db): State<Db>, Path(id): Path<i64>) -> ResultApi<StatusCode> {
    let mut db = db.lock().map_err(internal)?;
    let tx = db
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(internal)?;
    collection::ensure_not_refreshing(&tx, id)?;
    if tx
        .execute("DELETE FROM feeds WHERE id=?", [id])
        .map_err(internal)?
        == 0
    {
        return Err(ApiError(StatusCode::NOT_FOUND));
    }
    tx.commit().map_err(internal)?;
    Ok(StatusCode::NO_CONTENT)
}
async fn refresh(State(db): State<Db>, Path(id): Path<i64>) -> ResultApi<Json<serde_json::Value>> {
    collection::refresh_feed(db.clone(), id, false).await?;
    let db = db.lock().map_err(internal)?;
    Ok(envelope(
        serde_json::json!({"succeeded":1,"failed":0,"feed":get_feed(&db,id)?}),
    ))
}

#[derive(Serialize)]
struct ArticleDto {
    id: i64,
    feed_id: Option<i64>,
    guid: Option<String>,
    title: String,
    link: String,
    content: String,
    published_at: String,
    is_read: bool,
    is_starred: bool,
    feed_title: Option<String>,
    source_type: &'static str,
}
fn article_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<ArticleDto> {
    Ok(ArticleDto {
        id: r.get(0)?,
        feed_id: r.get(1)?,
        guid: r.get(2)?,
        title: r.get(3)?,
        link: r.get(4)?,
        content: r.get(5)?,
        published_at: r.get(6)?,
        is_read: r.get(7)?,
        is_starred: r.get(8)?,
        feed_title: r.get(9)?,
        source_type: "rss",
    })
}
const ARTICLE_SELECT: &str = "SELECT a.id,a.feed_id,a.external_id,a.title,a.url,a.content,a.published,a.is_read,a.is_starred,f.title FROM articles a LEFT JOIN feeds f ON f.id=a.feed_id";
fn get_article(db: &Connection, id: i64) -> ResultApi<ArticleDto> {
    db.query_row(&format!("{ARTICLE_SELECT} WHERE a.id=?"), [id], article_row)
        .optional()
        .map_err(internal)?
        .ok_or(ApiError(StatusCode::NOT_FOUND))
}
async fn articles(
    State(db): State<Db>,
    RawQuery(raw): RawQuery,
) -> ResultApi<Json<serde_json::Value>> {
    let raw = raw.unwrap_or_default();
    if raw.len() > 8192 {
        return Err(bad());
    }
    let mut seen = HashSet::new();
    let mut limit = 30_i64;
    let mut before = None;
    let mut after = None;
    let mut clauses = Vec::new();
    let mut values: Vec<Value> = Vec::new();
    for (k, v) in url::form_urlencoded::parse(raw.as_bytes()) {
        if !seen.insert(k.to_string()) {
            return Err(bad());
        }
        match k.as_ref() {
            "limit" => {
                limit = v.parse().map_err(|_| bad())?;
                if !(1..=100).contains(&limit) {
                    return Err(bad());
                }
            }
            "before_id" | "after_id" => {
                let n: i64 = v.parse().map_err(|_| bad())?;
                if n < 0 || (k == "before_id" && n == 0) {
                    return Err(bad());
                }
                if k == "before_id" {
                    before = Some(n)
                } else {
                    after = Some(n)
                }
            }
            "feed" => {
                let n: i64 = v.parse().map_err(|_| bad())?;
                if n <= 0 {
                    return Err(bad());
                }
                clauses.push("a.feed_id=?");
                values.push(n.into());
            }
            "is_read" | "is_starred" => {
                let b = match v.as_ref() {
                    "true" => 1,
                    "false" => 0,
                    _ => return Err(bad()),
                };
                clauses.push(if k == "is_read" {
                    "a.is_read=?"
                } else {
                    "a.is_starred=?"
                });
                values.push(Value::Integer(b));
            }
            "q" => {
                if v.chars().count() > 200 {
                    return Err(bad());
                }
                clauses.push(
                    "(instr(lower(a.title),lower(?))>0 OR instr(lower(a.content),lower(?))>0)",
                );
                values.push(v.to_string().into());
                values.push(v.to_string().into());
            }
            "source_type" => {
                if v != "rss" {
                    return Err(bad());
                }
            }
            _ => return Err(bad()),
        }
    }
    if before.is_some() && after.is_some() {
        return Err(bad());
    }
    if let Some(n) = before {
        clauses.push("a.id<?");
        values.push(n.into())
    }
    if let Some(n) = after {
        clauses.push("a.id>?");
        values.push(n.into())
    }
    let where_sql = if clauses.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", clauses.join(" AND "))
    };
    let order = if after.is_some() { "ASC" } else { "DESC" };
    let sql = format!("{ARTICLE_SELECT}{where_sql} ORDER BY a.id {order} LIMIT ?");
    values.push((limit + 1).into());
    let db = db.lock().map_err(internal)?;
    let mut stmt = db.prepare(&sql).map_err(internal)?;
    let mut rows = stmt
        .query_map(rusqlite::params_from_iter(values), article_row)
        .map_err(internal)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(internal)?;
    let more = rows.len() > limit as usize;
    rows.truncate(limit as usize);
    let cursor = rows.last().map(|a| a.id);
    Ok(Json(
        serde_json::json!({"data":rows,"has_more":more,"next_before_id":if after.is_none() && more {cursor}else{None},"next_after_id":if after.is_some(){cursor.or(after)}else{None}}),
    ))
}
async fn article_detail(
    State(db): State<Db>,
    Path(id): Path<i64>,
) -> ResultApi<Json<serde_json::Value>> {
    let db = db.lock().map_err(internal)?;
    Ok(envelope(get_article(&db, id)?))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EditArticle {
    #[serde(default, deserialize_with = "strict_bool")]
    is_read: Option<bool>,
    #[serde(default, deserialize_with = "strict_bool")]
    is_starred: Option<bool>,
}
fn strict_string<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    String::deserialize(d).map(Some)
}
fn strict_bool<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<bool>, D::Error> {
    bool::deserialize(d).map(Some)
}
async fn article_edit(
    State(db): State<Db>,
    Path(id): Path<i64>,
    input: Result<Json<EditArticle>, JsonRejection>,
) -> ResultApi<Json<serde_json::Value>> {
    let input = body(input)?;
    if input.is_read.is_none() && input.is_starred.is_none() {
        return Err(bad());
    }
    let db = db.lock().map_err(internal)?;
    let changed = db.execute(
        "UPDATE articles SET is_read=COALESCE(?,is_read),is_starred=COALESCE(?,is_starred) WHERE id=?",
        params![input.is_read, input.is_starred, id],
    ).map_err(internal)?;
    if changed == 0 {
        return Err(ApiError(StatusCode::NOT_FOUND));
    }
    Ok(envelope(get_article(&db, id)?))
}
pub(super) fn routes() -> Router<Db> {
    Router::new()
        .route("/api/v1/feeds", get(feeds).post(create))
        .route(
            "/api/v1/feeds/{id}",
            get(feed_detail).patch(edit).delete(remove),
        )
        .route("/api/v1/feeds/{id}/refresh", post(refresh))
        .route("/api/v1/articles", get(articles))
        .route(
            "/api/v1/articles/{id}",
            get(article_detail).patch(article_edit),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn pagination_validation_and_state_do_not_visit() {
        let dir = tempfile::tempdir().unwrap();
        let db = Arc::new(Mutex::new(
            open_db(&dir.path().join("test.db"), true).unwrap(),
        ));
        let router = routes().with_state(db.clone());
        let call = |uri: &str, method: &str, body: &str| {
            Request::builder()
                .uri(uri)
                .method(method)
                .header("content-type", "application/json")
                .body(Body::from(body.to_owned()))
                .unwrap()
        };
        let mut cursor = 0;
        let mut ids = Vec::new();
        loop {
            let response = router
                .clone()
                .oneshot(call(
                    &format!("/api/v1/articles?after_id={cursor}&limit=7"),
                    "GET",
                    "",
                ))
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let value: serde_json::Value =
                serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                    .unwrap();
            let page = value["data"].as_array().unwrap();
            ids.extend(page.iter().map(|v| v["id"].as_i64().unwrap()));
            if !value["has_more"].as_bool().unwrap() {
                break;
            }
            cursor = value["next_after_id"].as_i64().unwrap();
        }
        assert_eq!(ids, (1..=36).collect::<Vec<_>>());
        for uri in [
            "/api/v1/articles?limit=1&limit=2",
            "/api/v1/articles?before_id=1&after_id=1",
            "/api/v1/articles?source_type=page",
        ] {
            assert_eq!(
                router
                    .clone()
                    .oneshot(call(uri, "GET", ""))
                    .await
                    .unwrap()
                    .status(),
                StatusCode::BAD_REQUEST
            );
        }
        assert_eq!(
            router
                .clone()
                .oneshot(call(
                    "/api/v1/articles/1",
                    "PATCH",
                    r#"{"is_read":true,"is_starred":"yes"}"#
                ))
                .await
                .unwrap()
                .status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            router
                .clone()
                .oneshot(call(
                    "/api/v1/articles/1",
                    "PATCH",
                    r#"{"is_read":true,"is_starred":true}"#
                ))
                .await
                .unwrap()
                .status(),
            StatusCode::OK
        );
        let invalid = router
            .clone()
            .oneshot(call(
                "/api/v1/feeds",
                "POST",
                r#"{"url":"http://127.0.0.1/feed","folder":"Should not exist"}"#,
            ))
            .await
            .unwrap();
        assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
        let db = db.lock().unwrap();
        assert_eq!(
            db.query_row("SELECT COUNT(*) FROM feeds", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert_eq!(
            db.query_row(
                "SELECT COUNT(*) FROM folders WHERE name='Should not exist'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
        assert!(
            db.query_row(
                "SELECT is_read AND is_starred FROM articles WHERE id=1",
                [],
                |r| r.get::<_, bool>(0)
            )
            .unwrap()
        );
        assert_eq!(
            db.query_row("SELECT COUNT(*) FROM article_visits", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
    #[tokio::test]
    async fn fixture_feed_crud_and_atomic_validation() {
        let dir = tempfile::tempdir().unwrap();
        let db = Arc::new(Mutex::new(
            open_db(&dir.path().join("test.db"), false).unwrap(),
        ));
        {
            let db = db.lock().unwrap();
            db.execute_batch("INSERT INTO feeds(id,url,title) VALUES (1,'https://example.org/rss','Original'); INSERT INTO articles(id,feed_id,external_id,title,content,url,published,is_read,is_starred) VALUES (1,1,'g','100% real','body','https://example.org/a','2026',1,1);").unwrap();
        }
        let router = routes().with_state(db.clone());
        let request = |uri: &str, method: &str, body: &str| {
            Request::builder()
                .uri(uri)
                .method(method)
                .header("content-type", "application/json")
                .body(Body::from(body.to_owned()))
                .unwrap()
        };
        for body in [
            r#"{"title":"Changed","enabled":"true"}"#,
            r#"{"title":null}"#,
            r#"{"folder":null}"#,
            r#"{"poll_interval":59}"#,
            r#"{"url":"http://127.0.0.1/"}"#,
        ] {
            assert_eq!(
                router
                    .clone()
                    .oneshot(request("/api/v1/feeds/1", "PATCH", body))
                    .await
                    .unwrap()
                    .status(),
                StatusCode::BAD_REQUEST
            );
        }
        for body in [
            r#"{"url":"https://example.org","source_type":"page"}"#,
            r#"{"url":"https://example.org","source_type":null}"#,
            r#"{"url":"https://example.org","poll_interval":300}"#,
        ] {
            assert_eq!(
                router
                    .clone()
                    .oneshot(request("/api/v1/feeds", "POST", body))
                    .await
                    .unwrap()
                    .status(),
                StatusCode::BAD_REQUEST
            );
        }
        assert_eq!(
            router
                .clone()
                .oneshot(request(
                    "/api/v1/feeds/1",
                    "PATCH",
                    r#"{"title":"New","folder":"News","enabled":false}"#
                ))
                .await
                .unwrap()
                .status(),
            StatusCode::OK
        );
        assert_eq!(
            router
                .clone()
                .oneshot(request(
                    "/api/v1/articles/1",
                    "PATCH",
                    r#"{"is_read":false}"#
                ))
                .await
                .unwrap()
                .status(),
            StatusCode::OK
        );
        let response = router
            .clone()
            .oneshot(request(
                "/api/v1/articles?feed=1&is_starred=true&q=%25",
                "GET",
                "",
            ))
            .await
            .unwrap();
        let value: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(value["data"].as_array().unwrap().len(), 1);
        {
            let db = db.lock().unwrap();
            assert_eq!(
                db.query_row("SELECT title FROM feeds WHERE id=1", [], |r| r
                    .get::<_, String>(0))
                    .unwrap(),
                "New"
            );
            assert!(
                db.query_row(
                    "SELECT is_starred AND NOT is_read FROM articles WHERE id=1",
                    [],
                    |r| r.get::<_, bool>(0)
                )
                .unwrap()
            );
            assert_eq!(
                db.query_row("SELECT COUNT(*) FROM article_visits", [], |r| r
                    .get::<_, i64>(0))
                    .unwrap(),
                0
            );
        }
        assert_eq!(
            router
                .oneshot(request("/api/v1/feeds/1", "DELETE", ""))
                .await
                .unwrap()
                .status(),
            StatusCode::NO_CONTENT
        );
        let db = db.lock().unwrap();
        assert_eq!(
            db.query_row("SELECT COUNT(*) FROM articles", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
}
