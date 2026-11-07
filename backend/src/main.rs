mod auth;
mod collection;
mod collection_health;
mod digest;
mod discovery;
mod duplicates;
mod extensions;
mod hero;
mod mute_rules;
mod opml;
mod public_api;
mod read_later;
mod reader;
mod settings;
mod smart_folders;
mod social;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, Uri, header},
    response::IntoResponse,
    routing::{get, post},
};
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::{
    net::SocketAddr,
    path::{Path as FsPath, PathBuf},
    sync::{Arc, Mutex},
};
use tower_http::services::ServeDir;

const APP_ID: i64 = 0x46454544;
const ARTICLE_SOURCE_FIELDS: &str = "feed_id AS publisher_feed_id, (SELECT title FROM feeds WHERE feeds.id=articles.feed_id) AS publisher_feed_title, EXISTS(SELECT 1 FROM article_read_later WHERE article_id=articles.id) AS saved_for_later";
const SCHEMA: &str = "CREATE TABLE articles (id INTEGER PRIMARY KEY, title TEXT NOT NULL, content TEXT NOT NULL, url TEXT NOT NULL, published TEXT NOT NULL, is_read INTEGER NOT NULL DEFAULT 0 CHECK(is_read IN (0,1)), is_starred INTEGER NOT NULL DEFAULT 0 CHECK(is_starred IN (0,1)))";
type Db = Arc<Mutex<Connection>>;

#[derive(Debug, Serialize)]
struct Article {
    id: i64,
    title: String,
    content: String,
    url: String,
    published: String,
    is_read: bool,
    is_starred: bool,
    last_visited: Option<String>,
    image_url: Option<String>,
    feed_id: Option<i64>,
    feed_title: Option<String>,
    is_saved_for_later: bool,
}
fn article(row: &rusqlite::Row<'_>) -> rusqlite::Result<Article> {
    Ok(Article {
        id: row.get(0)?,
        title: row.get(1)?,
        content: row.get(2)?,
        url: row.get(3)?,
        published: row.get(4)?,
        is_read: row.get(5)?,
        is_starred: row.get(6)?,
        last_visited: row.get(7)?,
        image_url: hero::candidate(row.get(0)?, &row.get::<_, String>(3)?),
        feed_id: row.get("publisher_feed_id")?,
        feed_title: row.get("publisher_feed_title")?,
        is_saved_for_later: row.get("saved_for_later")?,
    })
}

