use super::*;
use axum::{extract::RawQuery, routing::get};
use std::collections::{BTreeMap, HashSet};
use url::Url;

const SCAN_LIMIT: usize = 1000;

pub fn migrate(_tx: &rusqlite::Transaction<'_>) -> rusqlite::Result<()> {
    Ok(())
}

fn canonical(raw: &str) -> Option<String> {
    let mut url = Url::parse(raw).ok()?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return None;
    }
    url.set_fragment(None);
    let pairs: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(k, _)| {
            let key = k.to_ascii_lowercase();
            !key.starts_with("utm_") && key != "fbclid" && key != "gclid"
        })
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    url.set_query(None);
    if !pairs.is_empty() {
        url.query_pairs_mut().extend_pairs(pairs);
    }
    Some(url.into())
}

#[derive(Serialize)]
struct Member {
    id: i64,
    title: String,
    url: String,
    feed_id: Option<i64>,
    feed_title: Option<String>,
    is_read: bool,
    is_starred: bool,
}
#[derive(Serialize)]
struct Group {
    canonical_url: String,
    articles: Vec<Member>,
}
#[derive(Serialize)]
struct Response {
    groups: Vec<Group>,
    group_count: usize,
    article_count: usize,
    scanned: usize,
    truncated: bool,
    offset: usize,
    limit: usize,
    has_more: bool,
}

fn query(raw: Option<String>) -> Result<(usize, usize), StatusCode> {
    let raw = raw.unwrap_or_default();
    if raw.len() > 128 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let (mut offset, mut limit) = (0, 20);
    let mut seen = HashSet::new();
    for (k, v) in url::form_urlencoded::parse(raw.as_bytes()) {
        if !seen.insert(k.to_string()) {
            return Err(StatusCode::BAD_REQUEST);
        }
        match k.as_ref() {
            "offset" => offset = v.parse::<usize>().map_err(|_| StatusCode::BAD_REQUEST)?,
            "limit" => limit = v.parse::<usize>().map_err(|_| StatusCode::BAD_REQUEST)?,
            _ => return Err(StatusCode::BAD_REQUEST),
        }
    }
    if offset > SCAN_LIMIT || !(1..=50).contains(&limit) {
        return Err(StatusCode::BAD_REQUEST);
    }
    Ok((offset, limit))
}
fn collect(db: &Connection, offset: usize, limit: usize) -> rusqlite::Result<Response> {
    let mut stmt = db.prepare("SELECT a.id,a.title,a.url,a.feed_id,f.title,a.is_read,a.is_starred FROM articles a LEFT JOIN feeds f ON f.id=a.feed_id ORDER BY a.id DESC LIMIT 1001")?;
    let rows = stmt.query_map([], |r| {
        Ok(Member {
            id: r.get(0)?,
            title: r.get(1)?,
            url: r.get(2)?,
            feed_id: r.get(3)?,
            feed_title: r.get(4)?,
            is_read: r.get(5)?,
            is_starred: r.get(6)?,
        })
    })?;
    let mut grouped: BTreeMap<String, Vec<Member>> = BTreeMap::new();
    let mut scanned = 0;
    let mut truncated = false;
    for row in rows {
        let member = row?;
        if scanned == SCAN_LIMIT {
            truncated = true;
            break;
        }
        scanned += 1;
        if let Some(key) = canonical(&member.url) {
            grouped.entry(key).or_default().push(member);
        }
    }
    let mut groups: Vec<Group> = grouped
        .into_iter()
        .filter(|(_, v)| v.len() > 1)
        .map(|(canonical_url, articles)| Group {
            canonical_url,
            articles,
        })
        .collect();
    groups.sort_by(|a, b| b.articles[0].id.cmp(&a.articles[0].id));
    let group_count = groups.len();
    let article_count = groups.iter().map(|g| g.articles.len()).sum();
    let has_more = offset.saturating_add(limit) < group_count;
    Ok(Response {
        groups: groups.into_iter().skip(offset).take(limit).collect(),
        group_count,
        article_count,
        scanned,
        truncated,
        offset,
        limit,
        has_more,
    })
}
async fn list(State(db): State<Db>, RawQuery(raw): RawQuery) -> Result<Json<Response>, StatusCode> {
    let (offset, limit) = query(raw)?;
    let db = db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    collect(&db, offset, limit)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
pub fn routes() -> Router<Db> {
    Router::new().route("/api/duplicates", get(list))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canonicalizes_only_tracking() {
        assert_eq!(
            canonical("https://example.com/a?x=1&utm_source=a&FBCLID=b&gclid=c#top"),
            canonical("https://example.com/a?x=1")
        );
        assert_ne!(
            canonical("https://example.com/a?x=1"),
            canonical("https://example.com/a?x=2")
        );
        assert!(canonical("file:///tmp/a").is_none());
        assert!(canonical("not a url").is_none());
    }
    #[test]
    fn scanning_preserves_original_states_and_urls() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE feeds(id INTEGER PRIMARY KEY,title TEXT); CREATE TABLE articles(id INTEGER PRIMARY KEY,title TEXT,url TEXT,feed_id INTEGER,is_read INTEGER,is_starred INTEGER); INSERT INTO feeds VALUES(1,'One'),(2,'Two'); INSERT INTO articles VALUES(1,'A','https://e.test/?utm_x=1',1,1,0),(2,'B','https://e.test/#a',2,0,1);").unwrap();
        let result = collect(&db, 0, 20).unwrap();
        assert_eq!(
            (
                result.group_count,
                result.article_count,
                result.scanned,
                result.truncated
            ),
            (1, 2, 2, false)
        );
        assert!(!result.groups[0].articles[0].is_read && result.groups[0].articles[0].is_starred);
        let original: (String, bool, bool) = db
            .query_row(
                "SELECT url,is_read,is_starred FROM articles WHERE id=1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(original, ("https://e.test/?utm_x=1".into(), true, false));
        assert!(query(Some("limit=51".into())).is_err());
    }
}
