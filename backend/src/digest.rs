use super::*;
use axum::{extract::RawQuery, routing::get};
use std::collections::HashSet;

#[derive(Serialize)]
struct DigestItem {
    id: i64,
    title: String,
    published: String,
    url: String,
}

#[derive(Serialize)]
struct DigestSource {
    feed_id: i64,
    source_title: String,
    count: i64,
    shown_count: usize,
    items: Vec<DigestItem>,
}

#[derive(Serialize)]
struct Digest {
    hours: u16,
    total: i64,
    shown_count: usize,
    source_count: usize,
    truncated: bool,
    sources: Vec<DigestSource>,
}

#[derive(Default)]
struct Query {
    hours: u16,
    feed_id: Option<i64>,
    folder_id: Option<i64>,
}

fn parse(raw: Option<&str>) -> Result<Query, StatusCode> {
    let raw = raw.unwrap_or("");
    if raw.len() > 1024 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut seen = HashSet::new();
    let mut query = Query {
        hours: 24,
        ..Query::default()
    };
    for (key, value) in url::form_urlencoded::parse(raw.as_bytes()) {
        if !seen.insert(key.to_string()) {
            return Err(StatusCode::BAD_REQUEST);
        }
        match key.as_ref() {
            "hours" => {
                query.hours = value.parse().map_err(|_| StatusCode::BAD_REQUEST)?;
                if !matches!(query.hours, 24 | 168) {
                    return Err(StatusCode::BAD_REQUEST);
                }
            }
            "feed_id" => query.feed_id = Some(value.parse().map_err(|_| StatusCode::BAD_REQUEST)?),
            "folder_id" => {
                query.folder_id = Some(value.parse().map_err(|_| StatusCode::BAD_REQUEST)?)
            }
            _ => return Err(StatusCode::BAD_REQUEST),
        }
    }
    if query.feed_id.is_some() && query.folder_id.is_some() {
        return Err(StatusCode::BAD_REQUEST);
    }
    if query.feed_id.is_some_and(|id| id <= 0) || query.folder_id.is_some_and(|id| id <= 0) {
        return Err(StatusCode::BAD_REQUEST);
    }
    Ok(query)
}

async fn get_digest(
    State(db): State<Db>,
    RawQuery(raw): RawQuery,
) -> Result<Json<Digest>, StatusCode> {
    let query = parse(raw.as_deref())?;
    let db = db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    for (table, id) in [("feeds", query.feed_id), ("folders", query.folder_id)] {
        if let Some(id) = id {
            let exists = db
                .query_row(
                    &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE id=?)"),
                    [id],
                    |row| row.get::<_, bool>(0),
                )
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            if !exists {
                return Err(StatusCode::BAD_REQUEST);
            }
        }
    }
    let modifier = format!("-{} hours", query.hours);
    let scope = if query.feed_id.is_some() {
        " AND a.feed_id=?2"
    } else if query.folder_id.is_some() {
        " AND f.folder_id=?2"
    } else {
        ""
    };
    let hidden = mute_rules::HIDDEN_SQL.replace("articles.", "a.");
    let base = format!(
        "FROM articles a JOIN feeds f ON f.id=a.feed_id WHERE datetime(a.published) >= datetime('now',?1) AND datetime(a.published) <= datetime('now') AND NOT ({hidden}){scope}"
    );
    let scope_id = query.feed_id.or(query.folder_id);
    let sql = format!("SELECT f.id,f.title,COUNT(*) {base} GROUP BY f.id,f.title");
    let mut stmt = db.prepare(&sql).map_err(collection::err)?;
    let args = rusqlite::params_from_iter(
        std::iter::once(rusqlite::types::Value::Text(modifier.clone()))
            .chain(scope_id.map(rusqlite::types::Value::Integer)),
    );
    let counts = stmt
        .query_map(args, |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
            ))
        })
        .map_err(collection::err)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(collection::err)?;
    let total = counts.iter().map(|(_, _, count)| count).sum::<i64>();
    let source_count = counts.len();
    let sql = format!(
        "SELECT feed_id,id,title,published,url FROM (SELECT a.feed_id,a.id,a.title,a.published,a.url,ROW_NUMBER() OVER (PARTITION BY a.feed_id ORDER BY datetime(a.published) DESC,a.id DESC) AS rank {base}) WHERE rank <= 10 ORDER BY datetime(published) DESC,id DESC LIMIT 100"
    );
    let mut stmt = db.prepare(&sql).map_err(collection::err)?;
    let args = rusqlite::params_from_iter(
        std::iter::once(rusqlite::types::Value::Text(modifier))
            .chain(scope_id.map(rusqlite::types::Value::Integer)),
    );
    let rows = stmt
        .query_map(args, |r| {
            Ok((
                r.get::<_, i64>(0)?,
                DigestItem {
                    id: r.get(1)?,
                    title: r.get(2)?,
                    published: r.get(3)?,
                    url: r.get(4)?,
                },
            ))
        })
        .map_err(collection::err)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(collection::err)?;
    let shown_count = rows.len();
    let mut result: Vec<DigestSource> = Vec::new();
    let lookup: std::collections::HashMap<_, _> = counts
        .into_iter()
        .map(|(id, title, count)| (id, (title, count)))
        .collect();
    for (feed_id, item) in rows {
        let index = if let Some(index) = result.iter().position(|source| source.feed_id == feed_id)
        {
            index
        } else {
            let (title, count) = &lookup[&feed_id];
            result.push(DigestSource {
                feed_id,
                source_title: title.clone(),
                count: *count,
                shown_count: 0,
                items: Vec::new(),
            });
            result.len() - 1
        };
        result[index].items.push(item);
        result[index].shown_count += 1;
    }
    Ok(Json(Digest {
        hours: query.hours,
        total,
        shown_count,
        source_count,
        truncated: (shown_count as i64) < total,
        sources: result,
    }))
}

