use super::*;
use url::Url;

pub const SCHEMA: &str = "CREATE TABLE social_sources (feed_id INTEGER PRIMARY KEY REFERENCES feeds(id) ON DELETE CASCADE, provider TEXT NOT NULL CHECK(provider IN ('mastodon','bluesky')), actor TEXT NOT NULL, server TEXT NOT NULL, remote_id TEXT NOT NULL, UNIQUE(provider,server,remote_id))";
pub fn migrate(tx: &rusqlite::Transaction<'_>) -> rusqlite::Result<()> {
    tx.execute(SCHEMA, [])?;
    Ok(())
}

#[derive(Clone, Serialize)]
pub(super) struct Source {
    feed_id: i64,
    provider: String,
    actor: String,
    server: String,
    remote_id: String,
    title: String,
    paused: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    provider: String,
    actor: String,
    server: Option<String>,
}
fn bad<T>(_: T) -> StatusCode {
    StatusCode::BAD_GATEWAY
}
fn source(db: &Connection, id: i64) -> Result<Option<Source>, StatusCode> {
    // Older database fixtures and pre-v9 databases have no social sources.
    let exists: bool = db.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='social_sources')", [], |r| r.get(0)).map_err(collection::err)?;
    if !exists {
        return Ok(None);
    }
    db.query_row("SELECT s.feed_id,s.provider,s.actor,s.server,s.remote_id,f.title,f.paused FROM social_sources s JOIN feeds f ON f.id=s.feed_id WHERE s.feed_id=?", [id], |r| Ok(Source { feed_id:r.get(0)?,provider:r.get(1)?,actor:r.get(2)?,server:r.get(3)?,remote_id:r.get(4)?,title:r.get(5)?,paused:r.get(6)? })).optional().map_err(collection::err)
}
pub(super) fn is_source(db: &Connection, id: i64) -> Result<bool, StatusCode> {
    Ok(source(db, id)?.is_some())
}
fn validate(input: &Input) -> Result<(String, String), StatusCode> {
    let actor = input.actor.trim().trim_start_matches('@');
    if actor.is_empty()
        || actor.len() > 253
        || !actor
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-:@".contains(&b))
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    let server = match input.provider.as_str() {
        "bluesky" if input.server.as_deref().is_none_or(|s| s.trim().is_empty()) => {
            "https://public.api.bsky.app".to_string()
        }
        "mastodon" => {
            let raw = input
                .server
                .as_deref()
                .ok_or(StatusCode::BAD_REQUEST)?
                .trim();
            if raw.len() > 2048 {
                return Err(StatusCode::BAD_REQUEST);
            }
            let u = Url::parse(raw).map_err(|_| StatusCode::BAD_REQUEST)?;
            if !matches!(u.scheme(), "http" | "https")
                || u.host_str().is_none()
                || !u.username().is_empty()
                || u.password().is_some()
                || u.path() != "/"
                || u.query().is_some()
                || u.fragment().is_some()
            {
                return Err(StatusCode::BAD_REQUEST);
            }
            u.origin().ascii_serialization()
        }
        _ => return Err(StatusCode::BAD_REQUEST),
    };
    Ok((actor.to_string(), server))
}
async fn json(url: &Url) -> Result<serde_json::Value, StatusCode> {
    let response = collection::fetch_body(url.as_str()).await?;
    serde_json::from_slice(&response.body.ok_or(StatusCode::BAD_GATEWAY)?).map_err(bad)
}
fn field<'a>(v: &'a serde_json::Value, key: &str) -> Result<&'a str, StatusCode> {
    v.get(key)
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty() && s.len() <= 2048)
        .ok_or(StatusCode::BAD_GATEWAY)
}
fn endpoint(server: &str, path: &str, params: &[(&str, &str)]) -> Result<Url, StatusCode> {
    let mut u = Url::parse(server).map_err(bad)?.join(path).map_err(bad)?;
    u.query_pairs_mut().extend_pairs(params.iter().copied());
    Ok(u)
}
async fn resolve(input: &Input) -> Result<Source, StatusCode> {
    let (actor, server) = validate(input)?;
    let path = if input.provider == "mastodon" {
        "/api/v1/accounts/lookup"
    } else {
        "/xrpc/app.bsky.actor.getProfile"
    };
    let key = if input.provider == "mastodon" {
        "acct"
    } else {
        "actor"
    };
    let v = json(&endpoint(&server, path, &[(key, &actor)])?).await?;
    let remote_id = field(
        &v,
        if input.provider == "mastodon" {
            "id"
        } else {
            "did"
        },
    )?
    .to_owned();
    if input.provider == "mastodon" && !remote_id.bytes().all(|b| b.is_ascii_digit()) {
        return Err(StatusCode::BAD_GATEWAY);
    }
    if input.provider == "bluesky"
        && (!remote_id.starts_with("did:")
            || !remote_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b)))
    {
        return Err(StatusCode::BAD_GATEWAY);
    }
    let title = v
        .get("display_name")
        .or_else(|| v.get("displayName"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty() && s.len() <= 512)
        .unwrap_or(&actor)
        .to_string();
    Ok(Source {
        feed_id: 0,
        provider: input.provider.clone(),
        actor,
        server,
        remote_id,
        title,
        paused: false,
    })
}
fn xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
        .chars()
        .filter(|c| *c == '\n' || *c == '\r' || *c == '\t' || *c >= ' ')
        .collect()
}
fn parsed(s: &Source, v: &serde_json::Value) -> Result<feed_rs::model::Feed, StatusCode> {
    let items = if s.provider == "mastodon" {
        v.as_array()
    } else {
        v.get("feed").and_then(|v| v.as_array())
    }
    .ok_or(StatusCode::BAD_GATEWAY)?;
    if items.len() > 100 {
        return Err(StatusCode::BAD_GATEWAY);
    }
    let mut atom = format!(
        "<feed xmlns=\"http://www.w3.org/2005/Atom\"><id>{}</id><title>{}</title><updated>1970-01-01T00:00:00Z</updated>",
        xml(&s.server),
        xml(&s.title)
    );
    for item in items {
        let (id, link, text, date) = if s.provider == "mastodon" {
            if item.get("visibility").and_then(|v| v.as_str()) != Some("public")
                || !item.get("reblog").is_none_or(|v| v.is_null())
            {
                continue;
            }
            (
                field(item, "id")?.to_owned(),
                field(item, "url")?.to_owned(),
                field(item, "content")?.to_owned(),
                field(item, "created_at")?.to_owned(),
            )
        } else {
            if item.get("reason").is_some() {
                continue;
            }
            let p = item.get("post").ok_or(StatusCode::BAD_GATEWAY)?;
            if p.pointer("/author/did").and_then(|x| x.as_str()) != Some(s.remote_id.as_str()) {
                continue;
            }
            let uri = field(p, "uri")?;
            let prefix = format!("at://{}/app.bsky.feed.post/", s.remote_id);
            let rkey = uri
                .strip_prefix(&prefix)
                .filter(|x| {
                    !x.is_empty()
                        && x.bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b"._~-".contains(&b))
                })
                .ok_or(StatusCode::BAD_GATEWAY)?;
            let rec = p.get("record").ok_or(StatusCode::BAD_GATEWAY)?;
            (
                uri.to_string(),
                format!("https://bsky.app/profile/{}/post/{}", s.remote_id, rkey),
                field(rec, "text")?.to_string(),
                field(rec, "createdAt")?.to_string(),
            )
        };
        let safe = opml::safe_url(&link).ok_or(StatusCode::BAD_GATEWAY)?;
        let content = if s.provider == "mastodon" {
            collection::plain(&text)
        } else {
            text
        };
        let title: String = content.chars().take(120).collect();
        atom.push_str(&format!("<entry><id>{}</id><title>{}</title><link href=\"{}\"/><updated>{}</updated><published>{}</published><content type=\"text\">{}</content></entry>",xml(&id),xml(&xml(&title)),xml(&safe),xml(&date),xml(&date),xml(&xml(&content))));
    }
    atom.push_str("</feed>");
    collection::parse_feed(atom.as_bytes(), &Url::parse(&s.server).map_err(bad)?)
}
pub(super) fn source_for_feed(db: &Connection, id: i64) -> Result<Option<Source>, StatusCode> {
    source(db, id)
}
pub(super) async fn fetch(s: Source) -> Result<collection::Fetched, StatusCode> {
    let u = if s.provider == "mastodon" {
        endpoint(
            &s.server,
            &format!("/api/v1/accounts/{}/statuses", s.remote_id),
            &[("limit", "40"), ("exclude_reblogs", "true")],
        )?
    } else {
        endpoint(
            &s.server,
            "/xrpc/app.bsky.feed.getAuthorFeed",
            &[
                ("actor", &s.remote_id),
                ("limit", "40"),
                ("filter", "posts_no_replies"),
            ],
        )?
    };
    let v = json(&u).await?;
    Ok(collection::Fetched {
        parsed: Some(parsed(&s, &v)?),
        etag: None,
        modified: None,
    })
}
async fn list(State(db): State<Db>) -> Result<Json<serde_json::Value>, StatusCode> {
    let db = db.lock().map_err(collection::err)?;
    let mut q = db
        .prepare("SELECT feed_id FROM social_sources ORDER BY feed_id")
        .map_err(collection::err)?;
    let ids = q
        .query_map([], |r| r.get::<_, i64>(0))
        .map_err(collection::err)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(collection::err)?;
    let sources = ids
        .into_iter()
        .map(|id| source(&db, id).and_then(|s| s.ok_or(StatusCode::INTERNAL_SERVER_ERROR)))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Json(serde_json::json!({"sources":sources})))
}
async fn create(
    State(db): State<Db>,
    headers: HeaderMap,
    Json(input): Json<Input>,
) -> Result<Json<Source>, StatusCode> {
    if !origin_allowed(&headers) {
        return Err(StatusCode::FORBIDDEN);
    }
    let mut s = resolve(&input).await?;
    let url = if s.provider == "mastodon" {
        format!("{}/@{}", s.server, s.actor)
    } else {
        format!("https://bsky.app/profile/{}", s.remote_id)
    };
    let mut db = db.lock().map_err(collection::err)?;
    let tx = db.transaction().map_err(collection::err)?;
    tx.execute(
        "INSERT INTO feeds(url,title) VALUES (?,?)",
        params![url, s.title],
    )
    .map_err(collection::db_write_error)?;
    s.feed_id = tx.last_insert_rowid();
    tx.execute(
        "INSERT INTO social_sources(feed_id,provider,actor,server,remote_id) VALUES (?,?,?,?,?)",
        params![s.feed_id, s.provider, s.actor, s.server, s.remote_id],
    )
    .map_err(collection::db_write_error)?;
    tx.commit().map_err(collection::err)?;
    Ok(Json(s))
}
async fn delete(
    State(db): State<Db>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<StatusCode, StatusCode> {
    if !origin_allowed(&headers) {
        return Err(StatusCode::FORBIDDEN);
    }
    let mut db = db.lock().map_err(collection::err)?;
    let tx = db.transaction().map_err(collection::err)?;
    if !is_source(&tx, id)? {
        return Err(StatusCode::NOT_FOUND);
    }
    collection::ensure_not_refreshing(&tx, id)?;
    tx.execute("DELETE FROM feeds WHERE id=?", [id])
        .map_err(collection::err)?;
    tx.commit().map_err(collection::err)?;
    Ok(StatusCode::NO_CONTENT)
}
pub fn routes() -> Router<Db> {
    Router::new()
        .route("/api/social/sources", get(list).post(create))
        .route("/api/social/sources/{id}", axum::routing::delete(delete))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(provider: &str) -> Source {
        Source {
            feed_id: 1,
            provider: provider.into(),
            actor: "alice".into(),
            server: "https://example.com".into(),
            remote_id: if provider == "mastodon" {
                "123"
            } else {
                "did:plc:alice"
            }
            .into(),
            title: "Alice".into(),
            paused: false,
        }
    }
    #[test]
    fn validates_origins_and_actors() {
        let mut input = Input {
            provider: "mastodon".into(),
            actor: "alice".into(),
            server: Some("https://example.com".into()),
        };
        assert!(validate(&input).is_ok());
        for server in [
            "https://user@example.com",
            "https://example.com/path",
            "file:///tmp/test",
            "https://example.com/?x=1",
        ] {
            input.server = Some(server.into());
            assert!(validate(&input).is_err());
        }
        input.server = Some("https://example.com".into());
        input.actor = "alice/../../test".into();
        assert!(validate(&input).is_err());
    }
    #[test]
    fn mastodon_only_public_and_escapes_content() {
        let v = serde_json::json!([{ "id":"1", "visibility":"public", "reblog":null,"url":"https://example.com/@alice/1", "content":"<p>fish &amp; chips</p>","created_at":"2025-01-01T00:00:00Z" }, {"visibility":"private"}, {"visibility":"unlisted"}, {"visibility":"public","reblog":{}}]);
        let feed = parsed(&fixture("mastodon"), &v).unwrap();
        assert_eq!(feed.entries.len(), 1);
        assert_eq!(feed.entries[0].id, "1");
        assert!(
            collection::plain(
                feed.entries[0]
                    .content
                    .as_ref()
                    .unwrap()
                    .body
                    .as_ref()
                    .unwrap()
            )
            .contains("fish & chips")
        );
    }
    #[test]
    fn bluesky_excludes_reposts_and_other_authors() {
        let post = serde_json::json!({"uri":"at://did:plc:alice/app.bsky.feed.post/abc", "author":{"did":"did:plc:alice"},"record":{"text":"hello <world>","createdAt":"2025-01-01T00:00:00Z"}});
        let v = serde_json::json!({"feed":[{"post":post},{"post":post,"reason":{"$type":"app.bsky.feed.defs#reasonRepost"}},{"post":{"author":{"did":"did:plc:other"}}}]});
        let feed = parsed(&fixture("bluesky"), &v).unwrap();
        assert_eq!(feed.entries.len(), 1);
        assert!(feed.entries[0].links[0].href.contains("/post/abc"));
    }
    #[tokio::test]
    async fn unsafe_social_endpoint_is_rejected() {
        let url = Url::parse("http://127.0.0.1/api/v1/accounts/lookup").unwrap();
        assert_eq!(json(&url).await.unwrap_err(), StatusCode::BAD_REQUEST);
    }
    #[test]
    fn social_feed_url_cannot_be_edited() {
        let dir = tempfile::tempdir().unwrap();
        let db = open_db(&dir.path().join("db"), false).unwrap();
        db.execute_batch("INSERT INTO feeds(id,url,title) VALUES(1,'https://example.org/@alice','Alice'); INSERT INTO social_sources VALUES(1,'mastodon','alice','https://example.org','123');").unwrap();
        assert_eq!(
            collection::edit_url(&db, 1, Some("https://example.org/other")),
            Err(StatusCode::CONFLICT)
        );
        assert_eq!(
            db.query_row("SELECT url FROM feeds WHERE id=1", [], |r| r
                .get::<_, String>(0))
                .unwrap(),
            "https://example.org/@alice"
        );
    }
    #[test]
    fn schema_cascades_with_feed() {
        let mut db = Connection::open_in_memory().unwrap();
        db.execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE feeds(id INTEGER PRIMARY KEY);")
            .unwrap();
        let tx = db.transaction().unwrap();
        migrate(&tx).unwrap();
        tx.execute("INSERT INTO feeds VALUES (1)", []).unwrap();
        tx.execute("INSERT INTO social_sources VALUES (1,'bluesky','alice','https://public.api.bsky.app','did:plc:alice')",[]).unwrap();
        tx.execute("DELETE FROM feeds WHERE id=1", []).unwrap();
        assert_eq!(
            tx.query_row("SELECT count(*) FROM social_sources", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
}
