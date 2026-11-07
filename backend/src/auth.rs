use super::*;
use axum::{extract::Request, middleware::Next, response::Response, routing::delete};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};
use subtle::ConstantTimeEq;

const KEYS: &str = "CREATE TABLE api_keys (id INTEGER PRIMARY KEY, name TEXT NOT NULL, prefix TEXT NOT NULL, token_hash TEXT NOT NULL UNIQUE, scope TEXT NOT NULL CHECK(scope IN ('read','write')), created_at TEXT NOT NULL DEFAULT (datetime('now')), expires_at TEXT NOT NULL, revoked_at TEXT, last_used_at TEXT, window_start INTEGER NOT NULL DEFAULT 0, window_count INTEGER NOT NULL DEFAULT 0)";
const RATE: &str = "CREATE TABLE api_rate_settings (id INTEGER PRIMARY KEY CHECK(id=1), rate_limit INTEGER NOT NULL DEFAULT 120 CHECK(rate_limit BETWEEN 1 AND 1000))";
pub fn migrate(tx: &rusqlite::Transaction<'_>) -> rusqlite::Result<()> {
    tx.execute(KEYS, [])?;
    tx.execute(RATE, [])?;
    tx.execute("INSERT INTO api_rate_settings(id) VALUES(1)", [])?;
    Ok(())
}
pub fn recognized_v7(objects: &[(String, String, Option<String>)]) -> bool {
    let mut rest = objects.to_vec();
    for (name, sql) in [("api_keys", KEYS), ("api_rate_settings", RATE)] {
        let Some(i) = rest
            .iter()
            .position(|o| o.0 == "table" && o.1 == name && o.2.as_deref() == Some(sql))
        else {
            return false;
        };
        rest.remove(i);
    }
    settings::recognized_v6(&rest)
}
fn random() -> Result<String, StatusCode> {
    let mut b = [0u8; 32];
    getrandom::getrandom(&mut b).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(b.iter().map(|x| format!("{x:02x}")).collect())
}
fn hash(s: &str) -> String {
    Sha256::digest(s.as_bytes())
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect()
}
#[derive(Clone)]
pub struct Access {
    mode: Mode,
    sessions: Arc<Mutex<HashMap<String, (String, Instant)>>>,
}
#[derive(Clone)]
enum Mode {
    Local,
    Tunnel { origin: String, authority: String },
}
impl Access {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        let mode = match std::env::var("FEEDER_ACCESS_MODE")
            .unwrap_or_else(|_| "local".into())
            .as_str()
        {
            "local" => Mode::Local,
            "tunnel" => {
                let origin = std::env::var("FEEDER_PUBLIC_ORIGIN")?;
                let u = url::Url::parse(&origin)?;
                if u.scheme() != "https"
                    || u.username() != ""
                    || u.password().is_some()
                    || u.path() != "/"
                    || u.query().is_some()
                    || u.fragment().is_some()
                    || !matches!(u.host(), Some(url::Host::Domain(_)))
                    || origin.ends_with('/')
                {
                    return Err("invalid FEEDER_PUBLIC_ORIGIN".into());
                }
                let authority = origin
                    .strip_prefix("https://")
                    .ok_or("invalid origin")?
                    .to_ascii_lowercase();
                if authority.contains(['@', '/', '?', '#']) {
                    return Err("invalid origin".into());
                }
                Mode::Tunnel { origin, authority }
            }
            _ => return Err("invalid FEEDER_ACCESS_MODE".into()),
        };
        Ok(Self {
            mode,
            sessions: Arc::new(Mutex::new(HashMap::new())),
        })
    }
    fn host_ok(&self, headers: &HeaderMap, bind: SocketAddr, allowed: Option<&[String]>) -> bool {
        let Some(host) = headers.get(header::HOST).and_then(|v| v.to_str().ok()) else {
            return false;
        };
        match &self.mode {
            Mode::Tunnel { authority, .. } => host.eq_ignore_ascii_case(authority),
            Mode::Local => {
                let Ok(a) = host.parse::<http::uri::Authority>() else {
                    return false;
                };
                let h = a.host().trim_matches(['[', ']']);
                (h.eq_ignore_ascii_case("localhost")
                    || h.eq_ignore_ascii_case("web")
                    || h.parse::<std::net::IpAddr>()
                        .is_ok_and(|ip| ip.is_loopback()))
                    && (valid_host(headers, bind, allowed)
                        || (h.eq_ignore_ascii_case("web") && allowed.is_none()))
            }
        }
    }
    fn session(&self, headers: &HeaderMap) -> Option<String> {
        let cookie = headers.get(header::COOKIE)?.to_str().ok()?;
        let mut ids = cookie
            .split(';')
            .filter_map(|s| s.trim().split_once('='))
            .filter(|(k, _)| *k == "feeder_ui")
            .map(|(_, v)| v);
        let id = ids.next()?;
        if ids.next().is_some() {
            return None;
        }
        let mut sessions = self.sessions.lock().ok()?;
        sessions.retain(|_, (_, t)| t.elapsed() < Duration::from_secs(43200));
        sessions.get(id).map(|(csrf, _)| csrf.clone())
    }
}
fn error(status: StatusCode, _code: &str, message: &str) -> Response {
    (
        status,
        Json(serde_json::json!({"error":{"code":status.as_u16(),"message":message}})),
    )
        .into_response()
}
fn auth_error(status: StatusCode, code: &str, message: &str) -> Response {
    let mut r = error(status, code, message);
    if status == StatusCode::UNAUTHORIZED {
        r.headers_mut()
            .insert(header::WWW_AUTHENTICATE, "Bearer".parse().unwrap());
    }
    r
}
#[allow(clippy::result_large_err)]
fn authenticate(db: &Db, headers: &HeaderMap, write: bool) -> Result<(), Response> {
    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split_once(' '))
        .filter(|(scheme, token)| {
            scheme.eq_ignore_ascii_case("bearer")
                && !token.is_empty()
                && token.len() <= 256
                && !token.chars().any(char::is_whitespace)
        })
        .map(|(_, token)| token)
        .ok_or_else(|| {
            auth_error(
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "Bearer key required",
            )
        })?;
    let mut db = db.lock().map_err(|_| {
        error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            "Database unavailable",
        )
    })?;
    let tx = db
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|_| {
            error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                "Database unavailable",
            )
        })?;
    let key: Option<(i64,String,i64,i64,i64)> = tx.query_row("SELECT id,scope,window_start,window_count,(SELECT rate_limit FROM api_rate_settings WHERE id=1) FROM api_keys WHERE token_hash=?1 AND revoked_at IS NULL AND expires_at>datetime('now')",[hash(token)],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).optional().map_err(|_|error(StatusCode::INTERNAL_SERVER_ERROR,"internal_error","Database unavailable"))?;
    let (id, scope, start, count, limit) = key.ok_or_else(|| {
        auth_error(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "Invalid or expired key",
        )
    })?;
    if write && scope != "write" {
        return Err(auth_error(
            StatusCode::FORBIDDEN,
            "forbidden",
            "Write scope required",
        ));
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    if now - start < 60 && count >= limit {
        let mut response = auth_error(
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limited",
            "Rate limit exceeded",
        );
        response.headers_mut().insert(
            header::RETRY_AFTER,
            (60 - (now - start).max(0)).to_string().parse().unwrap(),
        );
        return Err(response);
    }
    let (start, count) = if now - start >= 60 {
        (now, 1)
    } else {
        (start, count + 1)
    };
    tx.execute("UPDATE api_keys SET window_start=?1,window_count=?2,last_used_at=datetime('now') WHERE id=?3",params![start,count,id]).and_then(|_|tx.commit()).map_err(|_|error(StatusCode::INTERNAL_SERVER_ERROR,"internal_error","Database unavailable"))?;
    Ok(())
}
pub fn wrap(
    router: Router,
    db: Db,
    access: Access,
    bind: SocketAddr,
    allowed: Option<Vec<String>>,
) -> Router {
    router.layer(axum::middleware::from_fn(
        move |mut req: Request, next: Next| {
            let (db, access, allowed) = (db.clone(), access.clone(), allowed.clone());
            async move {
                let path = req.uri().path().to_owned();
                for name in [
                    header::HOST.as_str(),
                    header::ORIGIN.as_str(),
                    header::COOKIE.as_str(),
                    header::AUTHORIZATION.as_str(),
                    "x-feeder-csrf",
                    "sec-fetch-site",
                ] {
                    if req.headers().get_all(name).iter().count() > 1 {
                        return StatusCode::FORBIDDEN.into_response();
                    }
                }
                if path != "/healthz" && !access.host_ok(req.headers(), bind, allowed.as_deref()) {
                    return StatusCode::FORBIDDEN.into_response();
                }
                if path.starts_with("/api/v1/") || path == "/api/v1" {
                    let write = !matches!(
                        *req.method(),
                        http::Method::GET | http::Method::HEAD | http::Method::OPTIONS
                    );
                    if let Err(r) = authenticate(&db, req.headers(), write) {
                        return r;
                    }
                } else if path.starts_with("/api/") {
                    let unsafe_method = !matches!(
                        *req.method(),
                        http::Method::GET | http::Method::HEAD | http::Method::OPTIONS
                    );
                    let admin = path == "/api/keys" || path.starts_with("/api/keys/");
                    let cross = req
                        .headers()
                        .get("sec-fetch-site")
                        .and_then(|v| v.to_str().ok())
                        == Some("cross-site");
                    let origin_ok = match req.headers().get(header::ORIGIN) {
                        None => true,
                        Some(o) => o.to_str().ok().is_some_and(|o| match &access.mode {
                            Mode::Local => req
                                .headers()
                                .get(header::HOST)
                                .and_then(|v| v.to_str().ok())
                                .is_some_and(|h| o == format!("http://{h}")),
                            Mode::Tunnel { origin, .. } => o == origin,
                        }),
                    };
                    if cross || !origin_ok {
                        return StatusCode::FORBIDDEN.into_response();
                    }
                    let session = access.session(req.headers());
                    if (admin
                        || unsafe_method
                        || (path.starts_with("/api/articles/") && path.ends_with("/hero")))
                        && session.is_none()
                    {
                        return StatusCode::FORBIDDEN.into_response();
                    }
                    if unsafe_method
                        && !session
                            .as_ref()
                            .zip(
                                req.headers()
                                    .get("x-feeder-csrf")
                                    .and_then(|v| v.to_str().ok()),
                            )
                            .is_some_and(|(a, b)| bool::from(a.as_bytes().ct_eq(b.as_bytes())))
                    {
                        return StatusCode::FORBIDDEN.into_response();
                    }
                    if matches!(access.mode, Mode::Tunnel { .. }) {
                        req.headers_mut().remove(header::ORIGIN);
                    }
                }
                next.run(req).await
            }
        },
    ))
}
pub fn routes(access: Access) -> Router<Db> {
    let a = access.clone();
    Router::new()
        .route(
            "/api/session",
            get(move |headers: HeaderMap| {
                let a = a.clone();
                async move {
                    let csrf = if let Some(csrf) = a.session(&headers) {
                        return (
                            [(header::CACHE_CONTROL, "no-store")],
                            Json(serde_json::json!({"csrf_token":csrf})),
                        )
                            .into_response();
                    } else {
                        match random() {
                            Ok(v) => v,
                            Err(s) => return s.into_response(),
                        }
                    };
                    let id = match random() {
                        Ok(v) => v,
                        Err(s) => return s.into_response(),
                    };
                    let mut sessions = match a.sessions.lock() {
                        Ok(v) => v,
                        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
                    };
                    sessions.retain(|_, (_, t)| t.elapsed() < Duration::from_secs(43200));
                    if sessions.len() >= 1024
                        && let Some(old) = sessions
                            .iter()
                            .min_by_key(|(_, (_, t))| *t)
                            .map(|(k, _)| k.clone())
                    {
                        sessions.remove(&old);
                    }
                    sessions.insert(id.clone(), (csrf.clone(), Instant::now()));
                    let cookie = format!(
                        "feeder_ui={id}; HttpOnly; SameSite=Strict; Path=/; Max-Age=43200{}",
                        if matches!(a.mode, Mode::Tunnel { .. }) {
                            "; Secure"
                        } else {
                            ""
                        }
                    );
                    let mut r = Json(serde_json::json!({"csrf_token":csrf})).into_response();
                    r.headers_mut()
                        .insert(header::SET_COOKIE, cookie.parse().unwrap());
                    r.headers_mut()
                        .insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
                    r
                }
            }),
        )
        .route("/api/keys", get(list).post(issue))
        .route("/api/keys/settings", get(get_rate).patch(set_rate))
        .route("/api/keys/{id}", delete(revoke))
}
async fn list(State(db): State<Db>) -> Result<Json<serde_json::Value>, StatusCode> {
    let db = db.lock().map_err(collection::err)?;
    let mut s=db.prepare("SELECT id,name,prefix,scope,created_at,expires_at,revoked_at,last_used_at FROM api_keys ORDER BY id DESC").map_err(collection::err)?;
    let rows=s.query_map([],|r|Ok(serde_json::json!({"id":r.get::<_,i64>(0)?,"name":r.get::<_,String>(1)?,"prefix":r.get::<_,String>(2)?,"scope":r.get::<_,String>(3)?,"created_at":r.get::<_,String>(4)?,"expires_at":r.get::<_,String>(5)?,"revoked_at":r.get::<_,Option<String>>(6)?,"last_used_at":r.get::<_,Option<String>>(7)?}))).map_err(collection::err)?.collect::<Result<Vec<_>,_>>().map_err(collection::err)?;
    Ok(Json(serde_json::json!({"keys":rows})))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Issue {
    name: String,
    scope: String,
    days: i64,
}
async fn issue(State(db): State<Db>, Json(input): Json<Issue>) -> Result<Response, StatusCode> {
    let name = input.name.trim();
    if name.is_empty()
        || name.chars().count() > 80
        || !matches!(input.scope.as_str(), "read" | "write")
        || !(1..=365).contains(&input.days)
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    let token = random()?;
    let prefix = token[..8].to_owned();
    let db = db.lock().map_err(collection::err)?;
    db.execute("INSERT INTO api_keys(name,prefix,token_hash,scope,expires_at) VALUES(?1,?2,?3,?4,datetime('now',?5))",params![name,prefix,hash(&token),input.scope,format!("+{} days",input.days)]).map_err(collection::err)?;
    let mut r =
        Json(serde_json::json!({"id":db.last_insert_rowid(),"token":token})).into_response();
    r.headers_mut()
        .insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
    Ok(r)
}
async fn revoke(State(db): State<Db>, Path(id): Path<i64>) -> Result<StatusCode, StatusCode> {
    let db = db.lock().map_err(collection::err)?;
    let n = db
        .execute(
            "UPDATE api_keys SET revoked_at=datetime('now') WHERE id=?1 AND revoked_at IS NULL",
            [id],
        )
        .map_err(collection::err)?;
    if n == 0 {
        Err(StatusCode::NOT_FOUND)
    } else {
        Ok(StatusCode::NO_CONTENT)
    }
}
async fn get_rate(State(db): State<Db>) -> Result<Json<serde_json::Value>, StatusCode> {
    let db = db.lock().map_err(collection::err)?;
    let n: i64 = db
        .query_row(
            "SELECT rate_limit FROM api_rate_settings WHERE id=1",
            [],
            |r| r.get(0),
        )
        .map_err(collection::err)?;
    Ok(Json(serde_json::json!({"rate_limit":n})))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Rate {
    rate_limit: i64,
}
async fn set_rate(
    State(db): State<Db>,
    Json(v): Json<Rate>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if !(1..=1000).contains(&v.rate_limit) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let db = db.lock().map_err(collection::err)?;
    db.execute(
        "UPDATE api_rate_settings SET rate_limit=?1 WHERE id=1",
        [v.rate_limit],
    )
    .map_err(collection::err)?;
    Ok(Json(serde_json::json!({"rate_limit":v.rate_limit})))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{Body, to_bytes};
    use tower::ServiceExt;
    fn request(
        method: &str,
        path: &str,
        host: &str,
        cookie: Option<&str>,
        csrf: Option<&str>,
        origin: Option<&str>,
        body: &str,
    ) -> Request {
        let mut b = Request::builder()
            .method(method)
            .uri(path)
            .header("host", host)
            .header("content-type", "application/json");
        if let Some(x) = cookie {
            b = b.header("cookie", x)
        }
        if let Some(x) = csrf {
            b = b.header("x-feeder-csrf", x)
        }
        if let Some(x) = origin {
            b = b.header("origin", x)
        }
        b.body(Body::from(body.to_string())).unwrap()
    }
    #[tokio::test]
    async fn production_subscription_url_edits_are_atomic_and_lease_aware() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("db");
        let db = Arc::new(Mutex::new(open_db(&path, false).unwrap()));
        {
            let c = db.lock().unwrap();
            c.execute_batch("INSERT INTO feeds(id,url,title,etag,modified,last_checked,last_error) VALUES (1,'https://example.org/original','Original','old-etag','old-date','2026-01-01','old error'),(2,'https://example.org/other#fragment','Other',NULL,NULL,NULL,NULL); INSERT INTO articles(id,feed_id,external_id,title,content,url,published,is_read,is_starred) VALUES(1,1,'guid','Saved','body','https://example.org/article','2026-01-01',1,1); INSERT INTO article_visits VALUES(1,'2026-01-02');").unwrap();
            for (token, scope) in [("write-secret", "write"), ("read-secret", "read")] {
                c.execute("INSERT INTO api_keys(name,prefix,token_hash,scope,expires_at) VALUES('test','prefix',?,?,datetime('now','+1 day'))", params![hash(token),scope]).unwrap();
            }
        }
        let access = Access {
            mode: Mode::Local,
            sessions: Arc::new(Mutex::new(HashMap::new())),
        };
        let router = wrap(
            base_router(db.clone()).merge(routes(access.clone()).with_state(db.clone())),
            db.clone(),
            access,
            "127.0.0.1:8080".parse().unwrap(),
            None,
        );
        let session = router
            .clone()
            .oneshot(request(
                "GET",
                "/api/session",
                "127.0.0.1:8080",
                None,
                None,
                None,
                "",
            ))
            .await
            .unwrap();
        let cookie = session.headers()[header::SET_COOKIE]
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_owned();
        let csrf = serde_json::from_slice::<serde_json::Value>(
            &to_bytes(session.into_body(), 4096).await.unwrap(),
        )
        .unwrap()["csrf_token"]
            .as_str()
            .unwrap()
            .to_owned();
        let browser = |method: &str, path: &str, body: &str| {
            request(
                method,
                path,
                "127.0.0.1:8080",
                Some(&cookie),
                Some(&csrf),
                None,
                body,
            )
        };
        let api = |method: &str, path: &str, body: &str, token: &str| {
            let mut req = request(method, path, "127.0.0.1:8080", None, None, None, body);
            req.headers_mut().insert(
                header::AUTHORIZATION,
                format!("Bearer {token}").parse().unwrap(),
            );
            req
        };
        for url in [
            "http://127.0.0.1/feed",
            "http://private.local/feed",
            "http://localhost./feed",
            "http://private.local./feed",
            "https://user:pass@example.org/feed",
            "file:///tmp/feed",
        ] {
            let body =
                serde_json::json!({"url":url,"folder":"Must not be created","title":"Changed"})
                    .to_string();
            assert_eq!(
                router
                    .clone()
                    .oneshot(api("PATCH", "/api/v1/feeds/1", &body, "write-secret"))
                    .await
                    .unwrap()
                    .status(),
                StatusCode::BAD_REQUEST
            );
        }
        let body = r#"{"url":"https://EXAMPLE.org/other#new","folder":"Must not be created","title":"Changed"}"#;
        assert_eq!(
            router
                .clone()
                .oneshot(api("PATCH", "/api/v1/feeds/1", body, "write-secret"))
                .await
                .unwrap()
                .status(),
            StatusCode::CONFLICT
        );
        assert_eq!(
            router
                .clone()
                .oneshot(api(
                    "PATCH",
                    "/api/v1/feeds/1",
                    r#"{"url":"https://example.org/new"}"#,
                    "read-secret"
                ))
                .await
                .unwrap()
                .status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            router
                .clone()
                .oneshot(request(
                    "PATCH",
                    "/api/feeds/1",
                    "127.0.0.1:8080",
                    None,
                    None,
                    None,
                    r#"{"url":"https://example.org/new"}"#
                ))
                .await
                .unwrap()
                .status(),
            StatusCode::FORBIDDEN
        );
        for value in [
            serde_json::json!(59),
            serde_json::json!(86401),
            serde_json::json!(1.5),
            serde_json::json!(true),
            serde_json::json!("300"),
        ] {
            let body = serde_json::json!({"poll_interval":value,"url":"https://example.org/new","folder":"Must not be created"})
                .to_string();
            assert_eq!(
                router
                    .clone()
                    .oneshot(api("PATCH", "/api/v1/feeds/1", &body, "write-secret"))
                    .await
                    .unwrap()
                    .status(),
                StatusCode::BAD_REQUEST
            );
        }
        assert_eq!(
            db.lock()
                .unwrap()
                .query_row("SELECT url FROM feeds WHERE id=1", [], |r| r
                    .get::<_, String>(0))
                .unwrap(),
            "https://example.org/original"
        );
        assert_eq!(
            router
                .clone()
                .oneshot(api(
                    "PATCH",
                    "/api/v1/feeds/1",
                    r#"{"poll_interval":60}"#,
                    "read-secret"
                ))
                .await
                .unwrap()
                .status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            router
                .clone()
                .oneshot(api(
                    "PATCH",
                    "/api/v1/feeds/1",
                    r#"{"poll_interval":60}"#,
                    "write-secret"
                ))
                .await
                .unwrap()
                .status(),
            StatusCode::OK
        );
        let response = router
            .clone()
            .oneshot(api("GET", "/api/v1/feeds/1", "", "read-secret"))
            .await
            .unwrap();
        let value: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 4096).await.unwrap()).unwrap();
        assert_eq!(value["data"]["poll_interval"], 60);
        assert_eq!(
            router
                .clone()
                .oneshot(browser(
                    "PATCH",
                    "/api/feeds/1",
                    r#"{"poll_interval":86400}"#
                ))
                .await
                .unwrap()
                .status(),
            StatusCode::OK
        );
        assert_eq!(
            db.lock()
                .unwrap()
                .query_row(
                    "SELECT interval_seconds FROM feed_poll_intervals WHERE feed_id=1",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            86400
        );
        assert_eq!(
            router
                .clone()
                .oneshot(browser(
                    "PATCH",
                    "/api/feeds/1",
                    r#"{"poll_interval":null}"#
                ))
                .await
                .unwrap()
                .status(),
            StatusCode::OK
        );
        assert_eq!(
            db.lock()
                .unwrap()
                .query_row("SELECT count(*) FROM feed_poll_intervals", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
        let worker = open_db(&path, false).unwrap();
        worker
            .execute(
                "INSERT INTO refresh_leases VALUES(1,'worker',unixepoch()+60)",
                [],
            )
            .unwrap();
        for req in [
            browser(
                "PATCH",
                "/api/feeds/1",
                r#"{"url":"https://example.org/new","title":"Changed"}"#,
            ),
            browser("DELETE", "/api/feeds/1", ""),
            api(
                "PATCH",
                "/api/v1/feeds/1",
                r#"{"url":"https://example.org/new","folder":"Must not be created"}"#,
                "write-secret",
            ),
            api("DELETE", "/api/v1/feeds/1", "", "write-secret"),
        ] {
            assert_eq!(
                router.clone().oneshot(req).await.unwrap().status(),
                StatusCode::CONFLICT
            );
        }
        {
            let c = db.lock().unwrap();
            assert_eq!(
                c.query_row(
                    "SELECT count(*) FROM folders WHERE name='Must not be created'",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
                0
            );
            assert_eq!(
                c.query_row("SELECT title FROM feeds WHERE id=1", [], |r| r
                    .get::<_, String>(0))
                    .unwrap(),
                "Original"
            );
            c.execute(
                "UPDATE refresh_leases SET expires_at=unixepoch()-1 WHERE feed_id=1",
                [],
            )
            .unwrap();
        }
        assert_eq!(
            router
                .clone()
                .oneshot(browser(
                    "PATCH",
                    "/api/feeds/1",
                    r#"{"url":"https://EXAMPLE.org/original#ignored"}"#
                ))
                .await
                .unwrap()
                .status(),
            StatusCode::OK
        );
        assert_eq!(
            db.lock()
                .unwrap()
                .query_row("SELECT etag FROM feeds WHERE id=1", [], |r| r
                    .get::<_, String>(0))
                .unwrap(),
            "old-etag"
        );
        assert_eq!(
            router
                .clone()
                .oneshot(api(
                    "PATCH",
                    "/api/v1/feeds/1",
                    r#"{"url":"https://example.org/changed#fragment","folder":"New folder"}"#,
                    "write-secret"
                ))
                .await
                .unwrap()
                .status(),
            StatusCode::OK
        );
        {
            let c = db.lock().unwrap();
            let (url, etag, modified, checked, error): (
                String,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<String>,
            ) = c
                .query_row(
                    "SELECT url,etag,modified,last_checked,last_error FROM feeds WHERE id=1",
                    [],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
                )
                .unwrap();
            assert_eq!(url, "https://example.org/changed");
            assert_eq!((etag, modified, checked, error), (None, None, None, None));
            assert!(
                c.query_row(
                    "SELECT is_read AND is_starred FROM articles WHERE id=1",
                    [],
                    |r| r.get::<_, bool>(0)
                )
                .unwrap()
            );
            assert_eq!(
                c.query_row(
                    "SELECT visited_at FROM article_visits WHERE article_id=1",
                    [],
                    |r| r.get::<_, String>(0)
                )
                .unwrap(),
                "2026-01-02"
            );
        }
        assert_eq!(
            router
                .oneshot(browser("DELETE", "/api/feeds/2", ""))
                .await
                .unwrap()
                .status(),
            StatusCode::NO_CONTENT
        );
    }
    #[tokio::test]
    async fn production_sessions_keys_and_limits() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("db");
        let db = Arc::new(Mutex::new(open_db(&path, false).unwrap()));
        let access = Access {
            mode: Mode::Local,
            sessions: Arc::new(Mutex::new(HashMap::new())),
        };
        let bind = "127.0.0.1:8080".parse().unwrap();
        let router = wrap(
            base_router(db.clone()).merge(routes(access.clone()).with_state(db.clone())),
            db.clone(),
            access.clone(),
            bind,
            None,
        );
        assert_eq!(
            router
                .clone()
                .oneshot(request(
                    "GET",
                    "/api/session",
                    "evil.test",
                    None,
                    None,
                    None,
                    ""
                ))
                .await
                .unwrap()
                .status(),
            StatusCode::FORBIDDEN
        );
        let response = router
            .clone()
            .oneshot(request(
                "GET",
                "/api/session",
                "127.0.0.1:8080",
                None,
                None,
                None,
                "",
            ))
            .await
            .unwrap();
        let cookie = response.headers()[header::SET_COOKIE]
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_owned();
        let body = to_bytes(response.into_body(), 10000).await.unwrap();
        let csrf = serde_json::from_slice::<serde_json::Value>(&body).unwrap()["csrf_token"]
            .as_str()
            .unwrap()
            .to_owned();
        for (c, x, o) in [
            (None, None, None),
            (Some(cookie.as_str()), None, None),
            (
                Some(cookie.as_str()),
                Some(csrf.as_str()),
                Some("http://evil.test"),
            ),
        ] {
            assert_eq!(
                router
                    .clone()
                    .oneshot(request(
                        "POST",
                        "/api/keys",
                        "127.0.0.1:8080",
                        c,
                        x,
                        o,
                        r#"{"name":"test","scope":"write","days":1}"#
                    ))
                    .await
                    .unwrap()
                    .status(),
                StatusCode::FORBIDDEN
            );
        }
        let response = router
            .clone()
            .oneshot(request(
                "POST",
                "/api/keys",
                "127.0.0.1:8080",
                Some(&cookie),
                Some(&csrf),
                None,
                r#"{"name":"test","scope":"write","days":1}"#,
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 10000).await.unwrap();
        let token = serde_json::from_slice::<serde_json::Value>(&body).unwrap()["token"]
            .as_str()
            .unwrap()
            .to_owned();
        assert!(
            !std::fs::read(&path)
                .unwrap()
                .windows(token.len())
                .any(|w| w == token.as_bytes())
        );
        db.lock()
            .unwrap()
            .execute("UPDATE api_rate_settings SET rate_limit=1", [])
            .unwrap();
        let mut h = HeaderMap::new();
        h.insert(
            header::AUTHORIZATION,
            format!("Bearer {token}").parse().unwrap(),
        );
        assert!(authenticate(&db, &h, true).is_ok());
        assert_eq!(
            authenticate(&db, &h, false).unwrap_err().status(),
            StatusCode::TOO_MANY_REQUESTS
        );
        db.lock()
            .unwrap()
            .execute("UPDATE api_keys SET revoked_at=datetime('now')", [])
            .unwrap();
        assert_eq!(
            authenticate(&db, &h, false).unwrap_err().status(),
            StatusCode::UNAUTHORIZED
        );
        access
            .sessions
            .lock()
            .unwrap()
            .values_mut()
            .next()
            .unwrap()
            .1 = Instant::now() - Duration::from_secs(43201);
        assert_eq!(
            router
                .oneshot(request(
                    "GET",
                    "/api/keys",
                    "127.0.0.1:8080",
                    Some(&cookie),
                    None,
                    None,
                    ""
                ))
                .await
                .unwrap()
                .status(),
            StatusCode::FORBIDDEN
        );
    }
    #[tokio::test]
    async fn feature_routes_enforce_browser_boundary() {
        let dir = tempfile::tempdir().unwrap();
        let db = Arc::new(Mutex::new(open_db(&dir.path().join("db"), false).unwrap()));
        {
            let db = db.lock().unwrap();
            db.execute("INSERT INTO api_keys(name,prefix,token_hash,scope,expires_at) VALUES('test','prefix',?,'write',datetime('now','+1 day'))", [hash("write-secret")]).unwrap();
            db.execute("INSERT INTO articles(id,title,content,url,published) VALUES(1,'Synthetic','body','https://example.invalid/article','2026-01-01')", []).unwrap();
            db.execute("INSERT INTO article_revision_state(article_id,current_revision,fingerprint,first_seen_at,ingestion_kind) VALUES(1,1,'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',datetime('now'),'normal')", []).unwrap();
        }
        let access = Access {
            mode: Mode::Local,
            sessions: Arc::new(Mutex::new(HashMap::new())),
        };
        let router = wrap(
            base_router(db.clone()).merge(routes(access.clone()).with_state(db.clone())),
            db.clone(),
            access,
            "127.0.0.1:8080".parse().unwrap(),
            None,
        );
        let host = "127.0.0.1:8080";
        let session = router
            .clone()
            .oneshot(request("GET", "/api/session", host, None, None, None, ""))
            .await
            .unwrap();
        let cookie = session.headers()[header::SET_COOKIE]
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_owned();
        let csrf = serde_json::from_slice::<serde_json::Value>(
            &to_bytes(session.into_body(), 4096).await.unwrap(),
        )
        .unwrap()["csrf_token"]
            .as_str()
            .unwrap()
            .to_owned();
        let reads = [
            "/api/smart-folders",
            "/api/mute-rules",
            "/api/duplicates",
            "/api/digest",
            "/api/read-later",
            "/api/read-later/settings",
            "/api/social/sources",
            "/api/collection/health",
            "/api/catchup",
            "/api/catchup/interests",
            "/api/articles/1/revisions",
            "/api/articles/1/related",
        ];
        for path in reads {
            let ok = router
                .clone()
                .oneshot(request("GET", path, host, Some(&cookie), None, None, ""))
                .await
                .unwrap();
            assert_eq!(ok.status(), StatusCode::OK, "GET {path}");
            let blocked = router
                .clone()
                .oneshot(request(
                    "GET",
                    path,
                    host,
                    Some(&cookie),
                    None,
                    Some("https://attacker.invalid"),
                    "",
                ))
                .await
                .unwrap();
            assert_eq!(
                blocked.status(),
                StatusCode::FORBIDDEN,
                "cross-origin GET {path}"
            );
        }
        let writes = [
            (
                "POST",
                "/api/smart-folders",
                r#"{"name":"Synthetic","snapshot":{"filter":"all","query":"","source":null,"applied":{"order":"new","scope":"both"}}}"#,
            ),
            (
                "POST",
                "/api/mute-rules",
                r#"{"name":"Synthetic","term":"test","field":"title","feed_id":null,"enabled":true}"#,
            ),
            ("POST", "/api/articles/1/read-later", r#"{"saved":true}"#),
            (
                "PATCH",
                "/api/read-later/settings",
                r#"{"auto_remove":true}"#,
            ),
            (
                "POST",
                "/api/social/sources",
                r#"{"provider":"invalid","actor":"synthetic"}"#,
            ),
            ("DELETE", "/api/social/sources/999", ""),
            (
                "PUT",
                "/api/catchup/interests",
                r#"{"revision":1,"terms":[],"priority_feed_ids":[]}"#,
            ),
            (
                "PUT",
                "/api/catchup/acknowledgements",
                r#"{"article_id":1,"revision":1,"fingerprint":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","action":"confirmed"}"#,
            ),
            (
                "DELETE",
                "/api/catchup/acknowledgements",
                r#"{"article_id":1,"revision":1,"fingerprint":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}"#,
            ),
            (
                "PUT",
                "/api/articles/1/related-exclusion",
                r#"{"excluded":true}"#,
            ),
        ];
        for (method, path, body) in writes {
            let mut bearer = request(method, path, host, None, None, None, body);
            bearer.headers_mut().insert(
                header::AUTHORIZATION,
                "Bearer write-secret".parse().unwrap(),
            );
            assert_eq!(
                router.clone().oneshot(bearer).await.unwrap().status(),
                StatusCode::FORBIDDEN,
                "bearer {method} {path}"
            );
            for token in [None, Some("wrong")] {
                assert_eq!(
                    router
                        .clone()
                        .oneshot(request(
                            method,
                            path,
                            host,
                            Some(&cookie),
                            token,
                            None,
                            body
                        ))
                        .await
                        .unwrap()
                        .status(),
                    StatusCode::FORBIDDEN,
                    "CSRF {method} {path}"
                );
            }
            assert_eq!(
                router
                    .clone()
                    .oneshot(request(
                        method,
                        path,
                        host,
                        Some(&cookie),
                        Some(&csrf),
                        Some("https://attacker.invalid"),
                        body
                    ))
                    .await
                    .unwrap()
                    .status(),
                StatusCode::FORBIDDEN,
                "origin {method} {path}"
            );
            let status = router
                .clone()
                .oneshot(request(
                    method,
                    path,
                    host,
                    Some(&cookie),
                    Some(&csrf),
                    None,
                    body,
                ))
                .await
                .unwrap()
                .status();
            if path == "/api/social/sources/999" {
                assert_eq!(status, StatusCode::NOT_FOUND);
            } else if path == "/api/social/sources" {
                assert_eq!(status, StatusCode::BAD_REQUEST);
            } else {
                assert!(status.is_success(), "authorized {method} {path}: {status}");
            }
        }
        let mut admin = request("GET", "/api/keys", host, None, None, None, "");
        admin.headers_mut().insert(
            header::AUTHORIZATION,
            "Bearer write-secret".parse().unwrap(),
        );
        assert_eq!(
            router.oneshot(admin).await.unwrap().status(),
            StatusCode::FORBIDDEN
        );
    }
    #[test]
    fn v6_migration_preserves_data_and_backup() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("db.sqlite3");
        let db = open_db(&p, false).unwrap();
        db.execute(
            "INSERT INTO articles(title,content,url,published) VALUES('saved','body','url','date')",
            [],
        )
        .unwrap();
        crate::extensions::drop_for_legacy_fixture(&db);
        db.execute_batch("INSERT INTO folders(id,name) VALUES(10,'folder'); INSERT INTO feeds(id,url,title,folder_id) VALUES(20,'https://example.com/feed','feed',10); UPDATE articles SET is_read=1,is_starred=1,feed_id=20; INSERT INTO article_visits(article_id,visited_at) VALUES(1,'2026-01-01'); UPDATE reader_settings SET default_view='starred'; DROP TABLE feed_poll_intervals; DROP TABLE api_keys; DROP TABLE api_rate_settings; PRAGMA user_version=6;")
        .unwrap();
        drop(db);
        let db = open_db(&p, false).unwrap();
        assert_eq!(
            db.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
                .unwrap(),
            10
        );
        assert_eq!(
            db.query_row("SELECT title FROM articles", [], |r| r.get::<_, String>(0))
                .unwrap(),
            "saved"
        );
        assert_eq!(
            db.query_row("SELECT is_read,is_starred,feed_id FROM articles", [], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, i64>(2)?,
                ))
            })
            .unwrap(),
            (1, 1, 20)
        );
        assert_eq!(
            db.query_row(
                "SELECT visited_at FROM article_visits WHERE article_id=1",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            "2026-01-01"
        );
        assert_eq!(
            db.query_row("SELECT folder_id FROM folder_order", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            10
        );
        assert_eq!(
            db.query_row("SELECT default_view FROM reader_settings", [], |r| r
                .get::<_, String>(0))
                .unwrap(),
            "starred"
        );
        assert_eq!(
            db.query_row("SELECT rate_limit FROM api_rate_settings", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            120
        );
        let backup = p.with_extension("v6-backup.sqlite3");
        assert!(backup.exists());
        let old = Connection::open(backup).unwrap();
        assert_eq!(
            old.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
                .unwrap(),
            6
        );
        assert_eq!(
            old.query_row("SELECT is_starred FROM articles", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
        drop(db);
        assert!(open_db(&p, false).is_ok());
    }
    #[tokio::test]
    async fn tunnel_boundary_and_session_capacity() {
        let dir = tempfile::tempdir().unwrap();
        let db = Arc::new(Mutex::new(open_db(&dir.path().join("db"), false).unwrap()));
        let access = Access {
            mode: Mode::Tunnel {
                origin: "https://feed.example".into(),
                authority: "feed.example".into(),
            },
            sessions: Arc::new(Mutex::new(HashMap::new())),
        };
        let bind = "127.0.0.1:8080".parse().unwrap();
        let router = wrap(
            base_router(db.clone()).merge(routes(access.clone()).with_state(db.clone())),
            db,
            access.clone(),
            bind,
            None,
        );
        assert_eq!(
            router
                .clone()
                .oneshot(request(
                    "GET",
                    "/api/articles/1/hero",
                    "feed.example",
                    None,
                    None,
                    Some("https://feed.example"),
                    ""
                ))
                .await
                .unwrap()
                .status(),
            StatusCode::FORBIDDEN
        );
        let mut spoof = request(
            "GET",
            "/api/session",
            "evil.example",
            None,
            None,
            Some("https://feed.example"),
            "",
        );
        spoof
            .headers_mut()
            .insert("x-forwarded-host", "feed.example".parse().unwrap());
        assert_eq!(
            router.clone().oneshot(spoof).await.unwrap().status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            router
                .clone()
                .oneshot(request(
                    "GET",
                    "/healthz",
                    "evil.example",
                    None,
                    None,
                    None,
                    ""
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
                    "GET",
                    "/api/session",
                    "feed.example",
                    None,
                    None,
                    Some("http://feed.example"),
                    ""
                ))
                .await
                .unwrap()
                .status(),
            StatusCode::FORBIDDEN
        );
        let response = router
            .clone()
            .oneshot(request(
                "GET",
                "/api/session",
                "feed.example",
                None,
                None,
                Some("https://feed.example"),
                "",
            ))
            .await
            .unwrap();
        assert!(
            response.headers()[header::SET_COOKIE]
                .to_str()
                .unwrap()
                .contains("; Secure")
        );
        let cookie = response.headers()[header::SET_COOKIE]
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_owned();
        let bytes = to_bytes(response.into_body(), 4096).await.unwrap();
        let csrf = serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["csrf_token"]
            .as_str()
            .unwrap()
            .to_owned();
        assert_eq!(
            router
                .clone()
                .oneshot(request(
                    "POST",
                    "/api/keys",
                    "feed.example",
                    Some(&cookie),
                    Some(&csrf),
                    Some("https://feed.example"),
                    r#"{"name":"key","scope":"read","days":1}"#
                ))
                .await
                .unwrap()
                .status(),
            StatusCode::OK
        );
        let duplicate = format!("{cookie}; {cookie}");
        assert_eq!(
            router
                .clone()
                .oneshot(request(
                    "GET",
                    "/api/keys",
                    "feed.example",
                    Some(&duplicate),
                    None,
                    None,
                    ""
                ))
                .await
                .unwrap()
                .status(),
            StatusCode::FORBIDDEN
        );
        let mut duplicate = request("GET", "/api/session", "feed.example", None, None, None, "");
        duplicate
            .headers_mut()
            .append(header::HOST, "feed.example".parse().unwrap());
        assert_eq!(
            router.clone().oneshot(duplicate).await.unwrap().status(),
            StatusCode::FORBIDDEN
        );
        {
            let mut sessions = access.sessions.lock().unwrap();
            sessions.clear();
            for i in 0..1024 {
                sessions.insert(format!("old{i}"), ("csrf".into(), Instant::now()));
            }
        }
        assert_eq!(
            router
                .oneshot(request(
                    "GET",
                    "/api/session",
                    "feed.example",
                    None,
                    None,
                    None,
                    ""
                ))
                .await
                .unwrap()
                .status(),
            StatusCode::OK
        );
        assert_eq!(access.sessions.lock().unwrap().len(), 1024);
    }
    #[tokio::test]
    async fn public_scope_expiry_and_rate_responses() {
        let dir = tempfile::tempdir().unwrap();
        let db = Arc::new(Mutex::new(open_db(&dir.path().join("db"), false).unwrap()));
        let access = Access {
            mode: Mode::Local,
            sessions: Arc::new(Mutex::new(HashMap::new())),
        };
        let router = wrap(
            base_router(db.clone()),
            db.clone(),
            access,
            "127.0.0.1:8080".parse().unwrap(),
            None,
        );
        {
            let db = db.lock().unwrap();
            db.execute("INSERT INTO api_keys(name,prefix,token_hash,scope,expires_at) VALUES('read','prefix',?1,'read',datetime('now','+1 day'))",[hash("secret")]).unwrap();
        }
        let api = |method: &str, token: &str| {
            let mut r = request(
                method,
                "/api/v1/articles",
                "127.0.0.1:8080",
                None,
                None,
                None,
                "",
            );
            r.headers_mut().insert(
                header::AUTHORIZATION,
                format!("bEaReR {token}").parse().unwrap(),
            );
            r
        };
        assert_eq!(
            router
                .clone()
                .oneshot(api("GET", "secret"))
                .await
                .unwrap()
                .status(),
            StatusCode::OK
        );
        let r = router
            .clone()
            .oneshot(api("PATCH", "secret"))
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::FORBIDDEN);
        let b = to_bytes(r.into_body(), 4096).await.unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&b).unwrap()["error"]["code"],
            403
        );
        {
            let db = db.lock().unwrap();
            db.execute("UPDATE api_rate_settings SET rate_limit=1", [])
                .unwrap();
        }
        let r = router.clone().oneshot(api("GET", "secret")).await.unwrap();
        assert_eq!(r.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(
            (1..=60).contains(
                &r.headers()[header::RETRY_AFTER]
                    .to_str()
                    .unwrap()
                    .parse::<u64>()
                    .unwrap()
            )
        );
        {
            let db = db.lock().unwrap();
            db.execute(
                "UPDATE api_keys SET expires_at=datetime('now','-1 day')",
                [],
            )
            .unwrap();
        }
        let r = router.clone().oneshot(api("GET", "secret")).await.unwrap();
        assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(r.headers()[header::WWW_AUTHENTICATE], "Bearer");
        assert_eq!(
            router
                .oneshot(api("GET", &"a".repeat(257)))
                .await
                .unwrap()
                .status(),
            StatusCode::UNAUTHORIZED
        );
    }
}