pub fn routes() -> Router<Db> {
    Router::new().route("/api/digest", get(get_digest))
}

pub fn migrate(_tx: &rusqlite::Transaction<'_>) -> rusqlite::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_bounded_query() {
        assert_eq!(parse(Some("hours=168&feed_id=3")).unwrap().hours, 168);
        for invalid in [
            "hours=25",
            "feed_id=0",
            "feed_id=1&folder_id=2",
            "hours=24&hours=168",
            "other=1",
        ] {
            assert!(parse(Some(invalid)).is_err(), "{invalid}");
        }
    }

    #[tokio::test]
    async fn counts_actual_recent_rows_and_caps_items() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE articles(id INTEGER PRIMARY KEY,title TEXT NOT NULL,content TEXT NOT NULL,url TEXT NOT NULL,published TEXT NOT NULL,is_read INTEGER NOT NULL DEFAULT 0,is_starred INTEGER NOT NULL DEFAULT 0,feed_id INTEGER); CREATE TABLE folders(id INTEGER PRIMARY KEY); INSERT INTO folders VALUES(4); CREATE TABLE feeds(id INTEGER PRIMARY KEY,url TEXT,title TEXT,folder_id INTEGER); INSERT INTO feeds VALUES(1,'u','Source',4),(2,'v','Other',4); INSERT INTO articles VALUES(20,'Old','body','url',datetime('now','-30 hours'),0,0,1); INSERT INTO articles VALUES(21,'Unknown','body','url','not a date',0,0,1); INSERT INTO articles VALUES(30,'Recent','body','url',datetime('now','-2 hours'),0,0,2);").unwrap();
        let tx = conn.transaction().unwrap();
        mute_rules::migrate(&tx).unwrap();
        tx.commit().unwrap();
        for id in 1..=12 {
            conn.execute("INSERT INTO articles(id,title,content,url,published,feed_id) VALUES (?,'Story','body','https://example.org',datetime('now','-1 hour'),1)", [id]).unwrap();
        }
        let db = Arc::new(Mutex::new(conn));
        let response = get_digest(State(db), RawQuery(Some("hours=24&folder_id=4".into())))
            .await
            .unwrap();
        let digest = response.0;
        assert_eq!(digest.total, 13);
        assert_eq!(
            digest.sources.iter().map(|s| s.items.len()).sum::<usize>(),
            11
        );
        assert_eq!(digest.sources[0].source_title, "Source");
        assert_eq!(digest.sources[0].count, 12);
        assert_eq!(digest.sources[0].items.len(), 10);
    }
    #[tokio::test]
    async fn newest_global_cap_and_mutes_apply_to_counts_and_items() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE folders(id INTEGER PRIMARY KEY); INSERT INTO folders VALUES(1); CREATE TABLE feeds(id INTEGER PRIMARY KEY,title TEXT,folder_id INTEGER); CREATE TABLE articles(id INTEGER PRIMARY KEY,title TEXT,content TEXT,url TEXT,published TEXT,feed_id INTEGER); INSERT INTO feeds VALUES(1,'Old bulk',1); INSERT INTO articles VALUES(9000,'Future','body','url',datetime('now','+1 hour'),1),(9001,'Unknown','body','url','invalid',1);").unwrap();
        let tx = conn.transaction().unwrap();
        mute_rules::migrate(&tx).unwrap();
        tx.commit().unwrap();
        for feed in 1..=12 {
            if feed > 1 {
                conn.execute(
                    "INSERT INTO feeds VALUES(?1,?2,1)",
                    params![feed, format!("Source {feed}")],
                )
                .unwrap();
            }
            for n in 0..12 {
                let id = feed * 100 + n;
                let age = if feed == 1 { 6000 + n } else { feed * 20 + n };
                conn.execute(
                    "INSERT INTO articles VALUES(?1,'visible','body','url',datetime('now',?2),?3)",
                    params![id, format!("-{age} seconds"), feed],
                )
                .unwrap();
            }
        }
        conn.execute_batch("UPDATE articles SET title='hidden' WHERE id=200; INSERT INTO mute_rules(name,term,field,feed_id,enabled,revision) VALUES('hide','hidden','title',2,1,1),('disabled','visible','title',NULL,0,1);").unwrap();
        let db = Arc::new(Mutex::new(conn));
        let digest = get_digest(State(db.clone()), RawQuery(None))
            .await
            .unwrap()
            .0;
        assert_eq!(digest.total, 143);
        assert_eq!(digest.source_count, 12);
        assert_eq!(digest.shown_count, 100);
        assert!(digest.truncated);
        assert!(!digest.sources.iter().any(|source| source.feed_id == 1));
        assert!(digest.sources.iter().all(
            |source| source.shown_count <= 10 && source.items.iter().all(|item| item.id != 200)
        ));
        let scoped = get_digest(State(db), RawQuery(Some("feed_id=2".into())))
            .await
            .unwrap()
            .0;
        assert_eq!(scoped.total, 11);
        assert_eq!(scoped.shown_count, 10);
        assert!(scoped.truncated);
        assert_eq!(scoped.sources[0].items[0].id, 201);
    }
}
