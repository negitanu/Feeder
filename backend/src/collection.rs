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
const FEED_INTERVALS: &str = "CREATE TABLE feed_poll_intervals (feed_id INTEGER PRIMARY KEY REFERENCES feeds(id) ON DELETE CASCADE, interval_seconds INTEGER NOT NULL CHECK(interval_seconds BETWEEN 60 AND 86400))";
pub fn migrate_intervals(tx: &rusqlite::Transaction<'_>) -> rusqlite::Result<()> {
    tx.execute(FEED_INTERVALS, [])?;
    Ok(())
}
pub fn recognized_v8(objects: &[(String, String, Option<String>)]) -> bool {
    let mut previous = objects.to_vec();
    let Some(index) = previous.iter().position(|o| {
        o.0 == "table" && o.1 == "feed_poll_intervals" && o.2.as_deref() == Some(FEED_INTERVALS)
    }) else {
        return false;
    };
    previous.remove(index);
    auth::recognized_v7(&previous)
}
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
        tx.execute(
            "INSERT INTO collection_settings(id,enabled) VALUES (1,1)",
            [],
        )?;
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
    pub(super) id: i64,
    url: String,
    title: String,
    folder_id: Option<i64>,
    paused: bool,
    last_checked: Option<String>,
    last_error: Option<String>,
    poll_interval: Option<i64>,
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
        poll_interval: r.get(7)?,
    })
}
pub(super) fn get_feed(db: &Connection, id: i64) -> Result<Feed, StatusCode> {
    db.query_row(
        "SELECT id,url,title,folder_id,paused,last_checked,last_error,(SELECT interval_seconds FROM feed_poll_intervals WHERE feed_id=feeds.id) FROM feeds WHERE id=?",
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
    let feeds=tx.prepare("SELECT f.id,f.url,f.title,f.folder_id,f.paused,f.last_checked,f.last_error,(SELECT interval_seconds FROM feed_poll_intervals WHERE feed_id=f.id) FROM feeds f JOIN feed_order o ON o.feed_id=f.id LEFT JOIN folder_order fo ON fo.folder_id=f.folder_id ORDER BY f.folder_id IS NULL,fo.position,o.position,f.id").map_err(err)?.query_map([],feed).map_err(err)?.collect::<Result<Vec<_>,_>>().map_err(err)?;
    tx.commit().map_err(err)?;
    Ok(Json(Library { folders, feeds }))
}
#[derive(Serialize)]
struct UnreadCounts {
    total: i64,
    folders: Vec<FolderUnread>,
}
#[derive(Serialize)]
struct FolderUnread {
    id: i64,
    count: i64,
}
async fn unread_counts(State(db): State<Db>) -> Result<Json<UnreadCounts>, StatusCode> {
    let db = db.lock().map_err(err)?;
    let predicate = format!("articles.is_read=0 AND NOT ({})", mute_rules::HIDDEN_SQL);
    let total = db
        .query_row(
            &format!("SELECT count(*) FROM articles WHERE {predicate}"),
            [],
            |r| r.get(0),
        )
        .map_err(err)?;
    let folders = db.prepare(&format!("SELECT folders.id, count(articles.id) FROM folders LEFT JOIN feeds ON feeds.folder_id=folders.id LEFT JOIN articles ON articles.feed_id=feeds.id AND {predicate} GROUP BY folders.id")).map_err(err)?
        .query_map([], |r| Ok(FolderUnread { id: r.get(0)?, count: r.get(1)? })).map_err(err)?
        .collect::<Result<Vec<_>, _>>().map_err(err)?;
    Ok(Json(UnreadCounts { total, folders }))
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
    #[serde(default, deserialize_with = "optional_present_string")]
    url: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    folder_id: Option<Option<i64>>,
    paused: Option<bool>,
    #[serde(default, deserialize_with = "double_interval")]
    poll_interval: Option<Option<i64>>,
}
pub(super) fn double_interval<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Option<Option<i64>>, D::Error> {
    Option::<i64>::deserialize(d).map(Some)
}
pub(super) fn set_poll_interval(
    db: &Connection,
    id: i64,
    interval: Option<Option<i64>>,
) -> Result<(), StatusCode> {
    match interval {
        None => {}
        Some(None) => {
            db.execute("DELETE FROM feed_poll_intervals WHERE feed_id=?", [id])
                .map_err(err)?;
        }
        Some(Some(value)) if (60..=86400).contains(&value) => {
            db.execute("INSERT INTO feed_poll_intervals(feed_id,interval_seconds) VALUES (?,?) ON CONFLICT(feed_id) DO UPDATE SET interval_seconds=excluded.interval_seconds", params![id,value]).map_err(err)?;
        }
        Some(Some(_)) => return Err(StatusCode::BAD_REQUEST),
    }
    Ok(())
}
fn optional_present_string<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Option<String>, D::Error> {
    String::deserialize(d).map(Some)
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
pub(super) fn ensure_not_refreshing(db: &Connection, id: i64) -> Result<(), StatusCode> {
    get_feed(db, id)?;
    let active = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM refresh_leases WHERE feed_id=? AND expires_at>unixepoch())",
        [id], |r| r.get::<_, bool>(0),
    ).map_err(err)?;
    if active {
        Err(StatusCode::CONFLICT)
    } else {
        Ok(())
    }
}
pub(super) fn edit_url(db: &Connection, id: i64, input: Option<&str>) -> Result<(), StatusCode> {
    let Some(input) = input else {
        return Ok(());
    };
    if super::social::is_source(db, id)? {
        return Err(StatusCode::CONFLICT);
    }
    if input.len() > 8192 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let new_url = opml::safe_url(input).ok_or(StatusCode::BAD_REQUEST)?;
    let old_url: String = db
        .query_row("SELECT url FROM feeds WHERE id=?", [id], |r| r.get(0))
        .map_err(err)?;
    if opml::safe_url(&old_url).as_deref() == Some(new_url.as_str()) {
        return Ok(());
    }
    let mut stmt = db
        .prepare("SELECT url FROM feeds WHERE id<>?")
        .map_err(err)?;
    let urls = stmt
        .query_map([id], |r| r.get::<_, String>(0))
        .map_err(err)?;
    for value in urls {
        if opml::safe_url(&value.map_err(err)?).as_deref() == Some(new_url.as_str()) {
            return Err(StatusCode::CONFLICT);
        }
    }
    db.execute("UPDATE feeds SET url=?,etag=NULL,modified=NULL,last_checked=NULL,last_error=NULL WHERE id=?", params![new_url,id]).map_err(db_write_error)?;
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
    if input.title.is_none()
        && input.url.is_none()
        && input.folder_id.is_none()
        && input.paused.is_none()
        && input.poll_interval.is_none()
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut db = db.lock().map_err(err)?;
    let tx = db
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(err)?;
    ensure_not_refreshing(&tx, id)?;
    let old = get_feed(&tx, id)?;
    let title = match &input.title {
        Some(s) => clean_name(s)?,
        None => &old.title,
    };
    let folder = input.folder_id.unwrap_or(old.folder_id);
    folder_exists(&tx, folder)?;
    edit_url(&tx, id, input.url.as_deref())?;
    set_poll_interval(&tx, id, input.poll_interval)?;
    tx.execute(
        "UPDATE feeds SET title=?,folder_id=?,paused=? WHERE id=?",
        params![title, folder, input.paused.unwrap_or(old.paused), id],
    )
    .map_err(err)?;
    let result = get_feed(&tx, id)?;
    tx.commit().map_err(err)?;
    Ok(Json(result))
}
async fn delete_feed(
    State(db): State<Db>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<StatusCode, StatusCode> {
    if !origin_allowed(&headers) {
        return Err(StatusCode::FORBIDDEN);
    }
    let mut db = db.lock().map_err(err)?;
    let tx = db
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(err)?;
    ensure_not_refreshing(&tx, id)?;
    if tx
        .execute("DELETE FROM feeds WHERE id=?", [id])
        .map_err(err)?
        == 0
    {
        return Err(StatusCode::NOT_FOUND);
    }
    tx.commit().map_err(err)?;
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
tokio::task_local! { static RETRY_HINT: std::cell::Cell<Option<i64>>; }
tokio::task_local! { static FAILURE_DETAIL: std::cell::Cell<Option<FailureDetail>>; }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FailureDetail {
    Upstream(u16),
    Dns,
    Connection,
    Timeout,
    Body,
    Redirect,
    UnsafeTarget,
    TooLarge,
    InvalidFeed,
}
impl FailureDetail {
    fn code(self) -> String {
        match self {
            Self::Upstream(status) => format!("upstream_http_{status}"),
            Self::Dns => "dns".into(),
            Self::Connection => "connection".into(),
            Self::Timeout => "timeout".into(),
            Self::Body => "body".into(),
            Self::Redirect => "redirect".into(),
            Self::UnsafeTarget => "unsafe_target".into(),
            Self::TooLarge => "too_large".into(),
            Self::InvalidFeed => "invalid_feed".into(),
        }
    }
}
fn failure(detail: FailureDetail, status: StatusCode) -> StatusCode {
    let _ = FAILURE_DETAIL.try_with(|value| value.set(Some(detail)));
    status
}

fn record_retry_hint(status: StatusCode, headers: &HeaderMap, now: i64) {
    if matches!(status.as_u16(), 429 | 503)
        && let Some(value) = headers
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
        && let Some(deadline) = retry_after_deadline(value, now)
    {
        let _ = RETRY_HINT.try_with(|hint| hint.set(Some(deadline)));
    }
}

fn retry_after_deadline(value: &str, now: i64) -> Option<i64> {
    let value = value.trim();
    if !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()) {
        return Some(now.saturating_add(value.parse::<i64>().unwrap_or(i64::MAX)));
    }
    // Accept IMF-fixdate and the two obsolete HTTP-date forms required of recipients.
    let parts: Vec<_> = value.split_ascii_whitespace().collect();
    let db = Connection::open_in_memory().ok()?;
    let (day_text, month_text, year, time_text) = match parts.as_slice() {
        [weekday, day, month, year, time, "GMT"] if weekday.ends_with(',') => {
            (*day, *month, year.parse::<i32>().ok()?, *time)
        }
        [weekday, date, time, "GMT"] if weekday.ends_with(',') => {
            let date: Vec<_> = date.split('-').collect();
            if date.len() != 3 || date[2].len() != 2 {
                return None;
            }
            let short_year: i32 = date[2].parse().ok()?;
            let current_year: i32 = db
                .query_row(
                    "SELECT CAST(strftime('%Y',?,'unixepoch') AS INTEGER)",
                    [now],
                    |r| r.get(0),
                )
                .ok()?;
            let mut year = current_year / 100 * 100 + short_year;
            if year > current_year + 50 {
                year -= 100;
            }
            (date[0], date[1], year, *time)
        }
        [_weekday, month, day, time, year] => (*day, *month, year.parse::<i32>().ok()?, *time),
        _ => return None,
    };
    let day: u32 = day_text.parse().ok()?;
    let month = match month_text {
        "Jan" => 1,
        "Feb" => 2,
        "Mar" => 3,
        "Apr" => 4,
        "May" => 5,
        "Jun" => 6,
        "Jul" => 7,
        "Aug" => 8,
        "Sep" => 9,
        "Oct" => 10,
        "Nov" => 11,
        "Dec" => 12,
        _ => return None,
    };
    let days = match month {
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    if !(1..=days).contains(&day) || !(1970..=9999).contains(&year) {
        return None;
    }
    let t: Vec<_> = time_text.split(':').collect();
    if t.len() != 3 {
        return None;
    }
    let h: u32 = t[0].parse().ok()?;
    let m: u32 = t[1].parse().ok()?;
    let sec: u32 = t[2].parse().ok()?;
    if h > 23 || m > 59 || sec > 59 {
        return None;
    }
    let formatted = format!("{year:04}-{month:02}-{day:02} {h:02}:{m:02}:{sec:02}");
    let deadline: Option<i64> = db
        .query_row("SELECT unixepoch(?)", [formatted], |r| r.get(0))
        .ok()?;
    deadline.map(|d| d.max(now))
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
        .map_err(|_| failure(FailureDetail::Timeout, StatusCode::BAD_GATEWAY))?
}
pub(super) struct ResponseBody {
    pub final_url: Url,
    pub content_type: Option<String>,
    pub body: Option<Vec<u8>>,
    pub etag: Option<String>,
    pub modified: Option<String>,
}
pub(super) async fn fetch_body(start: &str) -> Result<ResponseBody, StatusCode> {
    tokio::time::timeout(
        Duration::from_secs(10),
        fetch_body_inner(start, None, None, 2 * 1024 * 1024),
    )
    .await
    .map_err(|_| failure(FailureDetail::Timeout, StatusCode::BAD_GATEWAY))?
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
    let response = fetch_body_inner(start, etag, modified, 8 * 1024 * 1024).await?;
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
        .map_err(|_| failure(FailureDetail::InvalidFeed, StatusCode::UNPROCESSABLE_ENTITY))?;
    if parsed.entries.iter().any(|entry| entry.id.is_empty()) {
        return Err(failure(
            FailureDetail::InvalidFeed,
            StatusCode::UNPROCESSABLE_ENTITY,
        ));
    }
    if parsed.entries.len() > 10_000 {
        return Err(failure(
            FailureDetail::TooLarge,
            StatusCode::PAYLOAD_TOO_LARGE,
        ));
    }
    Ok(parsed)
}
async fn host_permit(host: &str) -> Result<tokio::sync::OwnedSemaphorePermit, StatusCode> {
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex, OnceLock, Weak},
    };
    static HOSTS: OnceLock<Mutex<HashMap<String, Weak<tokio::sync::Semaphore>>>> = OnceLock::new();
    let semaphore = {
        let mut hosts = HOSTS
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
            .map_err(err)?;
        hosts.retain(|_, value| value.strong_count() > 0);
        if let Some(existing) = hosts.get(host).and_then(Weak::upgrade) {
            existing
        } else {
            let value = Arc::new(tokio::sync::Semaphore::new(1));
            hosts.insert(host.to_ascii_lowercase(), Arc::downgrade(&value));
            value
        }
    };
    semaphore.acquire_owned().await.map_err(err)
}
fn feed_http_client(host: &str, addresses: &[SocketAddr]) -> Result<reqwest::Client, StatusCode> {
    reqwest::Client::builder()
        .user_agent(concat!("Feeder/", env!("CARGO_PKG_VERSION")))
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .resolve_to_addrs(host, addresses)
        .build()
        .map_err(err)
}
async fn fetch_body_inner(
    start: &str,
    etag: Option<&str>,
    modified: Option<&str>,
    max_bytes: usize,
) -> Result<ResponseBody, StatusCode> {
    let mut url = normalized_url(start)?;
    for redirects in 0..=5 {
        if !matches!(url.scheme(), "http" | "https")
            || !url.username().is_empty()
            || url.password().is_some()
            || url.host_str().is_none()
        {
            return Err(failure(
                FailureDetail::UnsafeTarget,
                StatusCode::BAD_REQUEST,
            ));
        }
        let host = url.host_str().unwrap().trim_matches(['[', ']']).to_owned();
        // A permit covers DNS, the request and response body, including each redirect hop.
        // Weak references prevent the per-host registry from growing without bound.
        let permit = host_permit(&host).await?;
        let port = url.port_or_known_default().ok_or(StatusCode::BAD_REQUEST)?;
        let addresses: Vec<SocketAddr> = tokio::net::lookup_host((host.as_str(), port))
            .await
            .map_err(|_| failure(FailureDetail::Dns, StatusCode::BAD_GATEWAY))?
            .collect();
        if addresses.is_empty() || addresses.iter().any(|a| !public_ip(a.ip())) {
            return Err(failure(
                FailureDetail::UnsafeTarget,
                StatusCode::BAD_REQUEST,
            ));
        }
        let client = feed_http_client(&host, &addresses)?;
        let mut request = client.get(url.clone());
        if let Some(v) = etag {
            request = request.header(reqwest::header::IF_NONE_MATCH, v)
        }
        if let Some(v) = modified {
            request = request.header(reqwest::header::IF_MODIFIED_SINCE, v)
        }
        let response = request.send().await.map_err(|error| {
            failure(
                if error.is_timeout() {
                    FailureDetail::Timeout
                } else {
                    FailureDetail::Connection
                },
                StatusCode::BAD_GATEWAY,
            )
        })?;
        if response.status().is_redirection()
            && response.status() != reqwest::StatusCode::NOT_MODIFIED
        {
            if redirects == 5 {
                return Err(failure(FailureDetail::Redirect, StatusCode::BAD_GATEWAY));
            }
            let location = response
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|v| v.to_str().ok())
                .ok_or_else(|| failure(FailureDetail::Redirect, StatusCode::BAD_GATEWAY))?;
            url = url
                .join(location)
                .map_err(|_| failure(FailureDetail::Redirect, StatusCode::BAD_GATEWAY))?;
            url.set_fragment(None);
            drop(permit);
            continue;
        }
        if response.status() == reqwest::StatusCode::NOT_MODIFIED {
            return Ok(ResponseBody {
                final_url: url,
                content_type: None,
                body: None,
                etag: None,
                modified: None,
            });
        }
        if !response.status().is_success() {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs().min(i64::MAX as u64) as i64)
                .unwrap_or(0);
            record_retry_hint(response.status(), response.headers(), now);
            return Err(failure(
                FailureDetail::Upstream(response.status().as_u16()),
                StatusCode::BAD_GATEWAY,
            ));
        }
        if response
            .content_length()
            .is_some_and(|n| n > max_bytes as u64)
        {
            return Err(failure(
                FailureDetail::TooLarge,
                StatusCode::PAYLOAD_TOO_LARGE,
            ));
        }
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
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
            let chunk = chunk.map_err(|_| failure(FailureDetail::Body, StatusCode::BAD_GATEWAY))?;
            if body.len() + chunk.len() > max_bytes {
                return Err(failure(
                    FailureDetail::TooLarge,
                    StatusCode::PAYLOAD_TOO_LARGE,
                ));
            }
            body.extend_from_slice(&chunk)
        }
        return Ok(ResponseBody {
            final_url: url,
            content_type,
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
    context: super::collection_runtime::CollectionContext,
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
        let source_updated = e.updated.map(|date| date.to_rfc3339());
        super::article_revisions::store_entry(
            tx,
            id,
            &e.id,
            &title,
            &content,
            url,
            &published,
            source_updated.as_deref(),
            context,
        )?;
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
    add_feed_impl(db, input_url, folder_id, None, None).await
}
pub(super) async fn add_feed_in_folder(
    db: Db,
    input_url: &str,
    folder: &str,
) -> Result<Feed, StatusCode> {
    add_feed_impl(db, input_url, None, Some(folder), None).await
}
pub(super) async fn add_named_feed(
    db: Db,
    input_url: &str,
    folder_id: Option<i64>,
    title: Option<&str>,
) -> Result<Feed, StatusCode> {
    add_feed_impl(db, input_url, folder_id, None, title).await
}
async fn add_feed_impl(
    db: Db,
    input_url: &str,
    folder_id: Option<i64>,
    folder_name: Option<&str>,
    custom_title: Option<&str>,
) -> Result<Feed, StatusCode> {
    let custom_title = custom_title
        .filter(|s| !s.trim().is_empty())
        .map(clean_name)
        .transpose()?;
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
    let title = custom_title.unwrap_or(&title);
    let mut db = db.lock().map_err(err)?;
    let tx = db.transaction().map_err(err)?;
    folder_exists(&tx, folder_id)?;
    let folder_id = if let Some(name) = folder_name.filter(|name| !name.is_empty()) {
        tx.execute(
            "INSERT INTO folders(name) VALUES (?) ON CONFLICT(name) DO NOTHING",
            [name],
        )
        .map_err(err)?;
        Some(
            tx.query_row("SELECT id FROM folders WHERE name=?", [name], |r| r.get(0))
                .map_err(err)?,
        )
    } else {
        folder_id
    };
    tx.execute(
        "INSERT INTO feeds(url,title,folder_id,etag,modified) VALUES (?,?,?,?,?)",
        params![url, title, folder_id, result.etag, result.modified],
    )
    .map_err(db_write_error)?;
    let id = tx.last_insert_rowid();
    store_entries(
        &tx,
        id,
        &parsed,
        super::collection_runtime::CollectionContext::InitialBackfill,
    )
    .map_err(err)?;
    tx.execute(
        "UPDATE feeds SET last_checked=datetime('now') WHERE id=?",
        [id],
    )
    .map_err(err)?;
    super::collection_health::record_attempt(&tx, id, true).map_err(err)?;
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
pub(super) async fn refresh_feed(db: Db, id: i64, scheduled: bool) -> Result<Feed, StatusCode> {
    let owner = super::discovery::random()?;
    let (url, etag, modified, social, checkpoint) = {
        let mut db = db.lock().map_err(err)?;
        let tx = db
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(err)?;
        let f = get_feed(&tx, id)?;
        if f.paused {
            return Err(StatusCode::CONFLICT);
        }
        if runtime_table_exists(&tx).map_err(err)? && tx.query_row("SELECT EXISTS(SELECT 1 FROM collection_runtime WHERE feed_id=? AND next_retry>unixepoch())", [id], |r| r.get::<_,bool>(0)).map_err(err)? {
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }
        if scheduled {
            let pending = if runtime_table_exists(&tx).map_err(err)? {
                tx.query_row("SELECT EXISTS(SELECT 1 FROM collection_runtime WHERE feed_id=? AND more_pending=1)", [id], |r| r.get::<_,bool>(0)).map_err(err)?
            } else {
                false
            };
            let due = tx.query_row("SELECT EXISTS(SELECT 1 FROM feeds f, collection_settings s WHERE f.id=? AND s.id=1 AND s.enabled=1 AND f.paused=0 AND (? OR f.last_checked IS NULL OR unixepoch(f.last_checked)<=unixepoch()-COALESCE((SELECT interval_seconds FROM feed_poll_intervals WHERE feed_id=f.id),s.interval_seconds)))", params![id,pending], |r| r.get::<_,bool>(0)).map_err(err)?;
            if !due {
                return Err(StatusCode::CONFLICT);
            }
        }
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
        let social = super::social::source_for_feed(&tx, id)?;
        let mut checkpoint = if runtime_table_exists(&tx).map_err(err)? {
            super::collection_runtime::load(&tx, id).map_err(err)?
        } else {
            super::social::Checkpoint::default()
        };
        if social.is_some()
            && runtime_table_exists(&tx).map_err(err)?
            && matches!(
                super::collection_runtime::collection_context(&tx, id).map_err(err)?,
                super::collection_runtime::CollectionContext::RecoveryBackfill
            )
        {
            checkpoint.recovery = true;
        }
        tx.commit().map_err(err)?;
        (f.url, e, m, social, checkpoint)
    };
    let (result, retry_hint, detail) = FAILURE_DETAIL
        .scope(
            std::cell::Cell::new(None),
            RETRY_HINT.scope(std::cell::Cell::new(None), async {
                let result = if let Some(source) = social {
                    super::social::fetch_pages(source, checkpoint)
                        .await
                        .map(|(f, s)| (f, Some(s)))
                } else {
                    fetch(&url, etag.as_deref(), modified.as_deref())
                        .await
                        .map(|f| (f, None))
                };
                (
                    result,
                    RETRY_HINT.with(|hint| hint.get()),
                    FAILURE_DETAIL.with(|value| value.get()),
                )
            }),
        )
        .await;
    let mut db = db.lock().map_err(err)?;
    let outcome = match result {
        Ok((result, checkpoint)) => {
            apply_refresh_with_checkpoint(&mut db, id, result, Some(&owner), checkpoint)
        }
        Err(status) => {
            apply_refresh_failure_with_detail(&mut db, id, &owner, status, retry_hint, detail)?;
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
fn runtime_table_exists(db: &Connection) -> rusqlite::Result<bool> {
    db.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='collection_runtime')", [], |r| r.get(0))
}
fn due_feed_ids(db: &Connection) -> rusqlite::Result<Vec<i64>> {
    let sql = if runtime_table_exists(db)? {
        "SELECT f.id FROM feeds f JOIN collection_settings s ON s.id=1 LEFT JOIN collection_runtime r ON r.feed_id=f.id WHERE s.enabled=1 AND f.paused=0 AND COALESCE(r.next_retry,0)<=unixepoch() AND (COALESCE(r.more_pending,0)=1 OR f.last_checked IS NULL OR unixepoch(f.last_checked)<=unixepoch()-COALESCE((SELECT interval_seconds FROM feed_poll_intervals WHERE feed_id=f.id),s.interval_seconds)) ORDER BY f.id LIMIT 100"
    } else {
        "SELECT f.id FROM feeds f,collection_settings s WHERE s.id=1 AND s.enabled=1 AND f.paused=0 AND (f.last_checked IS NULL OR unixepoch(f.last_checked)<=unixepoch()-COALESCE((SELECT interval_seconds FROM feed_poll_intervals WHERE feed_id=f.id),s.interval_seconds)) ORDER BY f.id LIMIT 100"
    };
    let mut stmt = db.prepare(sql)?;
    stmt.query_map([], |r| r.get(0))?.collect()
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
            due_feed_ids(&db)?
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

#[cfg(test)]
fn apply_refresh_failure(
    db: &mut Connection,
    id: i64,
    owner: &str,
    status: StatusCode,
) -> Result<(), StatusCode> {
    apply_refresh_failure_with_retry(db, id, owner, status, None)
}
#[cfg(test)]
fn apply_refresh_failure_with_retry(
    db: &mut Connection,
    id: i64,
    owner: &str,
    status: StatusCode,
    retry_hint: Option<i64>,
) -> Result<(), StatusCode> {
    apply_refresh_failure_with_detail(db, id, owner, status, retry_hint, None)
}
fn apply_refresh_failure_with_detail(
    db: &mut Connection,
    id: i64,
    owner: &str,
    status: StatusCode,
    retry_hint: Option<i64>,
    detail: Option<FailureDetail>,
) -> Result<(), StatusCode> {
    let tx = db
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(err)?;
    let message = format!(
        "Refresh failed ({}){}",
        status.as_u16(),
        detail
            .map(|d| format!("; diagnostic={}", d.code()))
            .unwrap_or_default()
    );
    let changed = tx.execute("UPDATE feeds SET last_checked=datetime('now'),last_error=? WHERE id=? AND paused=0 AND EXISTS(SELECT 1 FROM refresh_leases WHERE feed_id=? AND owner=? AND expires_at>unixepoch())", params![message,id,id,owner]).map_err(err)?;
    if changed == 1 {
        super::collection_health::record_attempt(&tx, id, false).map_err(err)?;
        if runtime_table_exists(&tx).map_err(err)? {
            let failures: i64 = tx
                .query_row(
                    "SELECT consecutive_failures FROM collection_health WHERE feed_id=?",
                    [id],
                    |r| r.get(0),
                )
                .map_err(err)?;
            let delay = super::collection_runtime::backoff_seconds(
                u32::try_from(failures).unwrap_or(u32::MAX),
            );
            let now: i64 = tx
                .query_row("SELECT unixepoch()", [], |r| r.get(0))
                .map_err(err)?;
            let mut runtime = super::collection_runtime::load(&tx, id).map_err(err)?;
            runtime.next_retry = Some(now.saturating_add(delay).max(retry_hint.unwrap_or(0)));
            super::collection_runtime::save(&tx, id, &runtime).map_err(err)?;
        }
    }
    tx.commit().map_err(err)?;
    Ok(())
}
#[cfg(test)]
fn apply_refresh(
    db: &mut Connection,
    id: i64,
    result: Fetched,
    owner: Option<&str>,
) -> Result<Feed, StatusCode> {
    apply_refresh_with_checkpoint(db, id, result, owner, None)
}
fn apply_refresh_with_checkpoint(
    db: &mut Connection,
    id: i64,
    result: Fetched,
    owner: Option<&str>,
    checkpoint: Option<super::social::Checkpoint>,
) -> Result<Feed, StatusCode> {
    let tx = db
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(err)?;
    if let Some(owner) = owner {
        let owned = tx.query_row("SELECT EXISTS(SELECT 1 FROM refresh_leases WHERE feed_id=? AND owner=? AND expires_at>unixepoch())", params![id,owner], |r| r.get::<_,bool>(0)).map_err(err)?;
        if !owned {
            return Err(StatusCode::CONFLICT);
        }
    }
    if get_feed(&tx, id)?.paused {
        return Err(StatusCode::CONFLICT);
    }
    if let Some(parsed) = result.parsed {
        let mut context = super::collection_runtime::collection_context(&tx, id).map_err(err)?;
        if context != super::collection_runtime::CollectionContext::InitialBackfill
            && checkpoint.as_ref().is_some_and(|state| state.recovery)
        {
            context = super::collection_runtime::CollectionContext::RecoveryBackfill;
        }
        store_entries(&tx, id, &parsed, context).map_err(err)?;
        tx.execute(
            "UPDATE feeds SET etag=?,modified=? WHERE id=?",
            params![result.etag, result.modified, id],
        )
        .map_err(err)?;
    }
    if let Some(mut state) = checkpoint
        && runtime_table_exists(&tx).map_err(err)?
    {
        if !state.more_pending {
            state.recovery = false;
        }
        state.next_retry = None;
        super::collection_runtime::save(&tx, id, &state).map_err(err)?;
    }
    tx.execute(
        "UPDATE feeds SET last_checked=datetime('now'),last_error=NULL WHERE id=?",
        [id],
    )
    .map_err(err)?;
    if owner.is_some() {
        super::collection_health::record_attempt(&tx, id, true).map_err(err)?;
        if runtime_table_exists(&tx).map_err(err)? {
            tx.execute(
                "UPDATE collection_runtime SET next_retry=NULL WHERE feed_id=?",
                [id],
            )
            .map_err(err)?;
        }
    }
    tx.commit().map_err(err)?;
    get_feed(db, id)
}
pub fn routes() -> Router<Db> {
    Router::new()
        .route("/api/library", get(library))
        .route("/api/library/unread", get(unread_counts))
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
    async fn failure_diagnostics_are_isolated_and_bounded() {
        let first = FAILURE_DETAIL.scope(std::cell::Cell::new(None), async {
            assert_eq!(
                failure(FailureDetail::Upstream(403), StatusCode::BAD_GATEWAY),
                StatusCode::BAD_GATEWAY
            );
            tokio::task::yield_now().await;
            FAILURE_DETAIL.with(|detail| detail.get())
        });
        let second = FAILURE_DETAIL.scope(std::cell::Cell::new(None), async {
            let url = Url::parse("https://example.test/rss").unwrap();
            assert!(parse_feed(b"not a feed", &url).is_err());
            tokio::task::yield_now().await;
            FAILURE_DETAIL.with(|detail| detail.get())
        });
        let (first, second) = tokio::join!(first, second);
        assert_eq!(first, Some(FailureDetail::Upstream(403)));
        assert_eq!(second, Some(FailureDetail::InvalidFeed));
        assert_eq!(first.unwrap().code(), "upstream_http_403");
    }
    #[tokio::test]
    async fn feed_requests_identify_the_application() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut buffer = [0; 4096];
            let len = socket.read(&mut buffer).unwrap();
            socket
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
                .unwrap();
            String::from_utf8(buffer[..len].to_vec()).unwrap()
        });
        let client = feed_http_client("fixture.test", &[address]).unwrap();
        client
            .get(format!("http://fixture.test:{}/", address.port()))
            .send()
            .await
            .unwrap();
        let headers = server.join().unwrap().to_ascii_lowercase();
        assert!(headers.contains(&format!(
            "user-agent: feeder/{}\r\n",
            env!("CARGO_PKG_VERSION")
        )));
    }
    #[test]
    fn large_rss_is_bounded_by_entry_count() {
        let make_feed = |count| {
            let mut xml = String::from(
                "<rss version='2.0'><channel><title>Fixture</title><link>https://example.test/</link><description>Fixture</description>",
            );
            for i in 0..count {
                xml.push_str(&format!("<item><guid>{i}</guid><title>Item {i}</title><link>https://example.test/{i}</link></item>"));
            }
            xml.push_str("</channel></rss>");
            xml
        };
        let url = Url::parse("https://example.test/rss").unwrap();
        assert_eq!(
            parse_feed(make_feed(5_500).as_bytes(), &url)
                .unwrap()
                .entries
                .len(),
            5_500
        );
        assert_eq!(
            parse_feed(make_feed(10_001).as_bytes(), &url).unwrap_err(),
            StatusCode::PAYLOAD_TOO_LARGE
        );
    }
    #[tokio::test]
    async fn unread_counts_follow_membership_and_mute_rules() {
        let dir = tempfile::tempdir().unwrap();
        let db = Arc::new(Mutex::new(open_db(&dir.path().join("db"), false).unwrap()));
        db.lock().unwrap().execute_batch("INSERT INTO folders(id,name) VALUES(1,'One'),(2,'Empty'); INSERT INTO feeds(id,url,title,folder_id) VALUES(1,'https://a','A',1),(2,'https://b','B',NULL); INSERT INTO articles(id,title,content,url,published,is_read,feed_id) VALUES(1,'visible','','','',0,1),(2,'hidden','','','',0,1),(3,'read','','','',1,1),(4,'unfiled','','','',0,2); INSERT INTO mute_rules(name,term,field,enabled,revision) VALUES('Mute','hidden','title',1,1);").unwrap();
        let counts = unread_counts(State(db.clone())).await.unwrap().0;
        assert_eq!(counts.total, 2);
        assert_eq!(
            counts
                .folders
                .iter()
                .map(|f| (f.id, f.count))
                .collect::<Vec<_>>(),
            vec![(1, 1), (2, 0)]
        );
        db.lock().unwrap().execute_batch("UPDATE articles SET is_read=1 WHERE id=1; UPDATE feeds SET folder_id=2 WHERE id=2;").unwrap();
        let counts = unread_counts(State(db)).await.unwrap().0;
        assert_eq!(counts.total, 1);
        assert_eq!(
            counts
                .folders
                .iter()
                .map(|f| (f.id, f.count))
                .collect::<Vec<_>>(),
            vec![(1, 0), (2, 1)]
        );
    }
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
            url: None,
            poll_interval: None,
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
        super::super::extensions::drop_for_legacy_fixture(&c);
        c.execute_batch("INSERT INTO folders(id,name) VALUES (1,'Z'),(2,'A'); INSERT INTO feeds(id,url,title,folder_id) VALUES (1,'https://a','Z',1),(2,'https://b','A',1),(3,'https://c','C',NULL); DROP TRIGGER folder_order_insert; DROP TRIGGER feed_order_insert; DROP TRIGGER feed_order_move; DROP TABLE feed_order; DROP TABLE folder_order; DROP TABLE feed_poll_intervals; DROP TABLE api_keys; DROP TABLE api_rate_settings; DROP TABLE reader_settings; PRAGMA user_version=4;").unwrap();
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
    async fn per_feed_intervals_control_candidate_and_claim() {
        let dir = tempfile::tempdir().unwrap();
        let db = Arc::new(Mutex::new(open_db(&dir.path().join("db"), false).unwrap()));
        {
            let c = db.lock().unwrap();
            c.execute_batch("INSERT INTO feeds(id,url,title,last_checked) VALUES(1,'https://example.org/a','A',datetime('now','-5 minutes')),(2,'https://example.org/b','B',datetime('now','-5 minutes')),(3,'https://example.org/c','C',NULL); UPDATE collection_settings SET enabled=1,interval_seconds=900; INSERT INTO feed_poll_intervals VALUES(1,60),(2,3600); UPDATE feeds SET paused=1 WHERE id=3;").unwrap();
            assert_eq!(due_feed_ids(&c).unwrap(), vec![1]);
            c.execute(
                "UPDATE feed_poll_intervals SET interval_seconds=3600 WHERE feed_id=1",
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
            assert!(due_feed_ids(&c).unwrap().is_empty());
            c.execute("DELETE FROM feed_poll_intervals WHERE feed_id=1", [])
                .unwrap();
            c.execute("UPDATE collection_settings SET interval_seconds=60", [])
                .unwrap();
            assert_eq!(due_feed_ids(&c).unwrap(), vec![1]);
            c.execute("UPDATE collection_settings SET enabled=0", [])
                .unwrap();
            assert!(due_feed_ids(&c).unwrap().is_empty());
        }
    }
    #[test]
    fn v7_interval_upgrade_keeps_backup_and_data() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("db.sqlite3");
        let c = open_db(&path, false).unwrap();
        super::super::extensions::drop_for_legacy_fixture(&c);
        c.execute_batch("INSERT INTO feeds(id,url,title) VALUES(1,'https://example.org/feed','Saved'); INSERT INTO articles(id,title,content,url,published,feed_id,is_read,is_starred) VALUES(1,'saved','body','https://example.org/a','2026',1,1,1); INSERT INTO article_visits VALUES(1,'2026-01-01'); INSERT INTO api_keys(name,prefix,token_hash,scope,expires_at) VALUES('synthetic','prefix','synthetic-hash','read','2099-01-01'); UPDATE reader_settings SET default_view='starred'; DROP TABLE feed_poll_intervals; PRAGMA user_version=7;").unwrap();
        drop(c);
        let backup = path.with_extension("v7-backup.sqlite3");
        std::fs::write(&backup, b"do not overwrite").unwrap();
        assert!(open_db(&path, false).is_err());
        assert_eq!(std::fs::read(&backup).unwrap(), b"do not overwrite");
        std::fs::remove_file(&backup).unwrap();
        let c = open_db(&path, false).unwrap();
        assert_eq!(
            c.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
                .unwrap(),
            10
        );
        assert!(lookup(&c, 1).unwrap().is_starred);
        assert_eq!(
            c.query_row("SELECT count(*) FROM article_visits", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            c.query_row("SELECT count(*) FROM feed_poll_intervals", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert_eq!(
            c.query_row(
                "SELECT token_hash FROM api_keys WHERE name='synthetic'",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            "synthetic-hash"
        );
        assert_eq!(
            c.query_row("SELECT count(*) FROM feed_order WHERE feed_id=1", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            c.query_row("SELECT default_view FROM reader_settings", [], |r| r
                .get::<_, String>(0))
                .unwrap(),
            "starred"
        );
        let old = Connection::open(&backup).unwrap();
        assert_eq!(
            old.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
                .unwrap(),
            7
        );
        drop(c);
        assert!(open_db(&path, false).is_ok());
    }
    #[tokio::test]
    async fn scheduled_claim_checks_due_disabled_paused_and_lease() {
        let dir = tempfile::tempdir().unwrap();
        let db = Arc::new(Mutex::new(open_db(&dir.path().join("db"), false).unwrap()));
        {
            let c = db.lock().unwrap();
            c.execute("UPDATE collection_settings SET enabled=0", [])
                .unwrap();
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
        super::article_revisions::migrate_catchup(&tx).unwrap();
        super::story_groups::migrate_catchup(&tx).unwrap();
        tx.execute(
            "INSERT INTO feeds(url,title) VALUES ('https://example.com/feed','Feed')",
            [],
        )
        .unwrap();
        let xml=b"<rss version='2.0'><channel><title>Example</title><link>https://example.com</link><description>x</description><item><guid>one</guid><title>First</title><description><![CDATA[<p>Hello <b>world</b></p>]]></description></item></channel></rss>";
        let parsed = feed_rs::parser::parse(xml.as_slice()).unwrap();
        store_entries(
            &tx,
            1,
            &parsed,
            super::collection_runtime::CollectionContext::InitialBackfill,
        )
        .unwrap();
        tx.execute("UPDATE articles SET is_read=1,is_starred=1", [])
            .unwrap();
        store_entries(
            &tx,
            1,
            &parsed,
            super::collection_runtime::CollectionContext::Normal,
        )
        .unwrap();
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
        let feed = apply_refresh(&mut db, 1, result, None).unwrap();
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
                },
                None
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
                },
                None
            ),
            Err(StatusCode::NOT_FOUND)
        ));
    }
    #[test]
    fn health_tracks_only_accepted_lease_completions() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = open_db(&dir.path().join("db"), false).unwrap();
        db.execute_batch("INSERT INTO feeds(id,url,title) VALUES(1,'https://example.org/feed','test'); INSERT INTO refresh_leases VALUES(1,'owner',unixepoch()+60);").unwrap();
        apply_refresh_failure(&mut db, 1, "owner", StatusCode::BAD_GATEWAY).unwrap();
        apply_refresh_failure(&mut db, 1, "stale", StatusCode::BAD_GATEWAY).unwrap();
        assert_eq!(
            db.query_row(
                "SELECT consecutive_failures FROM collection_health WHERE feed_id=1",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            1
        );
        assert_eq!(
            db.query_row("SELECT last_error FROM feeds WHERE id=1", [], |r| r
                .get::<_, String>(0))
                .unwrap(),
            "Refresh failed (502)"
        );
        apply_refresh(
            &mut db,
            1,
            Fetched {
                parsed: None,
                etag: None,
                modified: None,
            },
            Some("owner"),
        )
        .unwrap();
        let health: (i64, bool) = db.query_row("SELECT consecutive_failures,last_success IS NOT NULL FROM collection_health WHERE feed_id=1", [], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
        assert_eq!(health, (0, true));
        db.execute("UPDATE feeds SET paused=1 WHERE id=1", [])
            .unwrap();
        apply_refresh_failure(&mut db, 1, "owner", StatusCode::BAD_GATEWAY).unwrap();
        assert!(
            apply_refresh(
                &mut db,
                1,
                Fetched {
                    parsed: None,
                    etag: None,
                    modified: None
                },
                Some("owner")
            )
            .is_err()
        );
        assert_eq!(
            db.query_row(
                "SELECT consecutive_failures FROM collection_health",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
        db.execute("DELETE FROM feeds WHERE id=1", []).unwrap();
        apply_refresh_failure(&mut db, 1, "owner", StatusCode::BAD_GATEWAY).unwrap();
        assert_eq!(
            db.query_row("SELECT count(*) FROM collection_health", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
    #[test]
    fn expired_refresh_cannot_apply_after_url_change() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = open_db(&dir.path().join("db"), false).unwrap();
        db.execute_batch("INSERT INTO feeds(id,url,title) VALUES(1,'https://example.org/new','saved'); INSERT INTO refresh_leases(feed_id,owner,expires_at) VALUES(1,'old-worker',unixepoch()-1);").unwrap();
        let parsed = feed_rs::parser::parse(&b"<feed xmlns='http://www.w3.org/2005/Atom'><title>Old</title><id>urn:old</id><updated>2026-01-01T00:00:00Z</updated><entry><id>urn:stale</id><title>Stale</title><updated>2026-01-01T00:00:00Z</updated></entry></feed>"[..]).unwrap();
        let result = Fetched {
            parsed: Some(parsed),
            etag: Some("stale-etag".into()),
            modified: None,
        };
        assert!(matches!(
            apply_refresh(&mut db, 1, result, Some("old-worker")),
            Err(StatusCode::CONFLICT)
        ));
        assert_eq!(
            db.query_row("SELECT count(*) FROM articles", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            0
        );
        let (url, etag): (String, Option<String>) = db
            .query_row("SELECT url,etag FROM feeds WHERE id=1", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!(url, "https://example.org/new");
        assert_eq!(etag, None);
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
    #[test]
    fn retry_after_seconds_and_http_dates() {
        assert_eq!(retry_after_deadline("120", 1000), Some(1120));
        assert_eq!(
            retry_after_deadline("Sun, 06 Nov 1994 08:49:37 GMT", 0),
            Some(784111777)
        );
        assert_eq!(
            retry_after_deadline("Sun, 31 Feb 2024 08:49:37 GMT", 0),
            None
        );
        assert_eq!(
            retry_after_deadline("Sunday, 06-Nov-94 08:49:37 GMT", 784000000),
            Some(784111777)
        );
        assert_eq!(
            retry_after_deadline("Sun Nov  6 08:49:37 1994", 0),
            Some(784111777)
        );
        assert_eq!(retry_after_deadline("invalid", 0), None);
        assert_eq!(retry_after_deadline("1", i64::MAX), Some(i64::MAX));
    }

    #[tokio::test]
    async fn host_requests_serialize_without_blocking_other_hosts() {
        let first = host_permit("same.example").await.unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(20), host_permit("same.example"))
                .await
                .is_err()
        );
        assert!(
            tokio::time::timeout(Duration::from_millis(20), host_permit("other.example"))
                .await
                .is_ok()
        );
        drop(first);
        assert!(
            tokio::time::timeout(Duration::from_millis(20), host_permit("same.example"))
                .await
                .is_ok()
        );
    }

    #[tokio::test]
    async fn retry_deadline_and_stale_lease_are_enforced() {
        let dir = tempfile::tempdir().unwrap();
        let db = Arc::new(Mutex::new(open_db(&dir.path().join("db"), false).unwrap()));
        {
            let mut c = db.lock().unwrap();
            c.execute_batch("INSERT INTO feeds(id,url,title) VALUES(1,'https://example.invalid/feed','Example'); INSERT INTO refresh_leases VALUES(1,'owner',unixepoch()+60);").unwrap();
            let deadline: i64 = c
                .query_row("SELECT unixepoch()+3600", [], |r| r.get(0))
                .unwrap();
            apply_refresh_failure_with_retry(
                &mut c,
                1,
                "stale",
                StatusCode::BAD_GATEWAY,
                Some(deadline),
            )
            .unwrap();
            assert_eq!(
                c.query_row("SELECT count(*) FROM collection_runtime", [], |r| r
                    .get::<_, i64>(0))
                    .unwrap(),
                0
            );
            apply_refresh_failure_with_detail(
                &mut c,
                1,
                "owner",
                StatusCode::BAD_GATEWAY,
                Some(deadline),
                Some(FailureDetail::Upstream(429)),
            )
            .unwrap();
            assert_eq!(
                c.query_row("SELECT last_error FROM feeds WHERE id=1", [], |r| r
                    .get::<_, String>(0))
                    .unwrap(),
                "Refresh failed (502); diagnostic=upstream_http_429"
            );
            assert_eq!(
                c.query_row(
                    "SELECT next_retry FROM collection_runtime WHERE feed_id=1",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
                deadline
            );
            assert!(due_feed_ids(&c).unwrap().is_empty());
        }
        assert_eq!(
            refresh_feed(db, 1, false).await.err(),
            Some(StatusCode::TOO_MANY_REQUESTS)
        );
    }
    #[test]
    fn stale_success_cannot_commit_social_checkpoint() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = open_db(&dir.path().join("db"), false).unwrap();
        db.execute_batch("INSERT INTO feeds(id,url,title) VALUES(1,'https://example.invalid/s','S'); INSERT INTO refresh_leases VALUES(1,'new-owner',unixepoch()+60);").unwrap();
        let state = super::super::social::Checkpoint {
            cursor: Some("next".into()),
            more_pending: true,
            ..Default::default()
        };
        let result = Fetched {
            parsed: None,
            etag: None,
            modified: None,
        };
        assert!(matches!(
            apply_refresh_with_checkpoint(&mut db, 1, result, Some("old-owner"), Some(state)),
            Err(StatusCode::CONFLICT)
        ));
        assert_eq!(
            db.query_row("SELECT count(*) FROM collection_runtime", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert_eq!(
            db.query_row("SELECT last_checked FROM feeds WHERE id=1", [], |r| r
                .get::<_, Option<String>>(0))
                .unwrap(),
            None
        );
    }
    fn synthetic_rss(entries: &[(&str, &str)]) -> Fetched {
        let mut xml = String::from(
            "<rss version=\"2.0\"><channel><title>Synthetic</title><link>https://example.invalid/</link><description>Fixture</description>",
        );
        for (id, body) in entries {
            xml.push_str(&format!("<item><guid isPermaLink=\"false\">{id}</guid><title>{id}</title><link>https://example.invalid/{id}</link><description>{body}</description><pubDate>Sat, 07 Nov 2026 00:00:00 GMT</pubDate></item>"));
        }
        xml.push_str("</channel></rss>");
        Fetched {
            parsed: Some(
                parse_feed(
                    xml.as_bytes(),
                    &Url::parse("https://example.invalid/rss").unwrap(),
                )
                .unwrap(),
            ),
            etag: None,
            modified: None,
        }
    }

    #[test]
    fn accepted_rss_hook_classifies_before_success_metadata_and_preserves_reader_state() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = open_db(&dir.path().join("db"), false).unwrap();
        db.execute_batch("INSERT INTO feeds(id,url,title) VALUES(1,'https://example.invalid/rss','Synthetic'); INSERT INTO refresh_leases VALUES(1,'owner',unixepoch()+3600); UPDATE collection_settings SET enabled=1,interval_seconds=900;").unwrap();
        apply_refresh(
            &mut db,
            1,
            synthetic_rss(&[("initial", "original")]),
            Some("owner"),
        )
        .unwrap();
        db.execute_batch("UPDATE articles SET is_read=1,is_starred=1 WHERE external_id='initial'; UPDATE feeds SET last_checked=datetime('now','-16 minutes'); UPDATE collection_health SET last_success=datetime('now','-16 minutes');").unwrap();
        assert_eq!(due_feed_ids(&db).unwrap(), vec![1]);
        apply_refresh(
            &mut db,
            1,
            synthetic_rss(&[
                ("initial", "changed once"),
                ("normal", "new during regular polling"),
            ]),
            Some("owner"),
        )
        .unwrap();
        apply_refresh_failure(&mut db, 1, "owner", StatusCode::SERVICE_UNAVAILABLE).unwrap();
        assert_eq!(
            db.query_row(
                "SELECT consecutive_failures FROM collection_health WHERE feed_id=1",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            1
        );
        db.execute(
            "UPDATE collection_runtime SET next_retry=unixepoch()-1 WHERE feed_id=1",
            [],
        )
        .unwrap();
        apply_refresh(
            &mut db,
            1,
            synthetic_rss(&[
                ("initial", "changed again"),
                ("normal", "new during regular polling"),
                ("recovery", "observed after failure"),
            ]),
            Some("owner"),
        )
        .unwrap();
        let kinds:Vec<(String,String)> = db.prepare("SELECT a.external_id,r.ingestion_kind FROM articles a JOIN article_revision_state r ON r.article_id=a.id ORDER BY a.external_id").unwrap().query_map([],|r|Ok((r.get(0)?,r.get(1)?))).unwrap().collect::<rusqlite::Result<_>>().unwrap();
        assert_eq!(
            kinds,
            vec![
                ("initial".into(), "initial".into()),
                ("normal".into(), "normal".into()),
                ("recovery".into(), "recovery".into())
            ]
        );
        assert_eq!(db.query_row("SELECT is_read,is_starred,current_revision FROM articles JOIN article_revision_state r ON r.article_id=articles.id WHERE external_id='initial'",[],|r|Ok((r.get::<_,bool>(0)?,r.get::<_,bool>(1)?,r.get::<_,i64>(2)?))).unwrap(),(true,true,3));
        assert_eq!(
            db.query_row(
                "SELECT consecutive_failures FROM collection_health WHERE feed_id=1",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
        assert!(
            db.query_row("SELECT last_error FROM feeds WHERE id=1", [], |r| r
                .get::<_, Option<String>>(0))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn social_recovery_context_survives_health_reset_and_final_page() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = open_db(&dir.path().join("db"), false).unwrap();
        db.execute_batch("INSERT INTO feeds(id,url,title,last_checked) VALUES(1,'https://example.invalid/social','Synthetic',datetime('now')); INSERT INTO social_sources VALUES(1,'mastodon','alice','https://example.invalid','123'); INSERT INTO collection_health VALUES(1,datetime('now'),1); INSERT INTO collection_runtime(feed_id,high_water) VALUES(1,'10'); INSERT INTO refresh_leases VALUES(1,'owner',unixepoch()+3600);").unwrap();
        let pending = super::social::Checkpoint {
            high_water: Some("10".into()),
            cycle_head: Some("30".into()),
            cursor: Some("20".into()),
            more_pending: true,
            recovery: true,
            ..Default::default()
        };
        apply_refresh_with_checkpoint(
            &mut db,
            1,
            synthetic_rss(&[("page1", "recovery page")]),
            Some("owner"),
            Some(pending),
        )
        .unwrap();
        assert_eq!(
            super::collection_runtime::collection_context(&db, 1).unwrap(),
            super::collection_runtime::CollectionContext::RecoveryBackfill
        );
        let complete = super::social::Checkpoint {
            high_water: Some("30".into()),
            recovery: true,
            ..Default::default()
        };
        apply_refresh_with_checkpoint(
            &mut db,
            1,
            synthetic_rss(&[("page2", "last recovery page")]),
            Some("owner"),
            Some(complete),
        )
        .unwrap();
        assert_eq!(
            db.query_row(
                "SELECT count(*) FROM article_revision_state WHERE ingestion_kind='recovery'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            2
        );
        assert_eq!(
            super::collection_runtime::collection_context(&db, 1).unwrap(),
            super::collection_runtime::CollectionContext::Normal
        );
    }

    #[tokio::test]
    async fn retry_headers_are_captured_only_for_429_and_503_and_are_task_local() {
        for status in [
            StatusCode::TOO_MANY_REQUESTS,
            StatusCode::SERVICE_UNAVAILABLE,
        ] {
            RETRY_HINT
                .scope(std::cell::Cell::new(None), async {
                    let mut headers = HeaderMap::new();
                    headers.insert(reqwest::header::RETRY_AFTER, "120".parse().unwrap());
                    record_retry_hint(status, &headers, 1000);
                    assert_eq!(RETRY_HINT.with(|hint| hint.get()), Some(1120));
                })
                .await;
        }
        RETRY_HINT
            .scope(std::cell::Cell::new(None), async {
                let mut headers = HeaderMap::new();
                headers.insert(reqwest::header::RETRY_AFTER, "120".parse().unwrap());
                record_retry_hint(StatusCode::BAD_GATEWAY, &headers, 1000);
                assert_eq!(RETRY_HINT.with(|hint| hint.get()), None);
                headers.insert(reqwest::header::RETRY_AFTER, "invalid".parse().unwrap());
                record_retry_hint(StatusCode::SERVICE_UNAVAILABLE, &headers, 1000);
                assert_eq!(RETRY_HINT.with(|hint| hint.get()), None);
                headers.insert(
                    reqwest::header::RETRY_AFTER,
                    "Sun, 06 Nov 1994 08:49:37 GMT".parse().unwrap(),
                );
                record_retry_hint(StatusCode::SERVICE_UNAVAILABLE, &headers, 0);
                assert_eq!(RETRY_HINT.with(|hint| hint.get()), Some(784111777));
            })
            .await;
    }
}
