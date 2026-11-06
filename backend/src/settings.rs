use super::*;
use axum::{
    response::Response,
    routing::{get, post},
};
use std::collections::HashSet;

const SETTINGS: &str = "CREATE TABLE reader_settings (id INTEGER PRIMARY KEY CHECK(id=1), display_timezone TEXT NOT NULL DEFAULT 'UTC' CHECK(display_timezone IN ('UTC','Asia/Tokyo')), default_view TEXT NOT NULL DEFAULT 'all' CHECK(default_view IN ('all','unread','starred')))";
type ExportFeed = (i64, String, String, Option<i64>, bool, Option<i64>);
pub fn migrate(tx: &rusqlite::Transaction<'_>) -> rusqlite::Result<()> {
    tx.execute(SETTINGS, [])?;
    tx.execute("INSERT INTO reader_settings(id) VALUES (1)", [])?;
    Ok(())
}
pub fn recognized_v6(objects: &[(String, String, Option<String>)]) -> bool {
    let Ok(mut db) = Connection::open_in_memory() else {
        return false;
    };
    if db.execute(SCHEMA, []).is_err() {
        return false;
    }
    let Ok(tx) = db.transaction() else {
        return false;
    };
    if collection::migrate(&tx).is_err()
        || tx.execute(reader::VISITS, []).is_err()
        || collection::migrate_order(&tx).is_err()
        || migrate(&tx).is_err()
        || tx.commit().is_err()
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
#[derive(Serialize)]
pub struct ReaderSettings {
    pub display_timezone: String,
    pub default_view: String,
}
pub fn read(db: &Connection) -> Result<ReaderSettings, StatusCode> {
    db.query_row(
        "SELECT display_timezone,default_view FROM reader_settings WHERE id=1",
        [],
        |r| {
            Ok(ReaderSettings {
                display_timezone: r.get(0)?,
                default_view: r.get(1)?,
            })
        },
    )
    .map_err(collection::err)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Patch {
    #[serde(default, deserialize_with = "non_null_string")]
    display_timezone: Option<String>,
    #[serde(default, deserialize_with = "non_null_string")]
    default_view: Option<String>,
}
fn non_null_string<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<String>, D::Error> {
    String::deserialize(deserializer).map(Some)
}
async fn get_settings(State(db): State<Db>) -> Result<Json<ReaderSettings>, StatusCode> {
    let db = db.lock().map_err(collection::err)?;
    Ok(Json(read(&db)?))
}
async fn patch_settings(
    State(db): State<Db>,
    headers: HeaderMap,
    Json(input): Json<Patch>,
) -> Result<Json<ReaderSettings>, StatusCode> {
    if !origin_allowed(&headers) {
        return Err(StatusCode::FORBIDDEN);
    }
    if input.display_timezone.is_none() && input.default_view.is_none()
        || input
            .display_timezone
            .as_deref()
            .is_some_and(|v| !matches!(v, "UTC" | "Asia/Tokyo"))
        || input
            .default_view
            .as_deref()
            .is_some_and(|v| !matches!(v, "all" | "unread" | "starred"))
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    let db = db.lock().map_err(collection::err)?;
    db.execute("UPDATE reader_settings SET display_timezone=COALESCE(?1,display_timezone),default_view=COALESCE(?2,default_view) WHERE id=1", params![input.display_timezone,input.default_view]).map_err(collection::err)?;
    Ok(Json(read(&db)?))
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Presentation {
    appearance: String,
    width: i64,
    layout: String,
    collapsed_folders: Vec<i64>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExportInput {
    presentation: Presentation,
}
#[derive(Serialize)]
struct Omission {
    path: String,
    reason: &'static str,
}
fn sensitive(s: &str) -> bool {
    let s = s.to_ascii_lowercase();
    [
        "bearer", "sk-", "api_key", "api-key", "apikey", "token", "password", "secret",
    ]
    .iter()
    .any(|p| s.contains(p))
}
fn unsafe_embedded_url(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower
        .match_indices("http://")
        .chain(lower.match_indices("https://"))
        .any(|(start, _)| {
            let candidate = value[start..]
                .split(|c: char| c.is_whitespace() || matches!(c, '\"' | '\''))
                .next()
                .unwrap_or("");
            url::Url::parse(candidate).is_ok_and(|u| {
                !u.username().is_empty()
                    || u.password().is_some()
                    || u.query().is_some()
                    || u.fragment().is_some()
                    || sensitive(&decoded_path(u.path()))
            })
        })
}
fn decoded_path(path: &str) -> String {
    let mut decoded = path.to_owned();
    for _ in 0..3 {
        decoded = url::form_urlencoded::parse(decoded.as_bytes())
            .map(|(k, v)| format!("{k}{v}"))
            .collect();
    }
    decoded
}
fn clean_text(value: String, path: String, omissions: &mut Vec<Omission>) -> Option<String> {
    if sensitive(&value) || unsafe_embedded_url(&value) {
        omissions.push(Omission {
            path,
            reason: "credential-like value",
        });
        None
    } else {
        Some(value)
    }
}
fn clean_url(value: String, path: String, omissions: &mut Vec<Omission>) -> Option<String> {
    let safe = url::Url::parse(&value).is_ok_and(|u| {
        matches!(u.scheme(), "http" | "https")
            && u.username().is_empty()
            && u.password().is_none()
            && u.query().is_none()
            && u.fragment().is_none()
            && !sensitive(&decoded_path(u.path()))
    });
    if safe && !sensitive(&value) {
        Some(value)
    } else {
        omissions.push(Omission {
            path,
            reason: "unsafe or credential-like URL",
        });
        None
    }
}
async fn export(
    State(db): State<Db>,
    headers: HeaderMap,
    Json(input): Json<ExportInput>,
) -> Result<Response, StatusCode> {
    if !origin_allowed(&headers) {
        return Err(StatusCode::FORBIDDEN);
    }
    let p = &input.presentation;
    if !matches!(p.appearance.as_str(), "system" | "light" | "dark")
        || !(200..=480).contains(&p.width)
        || !matches!(p.layout.as_str(), "standard" | "line" | "tiles")
        || p.collapsed_folders.len() > 2000
        || p.collapsed_folders.iter().any(|v| *v <= 0)
        || p.collapsed_folders.iter().collect::<HashSet<_>>().len() != p.collapsed_folders.len()
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut db = db.lock().map_err(collection::err)?;
    let tx = db.transaction().map_err(collection::err)?;
    let (enabled, interval_seconds): (bool, i64) = tx
        .query_row(
            "SELECT enabled,interval_seconds FROM collection_settings WHERE id=1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(collection::err)?;
    let reader = read(&tx)?;
    let rate_limit: i64 = tx
        .query_row(
            "SELECT rate_limit FROM api_rate_settings WHERE id=1",
            [],
            |r| r.get(0),
        )
        .map_err(collection::err)?;
    let schema_version: i64 = tx
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .map_err(collection::err)?;
    let folders:Vec<(i64,String)>=tx.prepare("SELECT f.id,f.name FROM folders f JOIN folder_order o ON o.folder_id=f.id ORDER BY o.position,f.id").map_err(collection::err)?.query_map([],|r| Ok((r.get(0)?,r.get(1)?))).map_err(collection::err)?.collect::<Result<_,_>>().map_err(collection::err)?;
    let feeds:Vec<ExportFeed>=tx.prepare("SELECT f.id,f.url,f.title,f.folder_id,f.paused,(SELECT interval_seconds FROM feed_poll_intervals WHERE feed_id=f.id) FROM feeds f JOIN feed_order o ON o.feed_id=f.id LEFT JOIN folder_order fo ON fo.folder_id=f.folder_id ORDER BY f.folder_id IS NULL,fo.position,o.position,f.id").map_err(collection::err)?.query_map([],|r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).map_err(collection::err)?.collect::<Result<_,_>>().map_err(collection::err)?;
    let timestamp: String = tx
        .query_row("SELECT strftime('%Y-%m-%dT%H:%M:%fZ','now')", [], |r| {
            r.get(0)
        })
        .map_err(collection::err)?;
    let mut omissions = Vec::new();
    let folders:Vec<_>=folders.into_iter().enumerate().map(|(i,(id,name))| serde_json::json!({"id":id,"name":clean_text(name,format!("settings.folders[{i}].name"),&mut omissions)})).collect();
    let subscriptions:Vec<_>=feeds.into_iter().enumerate().map(|(i,(id,url,title,folder_id,paused,poll_interval))| serde_json::json!({"id":id,"url":clean_url(url,format!("settings.subscriptions[{i}].url"),&mut omissions),"title":clean_text(title,format!("settings.subscriptions[{i}].title"),&mut omissions),"folder_id":folder_id,"paused":paused,"poll_interval":poll_interval})).collect();
    let result = serde_json::json!({"format":"feeder-settings","format_version":1,"app_version":env!("CARGO_PKG_VERSION"),"schema_version":schema_version,"exported_at":timestamp,"settings":{"collection":{"enabled":enabled,"interval_seconds":interval_seconds},"reader":reader,"api":{"rate_limit":rate_limit},"folders":folders,"subscriptions":subscriptions,"presentation":input.presentation},"omissions":omissions,"unsupported_sections":["ai","connectors","import"]});
    tx.commit().map_err(collection::err)?;
    Ok((
        [
            (header::CONTENT_TYPE, "application/json; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"feeder-settings.json\"",
            ),
            (header::CACHE_CONTROL, "no-store"),
        ],
        Json(result),
    )
        .into_response())
}
pub fn routes() -> Router<Db> {
    Router::new()
        .route(
            "/api/reader/settings",
            get(get_settings).patch(patch_settings),
        )
        .route("/api/configuration/export", post(export))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use tower::ServiceExt;
    fn request(method: &str, uri: &str, body: &str, origin: &str) -> Request<Body> {
        Request::builder()
            .method(method)
            .uri(uri)
            .header("host", "127.0.0.1:8080")
            .header("origin", origin)
            .header("content-type", "application/json")
            .body(Body::from(body.to_owned()))
            .unwrap()
    }
    #[tokio::test]
    async fn settings_export_and_timezone_scope() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("db");
        let db = open_db(&path, false).unwrap();
        db.execute_batch("INSERT INTO folders(id,name) VALUES (1,'password-canary'); INSERT INTO feeds(id,url,title,folder_id,etag) VALUES (1,'https://example.com/feed?token=canary','Safe',1,'secret-canary'); INSERT INTO articles(id,title,content,url,published) VALUES (1,'a','body-canary','https://a','2026-01-01T14:59:59Z'),(2,'b','body-canary','https://b','2026-01-01T15:00:00Z'),(3,'c','body-canary','https://c','2026-01-02T14:59:59Z'),(4,'d','body-canary','https://d','2026-01-02T15:00:00Z');").unwrap();
        let db = Arc::new(Mutex::new(db));
        let router = app(db.clone(), "127.0.0.1:8080".parse().unwrap());
        let origin = "http://127.0.0.1:8080";
        let good = router
            .clone()
            .oneshot(request(
                "PATCH",
                "/api/reader/settings",
                r#"{"display_timezone":"Asia/Tokyo","default_view":"unread"}"#,
                origin,
            ))
            .await
            .unwrap();
        assert_eq!(good.status(), StatusCode::OK);
        let invalid = router
            .clone()
            .oneshot(request(
                "PATCH",
                "/api/reader/settings",
                r#"{"default_view":"wrong"}"#,
                origin,
            ))
            .await
            .unwrap();
        assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
        let list = router
            .clone()
            .oneshot(request(
                "GET",
                "/api/articles?published_from=2026-01-02&published_to=2026-01-02",
                "",
                origin,
            ))
            .await
            .unwrap();
        let body = to_bytes(list.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["total"], 2);
        let bulk = router
            .clone()
            .oneshot(request(
                "POST",
                "/api/articles/read-all",
                r#"{"filters":{"published_from":"2026-01-02","published_to":"2026-01-02"}}"#,
                origin,
            ))
            .await
            .unwrap();
        assert_eq!(bulk.status(), StatusCode::OK);
        let states: Vec<i64> = db
            .lock()
            .unwrap()
            .prepare("SELECT is_read FROM articles ORDER BY id")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(states, vec![0, 1, 1, 0]);
        let payload = r#"{"presentation":{"appearance":"system","width":320,"layout":"tiles","collapsed_folders":[1]}}"#;
        let forbidden = router
            .clone()
            .oneshot(request(
                "POST",
                "/api/configuration/export",
                payload,
                "http://evil",
            ))
            .await
            .unwrap();
        assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);
        let export = router
            .clone()
            .oneshot(request(
                "POST",
                "/api/configuration/export",
                payload,
                origin,
            ))
            .await
            .unwrap();
        assert_eq!(export.status(), StatusCode::OK);
        assert_eq!(export.headers()[header::CACHE_CONTROL], "no-store");
        let body = to_bytes(export.into_body(), usize::MAX).await.unwrap();
        let text = std::str::from_utf8(&body).unwrap();
        assert!(!text.contains("canary"));
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["omissions"].as_array().unwrap().len(), 2);
        assert_eq!(json["format_version"], 1);
        assert!(json.get("version").is_none());
        let invalid=router.oneshot(request("POST","/api/configuration/export",r#"{"presentation":{"appearance":"system","width":320,"layout":"tiles","collapsed_folders":[1,1]}}"#,origin)).await.unwrap();
        assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
    }
    #[test]
    fn v5_migration_backup_and_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("db");
        let db = open_db(&path, false).unwrap();
        db.execute_batch("INSERT INTO folders(id,name) VALUES(4,'folder'),(5,'other'); UPDATE folder_order SET position=CASE folder_id WHEN 4 THEN 1 ELSE 0 END; INSERT INTO feeds(id,url,title,folder_id) VALUES(7,'https://example.com','feed',4),(8,'https://example.org','other',4); UPDATE feed_order SET position=CASE feed_id WHEN 7 THEN 1 ELSE 0 END; INSERT INTO articles(id,title,content,url,published,is_read,is_starred,feed_id) VALUES(9,'title','body','https://example.com/a','2026-01-01',1,1,7); INSERT INTO article_visits VALUES(9,'2026-01-02'); UPDATE collection_settings SET enabled=1; DROP TABLE feed_poll_intervals; DROP TABLE api_keys; DROP TABLE api_rate_settings; DROP TABLE reader_settings; PRAGMA user_version=5;").unwrap();
        drop(db);
        let db = open_db(&path, false).unwrap();
        assert_eq!(read(&db).unwrap().default_view, "all");
        assert!(lookup(&db, 9).unwrap().is_starred);
        assert_eq!(
            db.query_row("SELECT enabled FROM collection_settings", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
        drop(db);
        let backup = Connection::open(path.with_extension("v5-backup.sqlite3")).unwrap();
        assert_eq!(
            backup
                .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
                .unwrap(),
            5
        );
        assert_eq!(
            backup
                .query_row(
                    "SELECT visited_at FROM article_visits WHERE article_id=9",
                    [],
                    |r| r.get::<_, String>(0)
                )
                .unwrap(),
            "2026-01-02"
        );
        drop(backup);
        let db = open_db(&path, false).unwrap();
        let article = lookup(&db, 9).unwrap();
        assert!(article.is_read && article.is_starred);
        assert_eq!(article.last_visited.as_deref(), Some("2026-01-02"));
        for (table, id) in [("folder_order", "folder_id"), ("feed_order", "feed_id")] {
            let order: Vec<i64> = db
                .prepare(&format!("SELECT {id} FROM {table} ORDER BY position"))
                .unwrap()
                .query_map([], |r| r.get(0))
                .unwrap()
                .collect::<Result<_, _>>()
                .unwrap();
            assert_eq!(
                order,
                if table == "folder_order" {
                    vec![5, 4]
                } else {
                    vec![8, 7]
                }
            );
        }
        drop(db);
        let old = Connection::open(&path).unwrap();
        old.execute("DROP TABLE reader_settings", []).unwrap();
        old.execute_batch(
            "DROP TABLE feed_poll_intervals; DROP TABLE api_keys; DROP TABLE api_rate_settings;",
        )
        .unwrap();
        old.pragma_update(None, "user_version", 5).unwrap();
        drop(old);
        assert!(open_db(&path, false).is_err());
        let still_old = Connection::open(&path).unwrap();
        assert_eq!(
            still_old
                .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
                .unwrap(),
            5
        );
    }
    #[tokio::test]
    async fn invalid_requests_preserve_state() {
        let dir = tempfile::tempdir().unwrap();
        let db = Arc::new(Mutex::new(open_db(&dir.path().join("db"), true).unwrap()));
        let router = app(db.clone(), "127.0.0.1:8080".parse().unwrap());
        let origin = "http://127.0.0.1:8080";
        for body in [
            r#"{}"#,
            r#"{"default_view":null}"#,
            r#"{"default_view":12}"#,
            r#"{"display_timezone":""}"#,
            r#"{"default_view":"all","other":1}"#,
        ] {
            let response = router
                .clone()
                .oneshot(request("PATCH", "/api/reader/settings", body, origin))
                .await
                .unwrap();
            assert!(response.status().is_client_error(), "accepted {body}");
        }
        let response = router
            .clone()
            .oneshot(request(
                "PATCH",
                "/api/reader/settings",
                r#"{"default_view":"starred"}"#,
                "http://evil",
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let mut cases = vec![
            serde_json::json!({"presentation":{"appearance":"blue","width":320,"layout":"line","collapsed_folders":[]}}),
            serde_json::json!({"presentation":{"appearance":"system","width":199,"layout":"line","collapsed_folders":[]}}),
            serde_json::json!({"presentation":{"appearance":"system","width":481,"layout":"line","collapsed_folders":[]}}),
            serde_json::json!({"presentation":{"appearance":"system","width":320.5,"layout":"line","collapsed_folders":[]}}),
            serde_json::json!({"presentation":{"appearance":"system","width":320,"layout":"grid","collapsed_folders":[]}}),
            serde_json::json!({"presentation":{"appearance":"system","width":320,"layout":"line","collapsed_folders":[1,1]}}),
            serde_json::json!({"presentation":{"appearance":"system","width":320,"layout":"line","collapsed_folders":[0]}}),
            serde_json::json!({"presentation":{"appearance":"system","width":320,"layout":"line","collapsed_folders":[],"extra":true}}),
            serde_json::json!({"presentation":{"appearance":"system","width":320,"layout":"line","collapsed_folders":[]},"extra":true}),
        ];
        cases.push(serde_json::json!({"presentation":{"appearance":"system","width":320,"layout":"line","collapsed_folders":(1..=2001).collect::<Vec<_>>()}}));
        for body in cases {
            let response = router
                .clone()
                .oneshot(request(
                    "POST",
                    "/api/configuration/export",
                    &body.to_string(),
                    origin,
                ))
                .await
                .unwrap();
            assert!(response.status().is_client_error(), "accepted {body}");
            assert!(
                response
                    .headers()
                    .get(header::CONTENT_DISPOSITION)
                    .is_none()
            );
        }
        let db = db.lock().unwrap();
        assert_eq!(read(&db).unwrap().default_view, "all");
        assert_eq!(read(&db).unwrap().display_timezone, "UTC");
        assert_eq!(
            db.query_row("SELECT sum(is_read + is_starred) FROM articles", [], |r| {
                r.get::<_, i64>(0)
            })
            .unwrap(),
            0
        );
        assert_eq!(
            db.query_row("SELECT enabled FROM collection_settings", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
    #[tokio::test]
    async fn timezone_boundaries_and_selected_bulk() {
        let dir = tempfile::tempdir().unwrap();
        let db = open_db(&dir.path().join("db"), false).unwrap();
        db.execute_batch("INSERT INTO articles(id,title,content,url,published) VALUES (1,'a','','','2026-01-01T14:59:59Z'),(2,'b','','','2026-01-01T15:00:00Z'),(3,'c','','','2026-01-02T23:59:59+09:00'),(4,'d','','','2026-01-02T15:00:00Z'),(5,'e','','',''),(6,'f','','','2026-01-02T20:00:00Z');").unwrap();
        let db = Arc::new(Mutex::new(db));
        let router = app(db.clone(), "127.0.0.1:8080".parse().unwrap());
        let origin = "http://127.0.0.1:8080";
        let query = "/api/articles?published_from=2026-01-02&published_to=2026-01-02";
        let response = router
            .clone()
            .oneshot(request("GET", query, "", origin))
            .await
            .unwrap();
        let value: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        let ids = |v: &serde_json::Value| {
            let mut ids: Vec<i64> = v["items"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v["id"].as_i64().unwrap())
                .collect();
            ids.sort();
            ids
        };
        assert_eq!(ids(&value), vec![3, 4, 6]);
        router
            .clone()
            .oneshot(request(
                "PATCH",
                "/api/reader/settings",
                r#"{"display_timezone":"Asia/Tokyo"}"#,
                origin,
            ))
            .await
            .unwrap();
        let response = router
            .clone()
            .oneshot(request("GET", query, "", origin))
            .await
            .unwrap();
        let value: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(ids(&value), vec![2, 3]);
        let bad=router.clone().oneshot(request("POST","/api/articles/bulk-state",r#"{"ids":[2,4],"starred":true,"filters":{"published_from":"2026-01-02","published_to":"2026-01-02"}}"#,origin)).await.unwrap();
        assert_eq!(bad.status(), StatusCode::BAD_REQUEST);
        let good=router.oneshot(request("POST","/api/articles/bulk-state",r#"{"ids":[2,3],"starred":true,"filters":{"published_from":"2026-01-02","published_to":"2026-01-02"}}"#,origin)).await.unwrap();
        assert_eq!(good.status(), StatusCode::OK);
        let db = db.lock().unwrap();
        let states: Vec<i64> = db
            .prepare("SELECT is_starred FROM articles ORDER BY id")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(states, vec![0, 1, 1, 0, 0, 0]);
    }
    #[test]
    fn credential_urls_in_text_are_omitted() {
        let mut omissions = Vec::new();
        for value in [
            "see https://user:canary@example.com/feed",
            "https://example.com/path?x=canary",
            "prefix https://example.com/#canary",
            "https://example.com/%2574oken/canary",
        ] {
            assert!(
                clean_text(value.to_owned(), "field".into(), &mut omissions).is_none(),
                "leaked {value}"
            );
        }
        assert!(
            clean_url(
                "https://example.com/%2574oken/canary".into(),
                "url".into(),
                &mut omissions
            )
            .is_none()
        );
        assert_eq!(
            clean_text(
                "see https://example.com/feed".into(),
                "safe".into(),
                &mut omissions
            )
            .as_deref(),
            Some("see https://example.com/feed")
        );
    }
}
