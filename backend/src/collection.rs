use super::*;
use axum::routing::patch;
use futures_util::StreamExt;
use std::{
    net::{IpAddr, SocketAddr},
    time::Duration,
};
use url::Url;

const FOLDERS: &str = "CREATE TABLE folders (id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE)";
const FEEDS: &str = "CREATE TABLE feeds (id INTEGER PRIMARY KEY, url TEXT NOT NULL UNIQUE, title TEXT NOT NULL, folder_id INTEGER REFERENCES folders(id) ON DELETE SET NULL, paused INTEGER NOT NULL DEFAULT 0 CHECK(paused IN (0,1)), etag TEXT, modified TEXT, last_checked TEXT, last_error TEXT)";
const SETTINGS: &str = "CREATE TABLE collection_settings (id INTEGER PRIMARY KEY CHECK(id=1), enabled INTEGER NOT NULL DEFAULT 0 CHECK(enabled IN (0,1)), interval_seconds INTEGER NOT NULL DEFAULT 900 CHECK(interval_seconds BETWEEN 60 AND 86400))";
const LEASES: &str = "CREATE TABLE refresh_leases (feed_id INTEGER PRIMARY KEY REFERENCES feeds(id) ON DELETE CASCADE, owner TEXT NOT NULL, expires_at INTEGER NOT NULL)";
const INDEX: &str = "CREATE UNIQUE INDEX article_identity ON articles(feed_id,external_id)";
pub fn migrate(tx: &rusqlite::Transaction<'_>) -> rusqlite::Result<()> {
    let version: i64 = tx.pragma_query_value(None, "user_version", |r| r.get(0))?;
    if version < 2 {
        tx.execute(FOLDERS, [])?;
        tx.execute(FEEDS, [])?;
        tx.execute("ALTER TABLE articles ADD COLUMN feed_id INTEGER REFERENCES feeds(id) ON DELETE CASCADE", [])?;
        tx.execute("ALTER TABLE articles ADD COLUMN external_id TEXT", [])?;
        tx.execute(INDEX, [])?;
    }
    if version < 3 {
        tx.execute(SETTINGS, [])?;
        tx.execute("INSERT INTO collection_settings(id) VALUES (1)", [])?;
        tx.execute(LEASES, [])?;
    }
    Ok(())
}
pub fn recognized_v2(objects: &[(String, String, Option<String>)]) -> bool {
    let Ok(mut db) = Connection::open_in_memory() else {
        return false;
    };
    if db.execute(SCHEMA, []).is_err() {
        return false;
    }
    let Ok(tx) = db.transaction() else {
        return false;
    };
    if migrate(&tx).is_err()
        || tx.execute("DROP TABLE refresh_leases", []).is_err()
        || tx.execute("DROP TABLE collection_settings", []).is_err()
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
pub fn recognized_v3(objects: &[(String, String, Option<String>)]) -> bool {
    let Ok(mut db) = Connection::open_in_memory() else {
        return false;
    };
    if db.execute(SCHEMA, []).is_err() {
        return false;
    }
    let Ok(tx) = db.transaction() else {
        return false;
    };
    if migrate(&tx).is_err() || tx.commit().is_err() {
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
const ORDER_SCHEMA: &[&str] = &[
    "CREATE TABLE folder_order (folder_id INTEGER PRIMARY KEY REFERENCES folders(id) ON DELETE CASCADE, position INTEGER NOT NULL)",
    "CREATE TABLE feed_order (feed_id INTEGER PRIMARY KEY REFERENCES feeds(id) ON DELETE CASCADE, position INTEGER NOT NULL)",
    "CREATE TRIGGER folder_order_insert AFTER INSERT ON folders BEGIN INSERT INTO folder_order VALUES (NEW.id, COALESCE((SELECT MAX(position)+1 FROM folder_order),0)); END",
    "CREATE TRIGGER feed_order_insert AFTER INSERT ON feeds BEGIN INSERT INTO feed_order VALUES (NEW.id, COALESCE((SELECT MAX(o.position)+1 FROM feed_order o JOIN feeds f ON f.id=o.feed_id WHERE f.folder_id IS NEW.folder_id),0)); END",
    "CREATE TRIGGER feed_order_move AFTER UPDATE OF folder_id ON feeds WHEN OLD.folder_id IS NOT NEW.folder_id BEGIN UPDATE feed_order SET position=COALESCE((SELECT MAX(o.position)+1 FROM feed_order o JOIN feeds f ON f.id=o.feed_id WHERE f.folder_id IS NEW.folder_id AND f.id != NEW.id),0) WHERE feed_id=NEW.id; END",
];
pub fn migrate_order(tx: &rusqlite::Transaction<'_>) -> rusqlite::Result<()> {
    tx.execute(ORDER_SCHEMA[0], [])?;
    tx.execute(ORDER_SCHEMA[1], [])?;
    tx.execute(
        "INSERT INTO folder_order SELECT id, ROW_NUMBER() OVER (ORDER BY name,id)-1 FROM folders",
        [],
    )?;
    tx.execute("INSERT INTO feed_order SELECT id, ROW_NUMBER() OVER (PARTITION BY folder_id ORDER BY title,id)-1 FROM feeds", [])?;
    for sql in &ORDER_SCHEMA[2..] {
        tx.execute(sql, [])?;
    }
    Ok(())
}
pub fn recognized_v5(objects: &[(String, String, Option<String>)]) -> bool {
    let Ok(mut db) = Connection::open_in_memory() else {
        return false;
    };
    if db.execute(SCHEMA, []).is_err() {
        return false;
    }
    let Ok(tx) = db.transaction() else {
        return false;
    };
    if migrate(&tx).is_err()
        || tx.execute(reader::VISITS, []).is_err()
        || migrate_order(&tx).is_err()
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
struct Folder {
    id: i64,
    name: String,
}
#[derive(Serialize)]
pub(super) struct Feed {
    id: i64,
    url: String,
    title: String,
    folder_id: Option<i64>,
    paused: bool,
    last_checked: Option<String>,
    last_error: Option<String>,
}
fn feed(r: &rusqlite::Row<'_>) -> rusqlite::Result<Feed> {
    Ok(Feed {
        id: r.get(0)?,
        url: r.get(1)?,
        title: r.get(2)?,
        folder_id: r.get(3)?,
        paused: r.get(4)?,
        last_checked: r.get(5)?,
        last_error: r.get(6)?,
    })
}
pub(super) fn get_feed(db: &Connection, id: i64) -> Result<Feed, StatusCode> {
    db.query_row(
        "SELECT id,url,title,folder_id,paused,last_checked,last_error FROM feeds WHERE id=?",
        [id],
        feed,
    )
    .optional()
    .map_err(err)?
    .ok_or(StatusCode::NOT_FOUND)
}
pub(super) fn err<T>(_: T) -> StatusCode {
    StatusCode::INTERNAL_SERVER_ERROR
}
#[derive(Serialize)]
struct Library {
    folders: Vec<Folder>,
    feeds: Vec<Feed>,
}
async fn library(State(db): State<Db>) -> Result<Json<Library>, StatusCode> {
    let mut db = db.lock().map_err(err)?;
    let tx = db.transaction().map_err(err)?;
    let folders = tx
        .prepare("SELECT f.id,f.name FROM folders f JOIN folder_order o ON o.folder_id=f.id ORDER BY o.position,f.id")
        .map_err(err)?
        .query_map([], |r| {
            Ok(Folder {
                id: r.get(0)?,
                name: r.get(1)?,
            })
        })
        .map_err(err)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(err)?;
    let feeds=tx.prepare("SELECT f.id,f.url,f.title,f.folder_id,f.paused,f.last_checked,f.last_error FROM feeds f JOIN feed_order o ON o.feed_id=f.id LEFT JOIN folder_order fo ON fo.folder_id=f.folder_id ORDER BY f.folder_id IS NULL,fo.position,o.position,f.id").map_err(err)?.query_map([],feed).map_err(err)?.collect::<Result<Vec<_>,_>>().map_err(err)?;
    tx.commit().map_err(err)?;
    Ok(Json(Library { folders, feeds }))
}
#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    folders: Vec<i64>,
    feeds: Vec<OrderedFeed>,
}
#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct OrderedFeed {
    id: i64,
    folder_id: Option<i64>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OrderInput {
    expected: Snapshot,
    order: Snapshot,
}
fn snapshot(db: &Connection) -> Result<Snapshot, StatusCode> {
    let folders = db
        .prepare("SELECT folder_id FROM folder_order ORDER BY position,folder_id")
        .map_err(err)?
        .query_map([], |r| r.get(0))
        .map_err(err)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(err)?;
    let feeds = db.prepare("SELECT f.id,f.folder_id FROM feeds f JOIN feed_order o ON o.feed_id=f.id LEFT JOIN folder_order fo ON fo.folder_id=f.folder_id ORDER BY f.folder_id IS NULL,fo.position,o.position,f.id").map_err(err)?.query_map([], |r| Ok(OrderedFeed { id:r.get(0)?, folder_id:r.get(1)? })).map_err(err)?.collect::<Result<Vec<_>,_>>().map_err(err)?;
    Ok(Snapshot { folders, feeds })
}
fn valid_snapshot(candidate: &Snapshot, current: &Snapshot) -> bool {
    use std::collections::{HashMap, HashSet};
    let folders: HashSet<_> = candidate.folders.iter().copied().collect();
    let current_folders: HashSet<_> = current.folders.iter().copied().collect();
    let ids: HashSet<_> = candidate.feeds.iter().map(|f| f.id).collect();
    let current_ids: HashSet<_> = current.feeds.iter().map(|f| f.id).collect();
    if folders.len() != candidate.folders.len()
        || folders != current_folders
        || ids.len() != candidate.feeds.len()
        || ids != current_ids
    {
        return false;
    }
    let ranks: HashMap<_, _> = candidate
        .folders
        .iter()
        .enumerate()
        .map(|(i, id)| (*id, i))
        .collect();
    let mut previous = 0;
    for feed in &candidate.feeds {
        let rank = match feed.folder_id {
            Some(id) => match ranks.get(&id) {
                Some(rank) => *rank,
                None => return false,
            },
            None => candidate.folders.len(),
        };
        if rank < previous {
            return false;
        }
        previous = rank;
    }
    true
}
async fn order_library(
    State(db): State<Db>,
    headers: HeaderMap,
    Json(input): Json<OrderInput>,
) -> Result<StatusCode, StatusCode> {
    if !origin_allowed(&headers) {
        return Err(StatusCode::FORBIDDEN);
    }
    let mut db = db.lock().map_err(err)?;
    let tx = db
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(err)?;
    let current = snapshot(&tx)?;
    // Reject malformed snapshots before comparing stale membership or order.
    if !valid_snapshot(&input.expected, &input.expected)
        || !valid_snapshot(&input.order, &input.order)
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    if input.expected != current {
        return Err(StatusCode::CONFLICT);
    }
    if !valid_snapshot(&input.order, &current) {
        return Err(StatusCode::BAD_REQUEST);
    }
    for (position, id) in input.order.folders.iter().enumerate() {
        tx.execute(
            "UPDATE folder_order SET position=? WHERE folder_id=?",
            params![position as i64, id],
        )
        .map_err(err)?;
    }
    for feed in &input.order.feeds {
        tx.execute(
            "UPDATE feeds SET folder_id=? WHERE id=?",
            params![feed.folder_id, feed.id],
        )
        .map_err(err)?;
    }
    for (position, feed) in input.order.feeds.iter().enumerate() {
        tx.execute(
            "UPDATE feed_order SET position=? WHERE feed_id=?",
            params![position as i64, feed.id],
        )
        .map_err(err)?;
    }
    tx.commit().map_err(err)?;
    Ok(StatusCode::NO_CONTENT)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FolderInput {
    name: String,
}
pub(super) fn clean_name(s: &str) -> Result<&str, StatusCode> {
    let s = s.trim();
    if s.is_empty() || s.len() > 512 {
        Err(StatusCode::BAD_REQUEST)
    } else {
        Ok(s)
    }
}
pub(super) fn db_write_error(e: rusqlite::Error) -> StatusCode {
    if matches!(e,rusqlite::Error::SqliteFailure(ref x,_) if x.code == rusqlite::ErrorCode::ConstraintViolation)
    {
        StatusCode::CONFLICT
    } else {
        err(e)
    }
}
async fn create_folder(
    State(db): State<Db>,
    headers: HeaderMap,
    Json(input): Json<FolderInput>,
) -> Result<Json<Folder>, StatusCode> {
    if !origin_allowed(&headers) {
        return Err(StatusCode::FORBIDDEN);
    }
    let name = clean_name(&input.name)?;
    let db = db.lock().map_err(err)?;
    db.execute("INSERT INTO folders(name) VALUES (?)", [name])
        .map_err(db_write_error)?;
    Ok(Json(Folder {
        id: db.last_insert_rowid(),
        name: name.into(),
    }))
}
async fn edit_folder(
    State(db): State<Db>,
    Path(id): Path<i64>,
    headers: HeaderMap,
    Json(input): Json<FolderInput>,
) -> Result<Json<Folder>, StatusCode> {
    if !origin_allowed(&headers) {
        return Err(StatusCode::FORBIDDEN);
    }
    let name = clean_name(&input.name)?;
    let db = db.lock().map_err(err)?;
    if db
        .execute("UPDATE folders SET name=? WHERE id=?", params![name, id])
        .map_err(db_write_error)?
        == 0
    {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(Json(Folder {
        id,
        name: name.into(),
    }))
}
async fn delete_folder(
    State(db): State<Db>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<StatusCode, StatusCode> {
    if !origin_allowed(&headers) {
        return Err(StatusCode::FORBIDDEN);
    }
    let mut db = db.lock().map_err(err)?;
    let tx = db.transaction().map_err(err)?;
    let ids = tx.prepare("SELECT f.id FROM feeds f JOIN feed_order o ON o.feed_id=f.id WHERE f.folder_id=? ORDER BY o.position,f.id").map_err(err)?.query_map([id], |r| r.get::<_, i64>(0)).map_err(err)?.collect::<Result<Vec<_>,_>>().map_err(err)?;
    for feed_id in ids {
        tx.execute("UPDATE feeds SET folder_id=NULL WHERE id=?", [feed_id])
            .map_err(err)?;
    }
    if tx
        .execute("DELETE FROM folders WHERE id=?", [id])
        .map_err(err)?
        == 0
    {
        return Err(StatusCode::NOT_FOUND);
    }
    tx.commit().map_err(err)?;
    Ok(StatusCode::NO_CONTENT)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NewFeed {
    url: String,
    folder_id: Option<i64>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EditFeed {
    title: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    folder_id: Option<Option<i64>>,
    paused: Option<bool>,
}
fn double_option<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Option<i64>>, D::Error> {
    Option::deserialize(d).map(Some)
}
pub(super) fn folder_exists(db: &Connection, id: Option<i64>) -> Result<(), StatusCode> {
    if let Some(id) = id
        && !db
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM folders WHERE id=?)",
                [id],
                |r| r.get::<_, bool>(0),
            )
            .map_err(err)?
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    Ok(())
}
async fn edit_feed(
    State(db): State<Db>,
    Path(id): Path<i64>,
    headers: HeaderMap,
    Json(input): Json<EditFeed>,
) -> Result<Json<Feed>, StatusCode> {
    if !origin_allowed(&headers) {
        return Err(StatusCode::FORBIDDEN);
    }
    if input.title.is_none() && input.folder_id.is_none() && input.paused.is_none() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let db = db.lock().map_err(err)?;
    let old = get_feed(&db, id)?;
    let title = match &input.title {
        Some(s) => clean_name(s)?,
        None => &old.title,
    };
    let folder = input.folder_id.unwrap_or(old.folder_id);
    folder_exists(&db, folder)?;
    db.execute(
        "UPDATE feeds SET title=?,folder_id=?,paused=? WHERE id=?",
        params![title, folder, input.paused.unwrap_or(old.paused), id],
    )
    .map_err(err)?;
    Ok(Json(get_feed(&db, id)?))
}
async fn delete_feed(
    State(db): State<Db>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<StatusCode, StatusCode> {
    if !origin_allowed(&headers) {
        return Err(StatusCode::FORBIDDEN);
    }
    let db = db.lock().map_err(err)?;
    if db
        .execute("DELETE FROM feeds WHERE id=?", [id])
        .map_err(err)?
        == 0
    {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(StatusCode::NO_CONTENT)
}
pub(super) fn public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v) => {
            let [a, b, c, _] = v.octets();
            !(a == 0
                || a == 10
                || a == 127
                || a >= 224
                || a == 100 && (64..=127).contains(&b)
                || a == 169 && b == 254
                || a == 172 && (16..=31).contains(&b)
                || a == 192 && (b == 168 || b == 0 && (c == 0 || c == 2))
                || a == 198 && (b == 18 || b == 19 || b == 51 && c == 100)
                || a == 203 && b == 0 && c == 113)
        }
        IpAddr::V6(v) => {
            let s = v.segments();
            v.to_ipv4_mapped().is_none()
                && (s[0] & 0xe000) == 0x2000
                && !(s[0] == 0x2001 && (s[1] == 0xdb8 || s[1] < 0x200))
                && s[0] != 0x2002
        }
    }
}
pub(super) struct Fetched {
    pub(super) parsed: Option<feed_rs::model::Feed>,
    pub(super) etag: Option<String>,
    pub(super) modified: Option<String>,
}
pub(super) async fn fetch(
    start: &str,
    etag: Option<&str>,
    modified: Option<&str>,
) -> Result<Fetched, StatusCode> {
    tokio::time::timeout(Duration::from_secs(10), fetch_inner(start, etag, modified))
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?
}
pub(super) struct ResponseBody {
    pub final_url: Url,
    pub body: Option<Vec<u8>>,
    pub etag: Option<String>,
    pub modified: Option<String>,
}
pub(super) async fn fetch_body(start: &str) -> Result<ResponseBody, StatusCode> {
    tokio::time::timeout(Duration::from_secs(10), fetch_body_inner(start, None, None))
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?
}
fn normalized_url(raw: &str) -> Result<Url, StatusCode> {
    let mut url = Url::parse(raw.trim()).map_err(|_| StatusCode::BAD_REQUEST)?;
    url.set_fragment(None);
    Ok(url)
}
async fn fetch_inner(
    start: &str,
    etag: Option<&str>,
    modified: Option<&str>,
) -> Result<Fetched, StatusCode> {
    let response = fetch_body_inner(start, etag, modified).await?;
    let parsed = match response.body {
        Some(body) => Some(parse_feed(&body, &response.final_url)?),
        None => None,
    };
    Ok(Fetched {
        parsed,
        etag: response.etag,
        modified: response.modified,
    })
}
pub(super) fn parse_feed(body: &[u8], url: &Url) -> Result<feed_rs::model::Feed, StatusCode> {
    let parser = feed_rs::parser::Builder::new()
        .base_uri(Some(url.as_str()))
        .id_generator(|links, title, _| {
            links
                .first()
                .map(|link| format!("link:{}", link.href))
                .or_else(|| title.as_ref().map(|text| format!("title:{}", text.content)))
                .unwrap_or_default()
        })
        .build();
    let parsed = parser
        .parse(body)
        .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
    if parsed.entries.iter().any(|entry| entry.id.is_empty()) {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    if parsed.entries.len() > 500 {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }
    Ok(parsed)
}
async fn fetch_body_inner(
    start: &str,
    etag: Option<&str>,
    modified: Option<&str>,
) -> Result<ResponseBody, StatusCode> {
    let mut url = normalized_url(start)?;
    for redirects in 0..=5 {
        if !matches!(url.scheme(), "http" | "https")
            || !url.username().is_empty()
            || url.password().is_some()
            || url.host_str().is_none()
        {
            return Err(StatusCode::BAD_REQUEST);
        }
        let host = url.host_str().unwrap().trim_matches(['[', ']']).to_owned();
        let port = url.port_or_known_default().ok_or(StatusCode::BAD_REQUEST)?;
        let addresses: Vec<SocketAddr> = tokio::net::lookup_host((host.as_str(), port))
            .await
            .map_err(|_| StatusCode::BAD_GATEWAY)?
            .collect();
        if addresses.is_empty() || addresses.iter().any(|a| !public_ip(a.ip())) {
            return Err(StatusCode::BAD_REQUEST);
        }
        let client = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .resolve_to_addrs(&host, &addresses)
            .build()
            .map_err(err)?;
        let mut request = client.get(url.clone());
        if let Some(v) = etag {
            request = request.header(reqwest::header::IF_NONE_MATCH, v)
        }
        if let Some(v) = modified {
            request = request.header(reqwest::header::IF_MODIFIED_SINCE, v)
        }
        let response = request.send().await.map_err(|_| StatusCode::BAD_GATEWAY)?;
        if response.status().is_redirection()
            && response.status() != reqwest::StatusCode::NOT_MODIFIED
        {
            if redirects == 5 {
                return Err(StatusCode::BAD_GATEWAY);
            }
            let location = response
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|v| v.to_str().ok())
                .ok_or(StatusCode::BAD_GATEWAY)?;
            url = url.join(location).map_err(|_| StatusCode::BAD_GATEWAY)?;
            url.set_fragment(None);
            continue;
        }
        if response.status() == reqwest::StatusCode::NOT_MODIFIED {
            return Ok(ResponseBody {
                final_url: url,
                body: None,
                etag: None,
                modified: None,
            });
        }
        if !response.status().is_success() {
            return Err(StatusCode::BAD_GATEWAY);
        }
        if response
            .content_length()
            .is_some_and(|n| n > 2 * 1024 * 1024)
        {
            return Err(StatusCode::PAYLOAD_TOO_LARGE);
        }
        let etag = response
            .headers()
            .get(reqwest::header::ETAG)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
        let modified = response
            .headers()
            .get(reqwest::header::LAST_MODIFIED)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
        let mut stream = response.bytes_stream();
        let mut body = Vec::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|_| StatusCode::BAD_GATEWAY)?;
            if body.len() + chunk.len() > 2 * 1024 * 1024 {
                return Err(StatusCode::PAYLOAD_TOO_LARGE);
            }
            body.extend_from_slice(&chunk)
        }
        return Ok(ResponseBody {
            final_url: url,
            body: Some(body),
            etag,
            modified,
        });
    }
    Err(StatusCode::BAD_GATEWAY)
}
pub(super) fn plain(s: &str) -> String {
    html2text::from_read(s.as_bytes(), 100)
        .unwrap_or_default()
        .trim()
        .to_owned()
}
pub(super) fn store_entries(
    tx: &rusqlite::Transaction<'_>,
    id: i64,
    feed: &feed_rs::model::Feed,
) -> rusqlite::Result<()> {
    for e in &feed.entries {
        let title = e
            .title
            .as_ref()
            .map(|t| plain(&t.content))
            .unwrap_or_default();
        let content = e
            .content
            .as_ref()
            .and_then(|c| c.body.as_ref())
            .or_else(|| e.summary.as_ref().map(|s| &s.content))
            .map(|s| plain(s))
            .unwrap_or_default();
        let url = e.links.first().map(|l| l.href.as_str()).unwrap_or("");
        let published = e
            .published
            .or(e.updated)
            .map(|d| d.to_rfc3339())
            .unwrap_or_default();
        tx.execute("INSERT INTO articles(feed_id,external_id,title,content,url,published) VALUES (?,?,?,?,?,?) ON CONFLICT(feed_id,external_id) DO UPDATE SET title=excluded.title,content=excluded.content,url=excluded.url,published=excluded.published",params![id,e.id,title,content,url,published])?;
    }
    Ok(())
}
async fn create_feed(
    State(db): State<Db>,
    headers: HeaderMap,
    Json(input): Json<NewFeed>,
) -> Result<Json<Feed>, StatusCode> {
    if !origin_allowed(&headers) {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(Json(add_feed(db, &input.url, input.folder_id).await?))
}
pub(super) async fn add_feed(
    db: Db,
    input_url: &str,
    folder_id: Option<i64>,
) -> Result<Feed, StatusCode> {
    let url = normalized_url(input_url)?.to_string();
    {
        let db = db.lock().map_err(err)?;
        folder_exists(&db, folder_id)?;
        if db
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM feeds WHERE CASE WHEN instr(url,'#')>0 THEN substr(url,1,instr(url,'#')-1) ELSE url END =?)",
                [&url],
                |r| r.get::<_, bool>(0),
            )
            .map_err(err)?
        {
            return Err(StatusCode::CONFLICT);
        }
    }
    let result = fetch(&url, None, None).await?;
    let parsed = result.parsed.ok_or(StatusCode::BAD_GATEWAY)?;
    let title = parsed
        .title
        .as_ref()
        .map(|t| plain(&t.content))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| url.clone());
    let mut db = db.lock().map_err(err)?;
    let tx = db.transaction().map_err(err)?;
    folder_exists(&tx, folder_id)?;
    tx.execute(
        "INSERT INTO feeds(url,title,folder_id,etag,modified) VALUES (?,?,?,?,?)",
        params![url, title, folder_id, result.etag, result.modified],
    )
    .map_err(db_write_error)?;
    let id = tx.last_insert_rowid();
    store_entries(&tx, id, &parsed).map_err(err)?;
    tx.commit().map_err(err)?;
    get_feed(&db, id)
}
async fn refresh(
    State(db): State<Db>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<Json<Feed>, StatusCode> {
    if !origin_allowed(&headers) {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(Json(refresh_feed(db, id, false).await?))
}
async fn refresh_feed(db: Db, id: i64, scheduled: bool) -> Result<Feed, StatusCode> {
    let owner = super::discovery::random()?;
    let (url, etag, modified) = {
        let mut db = db.lock().map_err(err)?;
        let tx = db.transaction().map_err(err)?;
        let f = get_feed(&tx, id)?;
        if f.paused {
            return Err(StatusCode::CONFLICT);
        }
        if scheduled && !tx.query_row("SELECT EXISTS(SELECT 1 FROM feeds f, collection_settings s WHERE f.id=? AND s.id=1 AND s.enabled=1 AND f.paused=0 AND (f.last_checked IS NULL OR unixepoch(f.last_checked)<=unixepoch()-s.interval_seconds))", [id], |r| r.get::<_, bool>(0)).map_err(err)? { return Err(StatusCode::CONFLICT); }
        let changed = tx.execute("INSERT INTO refresh_leases(feed_id,owner,expires_at) VALUES (?1,?2,unixepoch()+60) ON CONFLICT(feed_id) DO UPDATE SET owner=excluded.owner,expires_at=excluded.expires_at WHERE refresh_leases.expires_at <= unixepoch()", params![id,owner]).map_err(err)?;
        if changed == 0 {
            return Err(StatusCode::CONFLICT);
        }
        let (e, m) = tx
            .query_row("SELECT etag,modified FROM feeds WHERE id=?", [id], |r| {
                Ok((
                    r.get::<_, Option<String>>(0)?,
                    r.get::<_, Option<String>>(1)?,
                ))
            })
            .map_err(err)?;
        tx.commit().map_err(err)?;
        (f.url, e, m)
    };
    let result = fetch(&url, etag.as_deref(), modified.as_deref()).await;
    let mut db = db.lock().map_err(err)?;
    let outcome = match result {
        Ok(result) => {
            let still_owned = db.query_row("SELECT EXISTS(SELECT 1 FROM refresh_leases WHERE feed_id=? AND owner=? AND expires_at>unixepoch())", params![id,owner], |r| r.get::<_,bool>(0)).map_err(err)?;
            if still_owned {
                apply_refresh(&mut db, id, result)
            } else {
                Err(StatusCode::CONFLICT)
            }
        }
        Err(status) => {
            db.execute("UPDATE feeds SET last_checked=datetime('now'),last_error=? WHERE id=? AND paused=0 AND EXISTS(SELECT 1 FROM refresh_leases WHERE feed_id=? AND owner=? AND expires_at>unixepoch())", params![format!("Refresh failed ({})", status.as_u16()),id,id,owner]).map_err(err)?;
            Err(status)
        }
    };
    db.execute(
        "DELETE FROM refresh_leases WHERE feed_id=? AND owner=?",
        params![id, owner],
    )
    .map_err(err)?;
    outcome
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Settings {
    enabled: bool,
    interval_seconds: i64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SettingsUpdate {
    enabled: Option<bool>,
    interval_seconds: Option<i64>,
}
fn read_settings(db: &Connection) -> Result<Settings, StatusCode> {
    db.query_row(
        "SELECT enabled,interval_seconds FROM collection_settings WHERE id=1",
        [],
        |r| {
            Ok(Settings {
                enabled: r.get(0)?,
                interval_seconds: r.get(1)?,
            })
        },
    )
    .map_err(err)
}
async fn settings(State(db): State<Db>) -> Result<Json<Settings>, StatusCode> {
    let db = db.lock().map_err(err)?;
    Ok(Json(read_settings(&db)?))
}
async fn update_settings(
    State(db): State<Db>,
    headers: HeaderMap,
    Json(input): Json<SettingsUpdate>,
) -> Result<Json<Settings>, StatusCode> {
    if !origin_allowed(&headers) {
        return Err(StatusCode::FORBIDDEN);
    }
    if input.enabled.is_none() && input.interval_seconds.is_none()
        || input
            .interval_seconds
            .is_some_and(|v| !(60..=86400).contains(&v))
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    let db = db.lock().map_err(err)?;
    db.execute("UPDATE collection_settings SET enabled=COALESCE(?1,enabled),interval_seconds=COALESCE(?2,interval_seconds) WHERE id=1", params![input.enabled,input.interval_seconds]).map_err(err)?;
    Ok(Json(read_settings(&db)?))
}
pub async fn worker(
    db: Db,
    once: bool,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        if *shutdown.borrow() {
            return Ok(());
        }
        let ids = {
            let db = db.lock().map_err(|_| "database lock poisoned")?;
            let mut stmt = db.prepare("SELECT f.id FROM feeds f,collection_settings s WHERE s.id=1 AND s.enabled=1 AND f.paused=0 AND (f.last_checked IS NULL OR unixepoch(f.last_checked) <= unixepoch()-s.interval_seconds) ORDER BY f.id LIMIT 100")?;
            stmt.query_map([], |r| r.get::<_, i64>(0))?
                .collect::<Result<Vec<_>, _>>()?
        };
        for id in ids {
            if *shutdown.borrow() {
                return Ok(());
            }
            let _ = refresh_feed(db.clone(), id, true).await;
        }
        if once {
            return Ok(());
        }
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(30)) => {},
            _ = shutdown.changed() => return Ok(()),
        }
    }
}

fn apply_refresh(db: &mut Connection, id: i64, result: Fetched) -> Result<Feed, StatusCode> {
    let tx = db.transaction().map_err(err)?;
    if get_feed(&tx, id)?.paused {
        return Err(StatusCode::CONFLICT);
    }
    if let Some(parsed) = result.parsed {
        store_entries(&tx, id, &parsed).map_err(err)?;
        tx.execute(
            "UPDATE feeds SET etag=?,modified=? WHERE id=?",
            params![result.etag, result.modified, id],
        )
        .map_err(err)?;
    }
    tx.execute(
        "UPDATE feeds SET last_checked=datetime('now'),last_error=NULL WHERE id=?",
        [id],
    )
    .map_err(err)?;
    tx.commit().map_err(err)?;
    get_feed(db, id)
}
pub fn routes() -> Router<Db> {
    Router::new()
        .route("/api/library", get(library))
        .route("/api/library/order", post(order_library))
        .route(
            "/api/collection/settings",
            get(settings).patch(update_settings),
        )
        .route("/api/feeds", post(create_feed))
        .route("/api/feeds/{id}", patch(edit_feed).delete(delete_feed))
        .route("/api/feeds/{id}/refresh", post(refresh))
        .route("/api/folders", post(create_folder))
        .route(
            "/api/folders/{id}",
            patch(edit_folder).delete(delete_folder),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn ordering_is_atomic_and_lifecycle_appends() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("db");
        let db = Arc::new(Mutex::new(open_db(&path, false).unwrap()));
        {
            let c = db.lock().unwrap();
            c.execute_batch("INSERT INTO folders(id,name) VALUES (1,'Z'),(2,'A'); INSERT INTO feeds(id,url,title,folder_id) VALUES (1,'https://a','A',1),(2,'https://b','B',1),(3,'https://c','C',NULL); INSERT INTO articles(id,title,content,url,published,is_read,is_starred,feed_id) VALUES (1,'a','','','',1,1,1); INSERT INTO article_visits VALUES (1,'today');").unwrap();
        }
        let parse = |s: &str| serde_json::from_str::<OrderInput>(s).unwrap();
        let original = r#"{"expected":{"folders":[1,2],"feeds":[{"id":1,"folder_id":1},{"id":2,"folder_id":1},{"id":3,"folder_id":null}]},"order":{"folders":[2,1],"feeds":[{"id":2,"folder_id":2},{"id":1,"folder_id":1},{"id":3,"folder_id":null}]}}"#;
        assert_eq!(
            order_library(State(db.clone()), HeaderMap::new(), Json(parse(original))).await,
            Ok(StatusCode::NO_CONTENT)
        );
        assert_eq!(
            order_library(State(db.clone()), HeaderMap::new(), Json(parse(original))).await,
            Err(StatusCode::CONFLICT)
        );
        let invalid = r#"{"expected":{"folders":[2,1],"feeds":[{"id":2,"folder_id":2},{"id":1,"folder_id":1},{"id":3,"folder_id":null}]},"order":{"folders":[2,1],"feeds":[{"id":2,"folder_id":99},{"id":1,"folder_id":1},{"id":3,"folder_id":null}]}}"#;
        assert_eq!(
            order_library(State(db.clone()), HeaderMap::new(), Json(parse(invalid))).await,
            Err(StatusCode::BAD_REQUEST)
        );
        {
            let c = db.lock().unwrap();
            c.execute(
                "INSERT INTO feeds(id,url,title,folder_id) VALUES (4,'https://d','D',2)",
                [],
            )
            .unwrap();
            assert_eq!(
                snapshot(&c)
                    .unwrap()
                    .feeds
                    .iter()
                    .map(|f| f.id)
                    .collect::<Vec<_>>(),
                vec![2, 4, 1, 3]
            );
            assert_eq!(c.query_row("SELECT is_read,is_starred,(SELECT visited_at FROM article_visits WHERE article_id=1) FROM articles WHERE id=1", [], |r| Ok((r.get::<_,i64>(0)?,r.get::<_,i64>(1)?,r.get::<_,String>(2)?))).unwrap(), (1,1,"today".into()));
        }
        delete_folder(State(db.clone()), Path(2), HeaderMap::new())
            .await
            .unwrap();
        assert_eq!(
            snapshot(&db.lock().unwrap())
                .unwrap()
                .feeds
                .iter()
                .map(|f| f.id)
                .collect::<Vec<_>>(),
            vec![1, 3, 2, 4]
        );
        drop(db);
        assert_eq!(
            snapshot(&open_db(&path, false).unwrap())
                .unwrap()
                .feeds
                .iter()
                .map(|f| f.id)
                .collect::<Vec<_>>(),
            vec![1, 3, 2, 4]
        );
    }
    #[tokio::test]
    async fn rejects_invalid_snapshots_and_hostile_origin_without_changes() {
        use serde_json::json;
        let dir = tempfile::tempdir().unwrap();
        let db = Arc::new(Mutex::new(open_db(&dir.path().join("db"), false).unwrap()));
        db.lock().unwrap().execute_batch("INSERT INTO folders(id,name) VALUES (1,'One'),(2,'Two'); INSERT INTO feeds(id,url,title,folder_id) VALUES (1,'https://a','A',1),(2,'https://b','B',NULL);").unwrap();
        let expected =
            json!({"folders":[1,2],"feeds":[{"id":1,"folder_id":1},{"id":2,"folder_id":null}]});
        let invalid_orders = [
            json!({"folders":[1,1],"feeds":[{"id":1,"folder_id":1},{"id":2,"folder_id":null}]}),
            json!({"folders":[1,2],"feeds":[{"id":1,"folder_id":1},{"id":1,"folder_id":null}]}),
            json!({"folders":[1],"feeds":[{"id":1,"folder_id":1},{"id":2,"folder_id":null}]}),
            json!({"folders":[1,2],"feeds":[{"id":1,"folder_id":1}]}),
            json!({"folders":[1,3],"feeds":[{"id":1,"folder_id":1},{"id":2,"folder_id":null}]}),
            json!({"folders":[1,2],"feeds":[{"id":1,"folder_id":1},{"id":99,"folder_id":null}]}),
            json!({"folders":[1,2],"feeds":[{"id":1,"folder_id":99},{"id":2,"folder_id":null}]}),
        ];
        for order in invalid_orders {
            let input =
                serde_json::from_value(json!({"expected": expected, "order": order})).unwrap();
            assert_eq!(
                order_library(State(db.clone()), HeaderMap::new(), Json(input)).await,
                Err(StatusCode::BAD_REQUEST)
            );
            assert_eq!(
                snapshot(&db.lock().unwrap()).unwrap(),
                serde_json::from_value::<Snapshot>(expected.clone()).unwrap()
            );
        }
        let mut headers = HeaderMap::new();
        headers.insert(header::HOST, "localhost:8080".parse().unwrap());
        headers.insert(header::ORIGIN, "https://hostile.example".parse().unwrap());
        let input = serde_json::from_value(json!({"expected":expected,"order":{"folders":[2,1],"feeds":[{"id":1,"folder_id":1},{"id":2,"folder_id":null}]}})).unwrap();
        assert_eq!(
            order_library(State(db.clone()), headers, Json(input)).await,
            Err(StatusCode::FORBIDDEN)
        );
        assert_eq!(
            snapshot(&db.lock().unwrap()).unwrap(),
            serde_json::from_value::<Snapshot>(expected).unwrap()
        );
    }
    #[tokio::test]
    async fn rename_preserves_position_and_edit_move_appends() {
        let dir = tempfile::tempdir().unwrap();
        let db = Arc::new(Mutex::new(open_db(&dir.path().join("db"), false).unwrap()));
        db.lock().unwrap().execute_batch("INSERT INTO folders(id,name) VALUES (1,'One'),(2,'Two'); INSERT INTO feeds(id,url,title,folder_id) VALUES (1,'https://a','A',1),(2,'https://b','B',2),(3,'https://c','C',2);").unwrap();
        let edit = |title: Option<&str>, folder_id| EditFeed {
            title: title.map(str::to_string),
            folder_id,
            paused: None,
        };
        let _ = edit_feed(
            State(db.clone()),
            Path(2),
            HeaderMap::new(),
            Json(edit(Some("Z"), None)),
        )
        .await
        .unwrap();
        assert_eq!(
            snapshot(&db.lock().unwrap())
                .unwrap()
                .feeds
                .iter()
                .map(|f| f.id)
                .collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        let _ = edit_feed(
            State(db.clone()),
            Path(1),
            HeaderMap::new(),
            Json(edit(None, Some(Some(2)))),
        )
        .await
        .unwrap();
        let result = snapshot(&db.lock().unwrap()).unwrap();
        assert_eq!(
            result
                .feeds
                .iter()
                .map(|f| (f.id, f.folder_id))
                .collect::<Vec<_>>(),
            vec![(2, Some(2)), (3, Some(2)), (1, Some(2))]
        );
    }
    #[test]
    fn v4_order_migration_preserves_visible_order_and_backup() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("db");
        let c = open_db(&path, false).unwrap();
        c.execute_batch("INSERT INTO folders(id,name) VALUES (1,'Z'),(2,'A'); INSERT INTO feeds(id,url,title,folder_id) VALUES (1,'https://a','Z',1),(2,'https://b','A',1),(3,'https://c','C',NULL); DROP TRIGGER folder_order_insert; DROP TRIGGER feed_order_insert; DROP TRIGGER feed_order_move; DROP TABLE feed_order; DROP TABLE folder_order; DROP TABLE reader_settings; PRAGMA user_version=4;").unwrap();
        drop(c);
        let c = open_db(&path, false).unwrap();
        let order = snapshot(&c).unwrap();
        assert_eq!(order.folders, vec![2, 1]);
        assert_eq!(
            order.feeds.iter().map(|f| f.id).collect::<Vec<_>>(),
            vec![2, 1, 3]
        );
        let backup = Connection::open(path.with_extension("v4-backup.sqlite3")).unwrap();
        assert_eq!(
            backup
                .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
                .unwrap(),
            4
        );
    }
    #[tokio::test]
    async fn fragments_do_not_create_duplicate_subscriptions() {
        assert_eq!(
            normalized_url("https://example.org/feed#one")
                .unwrap()
                .as_str(),
            "https://example.org/feed"
        );
        let dir = tempfile::tempdir().unwrap();
        let db = Arc::new(Mutex::new(open_db(&dir.path().join("db"), false).unwrap()));
        db.lock()
            .unwrap()
            .execute(
                "INSERT INTO feeds(url,title) VALUES ('https://example.org/feed#old','existing')",
                [],
            )
            .unwrap();
        assert!(matches!(
            add_feed(db, "https://example.org/feed#new", None).await,
            Err(StatusCode::CONFLICT)
        ));
    }
    #[tokio::test]
    async fn scheduled_claim_checks_due_disabled_paused_and_lease() {
        let dir = tempfile::tempdir().unwrap();
        let db = Arc::new(Mutex::new(open_db(&dir.path().join("db"), false).unwrap()));
        {
            let c = db.lock().unwrap();
            c.execute(
                "INSERT INTO feeds(url,title) VALUES ('http://127.0.0.1/','test')",
                [],
            )
            .unwrap();
        }
        assert!(matches!(
            refresh_feed(db.clone(), 1, true).await,
            Err(StatusCode::CONFLICT)
        ));
        {
            let c = db.lock().unwrap();
            c.execute("UPDATE collection_settings SET enabled=1", [])
                .unwrap();
            c.execute("UPDATE feeds SET last_checked=datetime('now')", [])
                .unwrap();
        }
        assert!(matches!(
            refresh_feed(db.clone(), 1, true).await,
            Err(StatusCode::CONFLICT)
        ));
        {
            let c = db.lock().unwrap();
            c.execute("UPDATE feeds SET last_checked=NULL,paused=1", [])
                .unwrap();
        }
        assert!(matches!(
            refresh_feed(db.clone(), 1, true).await,
            Err(StatusCode::CONFLICT)
        ));
        {
            let c = db.lock().unwrap();
            c.execute("UPDATE feeds SET paused=0", []).unwrap();
            c.execute(
                "INSERT INTO refresh_leases VALUES (1,'other',unixepoch()+60)",
                [],
            )
            .unwrap();
        }
        assert!(matches!(
            refresh_feed(db.clone(), 1, false).await,
            Err(StatusCode::CONFLICT)
        ));
        {
            let c = db.lock().unwrap();
            c.execute("UPDATE refresh_leases SET expires_at=unixepoch()-1", [])
                .unwrap();
        }
        assert!(refresh_feed(db.clone(), 1, false).await.is_err());
        assert_eq!(
            db.lock()
                .unwrap()
                .query_row("SELECT count(*) FROM refresh_leases", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
    #[test]
    fn blocks_special_addresses() {
        for ip in [
            "127.0.0.1",
            "10.0.0.1",
            "100.64.0.1",
            "169.254.169.254",
            "192.0.2.1",
            "198.18.0.1",
            "203.0.113.1",
            "224.0.0.1",
            "::1",
            "::ffff:8.8.8.8",
            "fc00::1",
            "fe80::1",
            "64:ff9b::808:808",
            "2001:db8::1",
            "2002::1",
            "2001::1",
        ] {
            assert!(!public_ip(ip.parse().unwrap()), "{ip}");
        }
        assert!(public_ip("8.8.8.8".parse().unwrap()));
        assert!(public_ip("2606:4700:4700::1111".parse().unwrap()));
    }
    #[test]
    fn parsed_entries_update_without_losing_state() {
        let mut db = Connection::open_in_memory().unwrap();
        db.execute(SCHEMA, []).unwrap();
        let tx = db.transaction().unwrap();
        migrate(&tx).unwrap();
        tx.execute(
            "INSERT INTO feeds(url,title) VALUES ('https://example.com/feed','Feed')",
            [],
        )
        .unwrap();
        let xml=b"<rss version='2.0'><channel><title>Example</title><link>https://example.com</link><description>x</description><item><guid>one</guid><title>First</title><description><![CDATA[<p>Hello <b>world</b></p>]]></description></item></channel></rss>";
        let parsed = feed_rs::parser::parse(xml.as_slice()).unwrap();
        store_entries(&tx, 1, &parsed).unwrap();
        tx.execute("UPDATE articles SET is_read=1,is_starred=1", [])
            .unwrap();
        store_entries(&tx, 1, &parsed).unwrap();
        let (count, read, star, content): (i64, bool, bool, String) = tx
            .query_row(
                "SELECT count(*),is_read,is_starred,content FROM articles",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap();
        assert_eq!(count, 1);
        assert!(read && star);
        assert!(content.contains("Hello"));
        assert!(!content.contains("<p>"));
    }
    #[test]
    fn not_modified_and_paused_refresh_preserve_data() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = open_db(&dir.path().join("db"), false).unwrap();
        db.execute("INSERT INTO feeds(url,title,etag,last_error) VALUES ('https://example.com','x','tag','old error')",[]).unwrap();
        db.execute("INSERT INTO articles(feed_id,external_id,title,content,url,published,is_read,is_starred) VALUES (1,'one','title','body','','',1,1)",[]).unwrap();
        let result = Fetched {
            parsed: None,
            etag: None,
            modified: None,
        };
        let feed = apply_refresh(&mut db, 1, result).unwrap();
        assert!(feed.last_checked.is_some());
        assert!(feed.last_error.is_none());
        assert_eq!(
            db.query_row("SELECT etag FROM feeds WHERE id=1", [], |r| r
                .get::<_, String>(0))
                .unwrap(),
            "tag"
        );
        assert!(lookup(&db, 1).unwrap().is_starred);
        db.execute("UPDATE feeds SET paused=1", []).unwrap();
        assert!(matches!(
            apply_refresh(
                &mut db,
                1,
                Fetched {
                    parsed: None,
                    etag: None,
                    modified: None
                }
            ),
            Err(StatusCode::CONFLICT)
        ));
        db.execute("DELETE FROM feeds", []).unwrap();
        assert!(matches!(
            apply_refresh(
                &mut db,
                1,
                Fetched {
                    parsed: None,
                    etag: None,
                    modified: None
                }
            ),
            Err(StatusCode::NOT_FOUND)
        ));
    }
    #[test]
    fn parses_atom_fixture() {
        let xml=b"<feed xmlns='http://www.w3.org/2005/Atom'><title>Example</title><id>https://example.com</id><updated>2026-01-01T00:00:00Z</updated><entry><title>Item</title><id>urn:one</id><updated>2026-01-01T00:00:00Z</updated></entry></feed>";
        assert_eq!(
            feed_rs::parser::parse(xml.as_slice())
                .unwrap()
                .entries
                .len(),
            1
        );
    }
    #[tokio::test]
    async fn worker_stops_when_shutdown_is_requested() {
        let dir = tempfile::tempdir().unwrap();
        let db = std::sync::Arc::new(std::sync::Mutex::new(
            super::super::open_db(&dir.path().join("worker.db"), false).unwrap(),
        ));
        let (tx, rx) = tokio::sync::watch::channel(false);
        let stop = async {
            tokio::task::yield_now().await;
            tx.send(true).unwrap();
        };
        let (result, ()) = tokio::join!(
            tokio::time::timeout(Duration::from_secs(1), worker(db.clone(), false, rx)),
            stop
        );
        result.unwrap().unwrap();
        let db = db.lock().unwrap();
        let leases: i64 = db
            .query_row("SELECT count(*) FROM refresh_leases", [], |r| r.get(0))
            .unwrap();
        assert_eq!(leases, 0);
    }
    #[tokio::test]
    async fn rejects_unsafe_targets() {
        for url in [
            "http://127.0.0.1/",
            "http://[::1]/",
            "http://user:pass@example.com/",
            "file:///etc/passwd",
        ] {
            assert!(fetch(url, None, None).await.is_err());
        }
    }
}
