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
    match_count: usize,
    members_truncated: bool,
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
    next_before_id: Option<i64>,
}

fn query(raw: Option<String>) -> Result<(usize, usize, Option<i64>), StatusCode> {
    let raw = raw.unwrap_or_default();
    if raw.len() > 128 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let (mut offset, mut limit) = (0, 20);
    let mut before_id = None;
    let mut seen = HashSet::new();
    for (k, v) in url::form_urlencoded::parse(raw.as_bytes()) {
        if !seen.insert(k.to_string()) {
            return Err(StatusCode::BAD_REQUEST);
        }
        match k.as_ref() {
            "offset" => offset = v.parse::<usize>().map_err(|_| StatusCode::BAD_REQUEST)?,
            "limit" => limit = v.parse::<usize>().map_err(|_| StatusCode::BAD_REQUEST)?,
            "before_id" => {
                let id: i64 = v.parse().map_err(|_| StatusCode::BAD_REQUEST)?;
                if id <= 0 {
                    return Err(StatusCode::BAD_REQUEST);
                }
                before_id = Some(id);
            }
            _ => return Err(StatusCode::BAD_REQUEST),
        }
    }
    if offset > SCAN_LIMIT || !(1..=50).contains(&limit) {
        return Err(StatusCode::BAD_REQUEST);
    }
    Ok((offset, limit, before_id))
}
fn collect(
    db: &Connection,
    offset: usize,
    limit: usize,
    before_id: Option<i64>,
) -> rusqlite::Result<Response> {
    // The selected window supplies at most 1,000 candidate URLs. Match those
    // URLs against the entire library without holding all articles in memory.
    let mut anchors: BTreeMap<String, (i64, usize)> = BTreeMap::new();
    let mut scanned = 0;
    let mut truncated = false;
    let mut last_id = None;
    {
        let mut stmt = db.prepare(
            "SELECT id,url FROM articles WHERE (?1 IS NULL OR id < ?1) ORDER BY id DESC LIMIT 1001",
        )?;
        let rows = stmt.query_map([before_id], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (id, url) = row?;
            if scanned == SCAN_LIMIT {
                truncated = true;
                break;
            }
            scanned += 1;
            last_id = Some(id);
            if let Some(key) = canonical(&url) {
                anchors.entry(key).or_insert((id, 0));
            }
        }
    }
    {
        let mut stmt = db.prepare("SELECT url FROM articles")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        for row in rows {
            if let Some((_, count)) = canonical(&row?).and_then(|key| anchors.get_mut(&key)) {
                *count += 1;
            }
        }
    }
    let mut candidates: Vec<(String, i64, usize)> = anchors
        .into_iter()
        .filter_map(|(key, (id, count))| (count > 1).then_some((key, id, count)))
        .collect();
    candidates.sort_by_key(|a| std::cmp::Reverse(a.1));
    let group_count = candidates.len();
    let article_count = candidates.iter().map(|candidate| candidate.2).sum();
    let has_more = offset.saturating_add(limit) < group_count;
    let mut groups: Vec<Group> = candidates
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(|(canonical_url, _, match_count)| Group {
            canonical_url,
            articles: Vec::new(),
            match_count,
            members_truncated: match_count > 100,
        })
        .collect();
    let slots: BTreeMap<String, usize> = groups
        .iter()
        .enumerate()
        .map(|(index, group)| (group.canonical_url.clone(), index))
        .collect();
    if !groups.is_empty() {
        let mut stmt = db.prepare("SELECT a.id,a.title,a.url,a.feed_id,f.title,a.is_read,a.is_starred FROM articles a LEFT JOIN feeds f ON f.id=a.feed_id ORDER BY a.id DESC")?;
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
        for row in rows {
            let member = row?;
            if let Some(index) = canonical(&member.url).and_then(|key| slots.get(&key).copied())
                && groups[index].articles.len() < 100
            {
                groups[index].articles.push(member);
            }
        }
    }

    Ok(Response {
        groups,
        group_count,
        article_count,
        scanned,
        truncated,
        offset,
        limit,
        has_more,
        next_before_id: if truncated { last_id } else { None },
    })
}
async fn list(State(db): State<Db>, RawQuery(raw): RawQuery) -> Result<Json<Response>, StatusCode> {
    let (offset, limit, before_id) = query(raw)?;
    let db = db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    collect(&db, offset, limit, before_id)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
#[derive(Serialize)]
struct MemberPage {
    articles: Vec<Member>,
    next_before_id: Option<i64>,
}
fn member_page(db: &Connection, anchor: i64, before: i64) -> Result<MemberPage, StatusCode> {
    let url: Option<String> = db
        .query_row("SELECT url FROM articles WHERE id=?", [anchor], |r| {
            r.get(0)
        })
        .optional()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let key = canonical(&url.ok_or(StatusCode::NOT_FOUND)?).ok_or(StatusCode::BAD_REQUEST)?;
    let mut stmt = db.prepare("SELECT a.id,a.title,a.url,a.feed_id,f.title,a.is_read,a.is_starred FROM articles a LEFT JOIN feeds f ON f.id=a.feed_id WHERE a.id < ? ORDER BY a.id DESC").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let rows = stmt
        .query_map([before], |r| {
            Ok(Member {
                id: r.get(0)?,
                title: r.get(1)?,
                url: r.get(2)?,
                feed_id: r.get(3)?,
                feed_title: r.get(4)?,
                is_read: r.get(5)?,
                is_starred: r.get(6)?,
            })
        })
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut articles = Vec::new();
    let mut more = false;
    for row in rows {
        let member = row.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if canonical(&member.url).as_deref() != Some(key.as_str()) {
            continue;
        }
        if articles.len() == 100 {
            more = true;
            break;
        }
        articles.push(member);
    }
    let next_before_id = if more {
        articles.last().map(|a| a.id)
    } else {
        None
    };
    Ok(MemberPage {
        articles,
        next_before_id,
    })
}
async fn members(
    State(db): State<Db>,
    RawQuery(raw): RawQuery,
) -> Result<Json<MemberPage>, StatusCode> {
    let raw = raw.ok_or(StatusCode::BAD_REQUEST)?;
    if raw.len() > 128 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let (mut anchor, mut before) = (None, None);
    for (key, value) in url::form_urlencoded::parse(raw.as_bytes()) {
        let parsed: i64 = value.parse().map_err(|_| StatusCode::BAD_REQUEST)?;
        if parsed <= 0 {
            return Err(StatusCode::BAD_REQUEST);
        }
        match key.as_ref() {
            "article_id" if anchor.is_none() => anchor = Some(parsed),
            "before_id" if before.is_none() => before = Some(parsed),
            _ => return Err(StatusCode::BAD_REQUEST),
        }
    }
    let db = db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(member_page(
        &db,
        anchor.ok_or(StatusCode::BAD_REQUEST)?,
        before.ok_or(StatusCode::BAD_REQUEST)?,
    )?))
}
pub fn routes() -> Router<Db> {
    Router::new()
        .route("/api/duplicates", get(list))
        .route("/api/duplicates/members", get(members))
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
        let result = collect(&db, 0, 20, None).unwrap();
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
    #[test]
    fn older_windows_reach_earlier_duplicates_without_overlap() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE feeds(id INTEGER PRIMARY KEY,title TEXT); CREATE TABLE articles(id INTEGER PRIMARY KEY,title TEXT,url TEXT,feed_id INTEGER,is_read INTEGER,is_starred INTEGER);").unwrap();
        for id in 1..=1002 {
            let url = if id <= 3 {
                "https://example.invalid/shared".to_owned()
            } else {
                format!("https://example.invalid/{id}")
            };
            db.execute(
                "INSERT INTO articles VALUES(?1,'fixture',?2,NULL,0,0)",
                params![id, url],
            )
            .unwrap();
        }
        let first = collect(&db, 0, 20, None).unwrap();
        assert!(first.truncated);
        assert_eq!(first.next_before_id, Some(3));
        assert_eq!(first.group_count, 1);
        assert_eq!(first.groups[0].match_count, 3);
        assert_eq!(
            first.groups[0]
                .articles
                .iter()
                .map(|a| a.id)
                .collect::<Vec<_>>(),
            vec![3, 2, 1]
        );
        let older = collect(&db, 0, 20, first.next_before_id).unwrap();
        assert_eq!(
            (older.scanned, older.group_count, older.article_count),
            (2, 1, 3)
        );
        assert_eq!(older.next_before_id, None);
        assert!(query(Some("before_id=0".into())).is_err());
        assert!(query(Some("before_id=1&before_id=2".into())).is_err());
    }
    #[test]
    fn large_duplicate_group_has_true_count_and_bounded_members() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE feeds(id INTEGER PRIMARY KEY,title TEXT); CREATE TABLE articles(id INTEGER PRIMARY KEY,title TEXT,url TEXT,feed_id INTEGER,is_read INTEGER,is_starred INTEGER); WITH RECURSIVE n(x) AS (VALUES(1) UNION ALL SELECT x+1 FROM n WHERE x<105) INSERT INTO articles SELECT x,'fixture','https://example.invalid/same',NULL,0,0 FROM n;").unwrap();
        let result = collect(&db, 0, 20, None).unwrap();
        assert_eq!(result.article_count, 105);
        assert_eq!(result.groups[0].match_count, 105);
        assert_eq!(result.groups[0].articles.len(), 100);
        assert!(result.groups[0].members_truncated);
        let page = member_page(&db, 105, 6).unwrap();
        assert_eq!(
            page.articles.iter().map(|a| a.id).collect::<Vec<_>>(),
            vec![5, 4, 3, 2, 1]
        );
        assert_eq!(page.next_before_id, None);
        assert!(matches!(
            member_page(&db, 999, 6),
            Err(StatusCode::NOT_FOUND)
        ));
    }
}