fn open_db(path: &FsPath, demo: bool) -> Result<Connection, Box<dyn std::error::Error>> {
    // Inspect existing files read-only: even journal creation must not touch an unsupported database.
    if path.exists() {
        let check = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        check.busy_timeout(std::time::Duration::from_secs(5))?;
        let objects: Vec<(String, String, Option<String>)> = check.prepare("SELECT type, name, sql FROM sqlite_master WHERE name NOT LIKE 'sqlite_%' ORDER BY name")?.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?.collect::<Result<_, _>>()?;
        let app_id: i64 = check.pragma_query_value(None, "application_id", |r| r.get(0))?;
        let version: i64 = check.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if !objects.is_empty()
            && !(app_id == APP_ID
                && ((version == 9 && extensions::recognized_v9(&objects))
                    || (version == 8 && collection::recognized_v8(&objects))
                    || (version == 7 && auth::recognized_v7(&objects))
                    || (version == 6 && settings::recognized_v6(&objects))
                    || (version == 5 && collection::recognized_v5(&objects))
                    || (version == 4 && reader::recognized_v4(&objects))
                    || (version == 3 && collection::recognized_v3(&objects))
                    || (version == 2 && collection::recognized_v2(&objects))
                    || (version == 1
                        && objects.len() == 1
                        && objects[0].0 == "table"
                        && objects[0].1 == "articles"
                        && objects[0].2.as_deref() == Some(SCHEMA))))
        {
            return Err("unsupported database schema; refusing to modify it".into());
        }
        if objects.is_empty() && (app_id != 0 || version != 0) {
            return Err("unsupported database metadata; refusing to modify it".into());
        }
    }
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    let mut db = Connection::open(path)?;
    db.pragma_update(None, "foreign_keys", true)?;
    db.busy_timeout(std::time::Duration::from_secs(5))?;
    let version: i64 = db.pragma_query_value(None, "user_version", |r| r.get(0))?;
    if (1..=8).contains(&version) {
        let backup_path = path.with_extension(format!("v{version}-backup.sqlite3"));
        if backup_path.exists() {
            return Err("database backup already exists; refusing to overwrite it".into());
        }
        let mut backup = Connection::open(&backup_path)?;
        backup.busy_timeout(std::time::Duration::from_secs(5))?;
        let copy = rusqlite::backup::Backup::new(&db, &mut backup)?;
        copy.run_to_completion(100, std::time::Duration::from_millis(10), None)?;
    }
    let tx = db.transaction()?;
    tx.execute(
        SCHEMA
            .replace("CREATE TABLE", "CREATE TABLE IF NOT EXISTS")
            .as_str(),
        [],
    )?;
    tx.pragma_update(None, "application_id", APP_ID)?;
    collection::migrate(&tx)?;
    if version < 4 {
        tx.execute(reader::VISITS, [])?;
    }
    if version < 5 {
        collection::migrate_order(&tx)?;
    }
    if version < 6 {
        settings::migrate(&tx)?;
    }
    if version < 7 {
        auth::migrate(&tx)?;
    }
    if version < 8 {
        collection::migrate_intervals(&tx)?;
    }
    if version < 9 {
        extensions::migrate(&tx)?;
    }
    tx.pragma_update(None, "user_version", 9)?;
    if demo {
        let count: i64 = tx.query_row("SELECT COUNT(*) FROM articles", [], |r| r.get(0))?;
        if count == 0 {
            for i in 1..=36 {
                tx.execute("INSERT INTO articles(title, content, url, published) VALUES (?1, ?2, ?3, ?4)", params![format!("サンプル記事 {i}"), format!("これは Feeder の表示を確認するためのサンプル記事です。\n\n記事 {i} の本文です。"), format!("https://example.com/articles/{i}"), format!("2026-10-{:02}T09:00:00Z", (i - 1) % 28 + 1)])?;
            }
        }
    }
    tx.commit()?;
    Ok(db)
}

fn lookup(db: &Connection, id: i64) -> Result<Article, StatusCode> {
    db.query_row(
        &format!("SELECT id,title,content,url,published,is_read,is_starred,(SELECT visited_at FROM article_visits WHERE article_id=articles.id),{ARTICLE_SOURCE_FIELDS} FROM articles WHERE id=?1"),
        [id],
        article,
    )
    .optional()
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)
}
async fn detail(State(db): State<Db>, Path(id): Path<i64>) -> Result<Json<Article>, StatusCode> {
    let db = db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(lookup(&db, id)?))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Update {
    read: Option<bool>,
    starred: Option<bool>,
}
fn origin_allowed(headers: &HeaderMap) -> bool {
    let Some(origin) = headers.get(header::ORIGIN) else {
        return true;
    };
    let Ok(origin) = origin.to_str() else {
        return false;
    };
    let Ok(uri) = origin.parse::<Uri>() else {
        return false;
    };
    let Some(host) = headers.get(header::HOST).and_then(|v| v.to_str().ok()) else {
        return false;
    };
    uri.scheme_str() == Some("http")
        && uri
            .authority()
            .is_some_and(|a| a.as_str().eq_ignore_ascii_case(host))
        && uri.path() == "/"
        && uri.query().is_none()
}
async fn update(
    State(db): State<Db>,
    Path(id): Path<i64>,
    headers: HeaderMap,
    Json(input): Json<Update>,
) -> Result<Json<Article>, StatusCode> {
    if !origin_allowed(&headers) {
        return Err(StatusCode::FORBIDDEN);
    }
    if input.read.is_none() && input.starred.is_none() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let db = db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let changed = db.execute("UPDATE articles SET is_read=COALESCE(?1,is_read), is_starred=COALESCE(?2,is_starred) WHERE id=?3", params![input.read,input.starred,id]).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if changed == 0 {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(Json(lookup(&db, id)?))
}
fn parse_allowed_hosts(value: &str) -> Result<Vec<String>, &'static str> {
    value
        .split(',')
        .map(|entry| {
            let entry = entry.trim();
            let authority = entry
                .parse::<http::uri::Authority>()
                .map_err(|_| "invalid FEEDER_ALLOWED_HOSTS authority")?;
            let host = authority.host().trim_matches(['[', ']']);
            if entry.is_empty()
                || entry.contains(['@', '*', '/', '?', '#'])
                || (host.parse::<std::net::IpAddr>().is_err() && url::Host::parse(host).is_err())
                || (entry[authority.host().len()..].starts_with(':')
                    && authority.port_u16().is_none())
            {
                return Err("invalid FEEDER_ALLOWED_HOSTS authority");
            }
            Ok(entry.to_ascii_lowercase())
        })
        .collect()
}
fn valid_host(headers: &HeaderMap, bind: SocketAddr, allowed: Option<&[String]>) -> bool {
    let Some(host) = headers.get(header::HOST).and_then(|v| v.to_str().ok()) else {
        return false;
    };
    let Ok(authority) = host.parse::<http::uri::Authority>() else {
        return false;
    };
    if let Some(allowed) = allowed {
        return allowed.iter().any(|a| a.eq_ignore_ascii_case(host));
    }
    let hostname = authority.host().trim_matches(['[', ']']);
    let matches_host = hostname
        .parse::<std::net::IpAddr>()
        .is_ok_and(|ip| ip == bind.ip() || (ip.is_loopback() && bind.ip().is_loopback()))
        || (hostname.eq_ignore_ascii_case("localhost") && bind.ip().is_loopback());
    matches_host && authority.port_u16().unwrap_or(80) == bind.port()
}
#[cfg(test)]
fn app(db: Db, bind: SocketAddr) -> Router {
    app_with_hosts(db, bind, None)
}
#[cfg(test)]
fn app_with_hosts(db: Db, bind: SocketAddr, allowed: Option<Vec<String>>) -> Router {
    base_router(db).layer(axum::middleware::from_fn(
        move |request: axum::extract::Request, next: axum::middleware::Next| {
            let allowed = allowed.clone();
            async move {
                if !valid_host(request.headers(), bind, allowed.as_deref()) {
                    return StatusCode::FORBIDDEN.into_response();
                }
                next.run(request).await
            }
        },
    ))
}
fn base_router(db: Db) -> Router {
    Router::new()
        .merge(public_api::routes())
        .merge(collection::routes())
        .merge(discovery::routes())
        .merge(opml::routes())
        .merge(settings::routes())
        .merge(smart_folders::routes())
        .merge(social::routes())
        .merge(read_later::routes())
        .merge(mute_rules::routes())
        .merge(collection_health::routes())
        .merge(digest::routes())
        .merge(duplicates::routes())
        .route("/healthz", get(|| async { "ok" }))
        .route("/api/articles", get(reader::list))
        .route("/api/articles/read-all", post(reader::read_all))
        .route("/api/articles/bulk-state", post(reader::bulk_state))
        .route("/api/articles/{id}/visit", post(reader::visit))
        .route("/api/articles/{id}/hero", get(hero::get))
        .route("/api/articles/{id}", get(detail))
        .route("/api/articles/{id}/state", post(update))
        .fallback_service(ServeDir::new("frontend/dist").append_index_html_on_directories(true))
        .with_state(db)
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let worker = match args.as_slice() {
        [] => None,
        [cmd] if cmd == "worker" => Some(false),
        [cmd, option] if cmd == "worker" && option == "--once" => Some(true),
        _ => return Err("usage: feeder [worker [--once]]".into()),
    };
    let access = if worker.is_none() {
        Some(auth::Access::from_env()?)
    } else {
        None
    };
    let allowed = std::env::var("FEEDER_ALLOWED_HOSTS")
        .ok()
        .map(|v| parse_allowed_hosts(&v))
        .transpose()?;
    let path =
        PathBuf::from(std::env::var("FEEDER_DB").unwrap_or_else(|_| "data/feeder.sqlite3".into()));
    let bind: SocketAddr = std::env::var("FEEDER_BIND")
        .unwrap_or_else(|_| "127.0.0.1:8080".into())
        .parse()?;
    let demo = matches!(std::env::var("FEEDER_DEMO").as_deref(), Ok("1" | "true"));
    let db = Arc::new(Mutex::new(open_db(&path, demo)?));
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    tokio::spawn(async move {
        shutdown_signal().await;
        let _ = shutdown_tx.send(true);
    });
    if let Some(once) = worker {
        return collection::worker(db, once, shutdown_rx).await;
    }
    let listener = tokio::net::TcpListener::bind(bind).await?;
    eprintln!("Feeder listening on http://{}", listener.local_addr()?);
    let actual_bind = listener.local_addr()?;
    let access = access.expect("web access configuration");
    let router = base_router(db.clone()).merge(auth::routes(access.clone()).with_state(db.clone()));
    axum::serve(
        listener,
        auth::wrap(router, db, access, actual_bind, allowed),
    )
    .with_graceful_shutdown(async move {
        let mut shutdown = shutdown_rx;
        let _ = shutdown.changed().await;
    })
    .await?;
    Ok(())
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("failed to install SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {},
            _ = terminate.recv() => {},
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
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
    async fn hero_contract_and_private_target_preserve_state() {
        let dir = tempfile::tempdir().unwrap();
        let db = Arc::new(Mutex::new(open_db(&dir.path().join("db"), false).unwrap()));
        db.lock().unwrap().execute("INSERT INTO articles(id,title,content,url,published) VALUES (1,'title','body','http://127.0.0.1/private','')", []).unwrap();
        let router = app(db.clone(), "127.0.0.1:8080".parse().unwrap());
        for (method, path) in [
            ("GET", "/api/articles/1"),
            ("POST", "/api/articles/1/visit"),
        ] {
            let result = router
                .clone()
                .oneshot(request(method, path, None, ""))
                .await
                .unwrap();
            let json: serde_json::Value =
                serde_json::from_slice(&to_bytes(result.into_body(), 8192).await.unwrap()).unwrap();
            assert_eq!(json["image_url"], "/api/articles/1/hero");
        }
        let before: (bool, i64) = db
            .lock()
            .unwrap()
            .query_row(
                "SELECT is_read,(SELECT count(*) FROM article_visits) FROM articles WHERE id=1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        let result = router
            .clone()
            .oneshot(request("GET", "/api/articles/1/hero", None, ""))
            .await
            .unwrap();
        assert_eq!(result.status(), StatusCode::NOT_FOUND);
        let after: (bool, i64) = db
            .lock()
            .unwrap()
            .query_row(
                "SELECT is_read,(SELECT count(*) FROM article_visits) FROM articles WHERE id=1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(before, after);
        let result = router
            .oneshot(request("GET", "/api/articles/999/hero", None, ""))
            .await
            .unwrap();
        assert_eq!(result.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn explicit_hosts_and_origins() {
        let hosts = parse_allowed_hosts("localhost:8080,127.0.0.1:8080,web:8080").unwrap();
        for invalid in [
            "",
            "web:abc",
            "*",
            "http://web:8080",
            "user@web:8080",
            "web:8080,",
        ] {
            assert!(parse_allowed_hosts(invalid).is_err(), "{invalid}");
        }
        let dir = tempfile::tempdir().unwrap();
        let db = Arc::new(Mutex::new(
            open_db(&dir.path().join("test.db"), true).unwrap(),
        ));
        let router = app_with_hosts(db, "0.0.0.0:8080".parse().unwrap(), Some(hosts));
        let req = Request::builder()
            .uri("/healthz")
            .header("host", "web:8080")
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            router.clone().oneshot(req).await.unwrap().status(),
            StatusCode::OK
        );
        let req = Request::builder()
            .method("POST")
            .uri("/api/articles/1/state")
            .header("host", "web:8080")
            .header("origin", "http://evil:8080")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"read":true}"#))
            .unwrap();
        assert_eq!(
            router.clone().oneshot(req).await.unwrap().status(),
            StatusCode::FORBIDDEN
        );
        let req = Request::builder()
            .uri("/healthz")
            .header("host", "evil:8080")
            .header("x-forwarded-host", "web:8080")
            .header("forwarded", "host=web:8080")
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            router.oneshot(req).await.unwrap().status(),
            StatusCode::FORBIDDEN
        );
    }

    fn request(method: &str, uri: &str, origin: Option<&str>, body: &str) -> Request<Body> {
        let mut builder = Request::builder()
            .method(method)
            .uri(uri)
            .header("host", "127.0.0.1:8080");
        if let Some(origin) = origin {
            builder = builder.header("origin", origin);
        }
        if !body.is_empty() {
            builder = builder.header("content-type", "application/json");
        }
        builder.body(Body::from(body.to_owned())).unwrap()
    }
    #[tokio::test]
    async fn api_filters_security_and_persistence() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("reader.db");
        let db = Arc::new(Mutex::new(open_db(&path, true).unwrap()));
        let router = app(db.clone(), "127.0.0.1:8080".parse().unwrap());
        let response = router
            .clone()
            .oneshot(request("GET", "/api/articles?limit=2&offset=0", None, ""))
            .await
            .unwrap();
        let json: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(json["total"], 36);
        assert_eq!(json["items"].as_array().unwrap().len(), 2);
        let bad = router
            .clone()
            .oneshot(request(
                "POST",
                "/api/articles/1/state",
                Some("http://evil.example"),
                r#"{"read":true}"#,
            ))
            .await
            .unwrap();
        assert_eq!(bad.status(), StatusCode::FORBIDDEN);
        let good = router
            .clone()
            .oneshot(request(
                "POST",
                "/api/articles/1/state",
                Some("http://127.0.0.1:8080"),
                r#"{"read":true,"starred":true}"#,
            ))
            .await
            .unwrap();
        assert_eq!(good.status(), StatusCode::OK);
        let filtered = router
            .clone()
            .oneshot(request(
                "GET",
                "/api/articles?starred=true&unread=true",
                None,
                "",
            ))
            .await
            .unwrap();
        let json: serde_json::Value =
            serde_json::from_slice(&to_bytes(filtered.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(json["total"], 0);
        let bad_host = Request::builder()
            .uri("/api/articles")
            .header("host", "evil.example:8080")
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            router.oneshot(bad_host).await.unwrap().status(),
            StatusCode::FORBIDDEN
        );
        drop(db);
        let reopened = open_db(&path, false).unwrap();
        let saved = lookup(&reopened, 1).unwrap();
        assert!(saved.is_read && saved.is_starred);
    }
    #[tokio::test]
    async fn collection_crud_and_filters() {
        let dir = tempfile::tempdir().unwrap();
        let db = Arc::new(Mutex::new(open_db(&dir.path().join("db"), false).unwrap()));
        let router = app(db.clone(), "127.0.0.1:8080".parse().unwrap());
        let response = router
            .clone()
            .oneshot(request("POST", "/api/folders", None, r#"{"name":"News"}"#))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        {
            let db = db.lock().unwrap();
            db.execute("INSERT INTO feeds(id,url,title,folder_id) VALUES (1,'https://example.com/1','one',1),(2,'https://example.com/2','two',NULL)",[]).unwrap();
            db.execute("INSERT INTO articles(feed_id,external_id,title,content,url,published) VALUES (1,'a','a','','',''),(2,'b','b','','','')",[]).unwrap();
        }
        let response = router
            .clone()
            .oneshot(request("GET", "/api/articles?folder_id=1", None, ""))
            .await
            .unwrap();
        let json: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(json["total"], 1);
        let response = router
            .clone()
            .oneshot(request(
                "PATCH",
                "/api/feeds/1",
                None,
                r#"{"paused":true,"title":"renamed"}"#,
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let response = router
            .clone()
            .oneshot(request("POST", "/api/feeds/1/refresh", None, ""))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);
        let response = router
            .clone()
            .oneshot(request("DELETE", "/api/folders/1", None, ""))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert!(
            db.lock()
                .unwrap()
                .query_row("SELECT folder_id FROM feeds WHERE id=1", [], |r| r
                    .get::<_, Option<i64>>(0))
                .unwrap()
                .is_none()
        );
        let response = router
            .oneshot(request("DELETE", "/api/feeds/1", None, ""))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert_eq!(
            db.lock()
                .unwrap()
                .query_row("SELECT count(*) FROM articles", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
    }
    #[test]
    fn migrates_known_v1_and_keeps_backup() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("old.sqlite3");
        let db = Connection::open(&path).unwrap();
        db.execute(SCHEMA, []).unwrap();
        db.pragma_update(None, "application_id", APP_ID).unwrap();
        db.pragma_update(None, "user_version", 1).unwrap();
        db.execute("INSERT INTO articles(id,title,content,url,published,is_read,is_starred) VALUES (42,'title','body','url','date',1,1)", []).unwrap();
        drop(db);
        let migrated = open_db(&path, false).unwrap();
        let article = lookup(&migrated, 42).unwrap();
        assert_eq!(article.content, "body");
        assert!(article.is_read && article.is_starred);
        drop(migrated);
        let backup = Connection::open(path.with_extension("v1-backup.sqlite3")).unwrap();
        assert_eq!(
            backup
                .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert!(open_db(&path, false).is_ok());
    }
    #[test]
    fn migrates_v2_and_retains_backup() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("old.sqlite3");
        let db = open_db(&path, false).unwrap();
        extensions::drop_for_legacy_fixture(&db);
        db.execute(
            "INSERT INTO feeds(url,title) VALUES ('https://example.com/feed','Example')",
            [],
        )
        .unwrap();
        for sql in [
            "DROP TRIGGER folder_order_insert",
            "DROP TRIGGER feed_order_insert",
            "DROP TRIGGER feed_order_move",
            "DROP TABLE feed_order",
            "DROP TABLE folder_order",
        ] {
            db.execute(sql, []).unwrap();
        }
        db.execute_batch(
            "DROP TABLE feed_poll_intervals; DROP TABLE api_keys; DROP TABLE api_rate_settings;",
        )
        .unwrap();
        db.execute("DROP TABLE reader_settings", []).unwrap();
        db.execute("DROP TABLE article_visits", []).unwrap();
        db.execute("DROP TABLE refresh_leases", []).unwrap();
        db.execute("DROP TABLE collection_settings", []).unwrap();
        db.pragma_update(None, "user_version", 2).unwrap();
        drop(db);
        let db = open_db(&path, false).unwrap();
        assert_eq!(
            db.query_row("SELECT title FROM feeds", [], |r| r.get::<_, String>(0))
                .unwrap(),
            "Example"
        );
        assert_eq!(
            db.query_row(
                "SELECT enabled,interval_seconds FROM collection_settings",
                [],
                |r| Ok((r.get::<_, bool>(0)?, r.get::<_, i64>(1)?))
            )
            .unwrap(),
            (true, 900)
        );
        drop(db);
        assert!(open_db(&path, false).is_ok());
        let backup = Connection::open(path.with_extension("v2-backup.sqlite3")).unwrap();
        assert_eq!(
            backup
                .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
                .unwrap(),
            2
        );
    }
    #[test]
    fn refuses_foreign_db_without_changes_and_demo_is_opt_in() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("foreign.db");
        let conn = Connection::open(&path).unwrap();
        conn.execute("CREATE TABLE source(data TEXT)", []).unwrap();
        drop(conn);
        let before = std::fs::read(&path).unwrap();
        assert!(open_db(&path, true).is_err());
        assert_eq!(before, std::fs::read(&path).unwrap());
        let empty = open_db(&dir.path().join("empty.db"), false).unwrap();
        assert_eq!(
            empty
                .query_row("SELECT count(*) FROM articles", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
}
