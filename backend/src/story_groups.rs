use super::*;
use axum::extract::RawQuery;
use axum::routing::put;
use sha2::{Digest, Sha256};
use url::Url;
fn err<T>(_: T) -> StatusCode {
    StatusCode::INTERNAL_SERVER_ERROR
}
pub fn migrate_catchup(tx: &rusqlite::Transaction<'_>) -> rusqlite::Result<()> {
    tx.execute_batch("CREATE TABLE article_story_index (article_id INTEGER PRIMARY KEY REFERENCES articles(id) ON DELETE CASCADE, canonical_url TEXT, body_fingerprint TEXT, title_key TEXT, published TEXT NOT NULL, source_key TEXT); CREATE INDEX article_story_url ON article_story_index(canonical_url,article_id); CREATE INDEX article_story_body ON article_story_index(body_fingerprint,title_key,article_id); CREATE TABLE article_story_exclusions (article_id INTEGER PRIMARY KEY REFERENCES articles(id) ON DELETE CASCADE);")
}
fn canonical(raw: &str) -> Option<String> {
    let mut u = Url::parse(raw).ok()?;
    if !matches!(u.scheme(), "http" | "https")
        || u.host_str().is_none()
        || !u.username().is_empty()
        || u.password().is_some()
    {
        return None;
    }
    u.set_fragment(None);
    let pairs: Vec<_> = u
        .query_pairs()
        .filter(|(k, _)| {
            let k = k.to_ascii_lowercase();
            !k.starts_with("utm_") && k != "gclid" && k != "fbclid"
        })
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    u.set_query(None);
    if !pairs.is_empty() {
        u.query_pairs_mut().extend_pairs(pairs);
    }
    if u.path() == "/" && u.query().is_none() {
        return None;
    }
    Some(u.into())
}
fn numbers(s: &str) -> Vec<String> {
    s.split(|c: char| !c.is_ascii_digit())
        .filter(|n| !n.is_empty())
        .map(str::to_owned)
        .collect()
}
fn normalized(s: &str) -> String {
    s.chars()
        .flat_map(char::to_lowercase)
        .filter(|c| c.is_alphanumeric())
        .collect()
}
pub fn index_article(
    tx: &rusqlite::Transaction<'_>,
    id: i64,
    title: &str,
    body: &str,
    url: &str,
    published: &str,
) -> rusqlite::Result<()> {
    let title = normalized(title);
    let body = normalized(body);
    let key = if body.chars().count() >= 200 && title.chars().count() >= 12 {
        Some(
            Sha256::digest(body.as_bytes())
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>(),
        )
    } else {
        None
    };
    let url = canonical(url);
    let source = url
        .as_deref()
        .and_then(|s| Url::parse(s).ok())
        .and_then(|u| u.host_str().map(str::to_owned));
    tx.execute("INSERT INTO article_story_index VALUES (?,?,?,?,?,?) ON CONFLICT(article_id) DO UPDATE SET canonical_url=excluded.canonical_url,body_fingerprint=excluded.body_fingerprint,title_key=excluded.title_key,published=excluded.published,source_key=excluded.source_key",params![id,url,key,title,published,source])?;
    Ok(())
}
#[derive(Debug, Clone)]
pub struct RelatedMember {
    pub article_id: i64,
    pub reason: &'static str,
}
#[derive(Debug, Clone)]
pub struct RelatedSelection {
    pub members: Vec<RelatedMember>,
    pub total: usize,
    pub truncated: bool,
    pub excluded: bool,
}
pub fn related_ids(db: &Connection, id: i64, limit: usize) -> rusqlite::Result<RelatedSelection> {
    let excluded: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM article_story_exclusions WHERE article_id=?)",
        [id],
        |r| r.get(0),
    )?;
    if excluded {
        return Ok(RelatedSelection {
            members: vec![],
            total: 0,
            truncated: false,
            excluded: true,
        });
    }
    let anchor:Option<(Option<String>,Option<String>,String,String)>=db.query_row("SELECT canonical_url,body_fingerprint,title_key,published FROM article_story_index WHERE article_id=?",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional()?;
    let Some((url, body, title, published)) = anchor else {
        return Ok(RelatedSelection {
            members: vec![],
            total: 0,
            truncated: false,
            excluded: false,
        });
    };
    let anchor_title: String =
        db.query_row("SELECT title FROM articles WHERE id=?", [id], |r| r.get(0))?;
    let anchor_numbers = numbers(&anchor_title);
    let mut matches = std::collections::BTreeMap::new();
    let mut capped = false;
    for (key, sql, reason) in [
        (
            url.as_deref(),
            "SELECT article_id FROM article_story_index WHERE canonical_url=? AND article_id<>? ORDER BY article_id DESC LIMIT 501",
            "canonical_url",
        ),
        (
            body.as_deref(),
            "SELECT article_id FROM article_story_index WHERE body_fingerprint=? AND article_id<>? AND title_key=? AND abs((julianday(published)-julianday(?))*24)<=48 ORDER BY article_id DESC LIMIT 501",
            "content_title_time",
        ),
    ] {
        let Some(key) = key else { continue };
        let mut st = db.prepare(sql)?;
        let rows: Vec<i64> = if reason == "canonical_url" {
            st.query_map(params![key, id], |r| r.get(0))?
                .collect::<Result<_, _>>()?
        } else {
            st.query_map(params![key, id, title, published], |r| r.get(0))?
                .collect::<Result<_, _>>()?
        };
        if rows.len() > 500 {
            capped = true
        }
        for other in rows.into_iter().take(500) {
            matches.entry(other).or_insert(reason);
        }
    }
    let mut members = Vec::new();
    let mut check=db.prepare(&format!("SELECT title FROM articles WHERE id=? AND NOT ({}) AND NOT EXISTS(SELECT 1 FROM article_story_exclusions WHERE article_id=articles.id)",crate::mute_rules::HIDDEN_SQL))?;
    for (other, reason) in matches.into_iter().rev() {
        let candidate: Option<String> = check.query_row([other], |r| r.get(0)).optional()?;
        if let Some(candidate) = candidate {
            let candidate_numbers = numbers(&candidate);
            if !anchor_numbers.is_empty()
                && !candidate_numbers.is_empty()
                && anchor_numbers != candidate_numbers
            {
                continue;
            }
            members.push(RelatedMember {
                article_id: other,
                reason,
            });
        }
    }
    let total = members.len();
    let truncated = capped || total > limit;
    members.truncate(limit);
    Ok(RelatedSelection {
        members,
        total,
        truncated,
        excluded: false,
    })
}
#[derive(Serialize)]
struct RelatedArticle {
    article: Article,
    reason: &'static str,
}
#[derive(Serialize)]
struct RelatedResponse {
    article_id: i64,
    method_version: &'static str,
    articles: Vec<RelatedArticle>,
    total: usize,
    truncated: bool,
    excluded: bool,
}
async fn related(
    State(db): State<Db>,
    Path(id): Path<i64>,
    RawQuery(raw): RawQuery,
) -> Result<Json<RelatedResponse>, StatusCode> {
    let mut limit = 20usize;
    let mut seen = false;
    if let Some(raw) = raw {
        if raw.len() > 128 {
            return Err(StatusCode::BAD_REQUEST);
        }
        for (k, v) in url::form_urlencoded::parse(raw.as_bytes()) {
            if k != "limit" || seen {
                return Err(StatusCode::BAD_REQUEST);
            }
            seen = true;
            limit = v.parse().map_err(|_| StatusCode::BAD_REQUEST)?;
            if !(1..=20).contains(&limit) {
                return Err(StatusCode::BAD_REQUEST);
            }
        }
    }
    let db = db.lock().map_err(err)?;
    lookup(&db, id)?;
    let found = related_ids(&db, id, limit).map_err(err)?;
    let mut articles = Vec::new();
    for m in found.members {
        articles.push(RelatedArticle {
            article: lookup(&db, m.article_id)?,
            reason: m.reason,
        });
    }
    Ok(Json(RelatedResponse {
        article_id: id,
        method_version: "1",
        articles,
        total: found.total,
        truncated: found.truncated,
        excluded: found.excluded,
    }))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Exclusion {
    excluded: bool,
}
#[derive(Serialize)]
struct ExclusionResponse {
    article_id: i64,
    excluded: bool,
}
async fn exclusion(
    State(db): State<Db>,
    Path(id): Path<i64>,
    headers: HeaderMap,
    Json(input): Json<Exclusion>,
) -> Result<Json<ExclusionResponse>, StatusCode> {
    if !origin_allowed(&headers) {
        return Err(StatusCode::FORBIDDEN);
    }
    let db = db.lock().map_err(err)?;
    let exists: bool = db
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM articles WHERE id=?)",
            [id],
            |r| r.get(0),
        )
        .map_err(err)?;
    if !exists {
        return Err(StatusCode::NOT_FOUND);
    }
    if input.excluded {
        db.execute(
            "INSERT OR IGNORE INTO article_story_exclusions(article_id) VALUES (?)",
            [id],
        )
        .map_err(err)?;
    } else {
        db.execute(
            "DELETE FROM article_story_exclusions WHERE article_id=?",
            [id],
        )
        .map_err(err)?;
    }
    Ok(Json(ExclusionResponse {
        article_id: id,
        excluded: input.excluded,
    }))
}
pub fn routes() -> Router<Db> {
    Router::new()
        .route("/api/articles/{id}/related", get(related))
        .route("/api/articles/{id}/related-exclusion", put(exclusion))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn conservative_japanese_matching_versions_exclusions_and_mute() {
        let mut db = Connection::open_in_memory().unwrap();
        db.execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE articles(id INTEGER PRIMARY KEY,title TEXT,content TEXT,url TEXT,feed_id INTEGER,is_read INTEGER,is_starred INTEGER); CREATE TABLE mute_rules(enabled INTEGER,feed_id INTEGER,field TEXT,term TEXT);").unwrap();
        let tx = db.transaction().unwrap();
        migrate_catchup(&tx).unwrap();
        let body = "これは合成された長い記事の本文です安全な試験用の文章です".repeat(12);
        let rows = [
            (
                1,
                "技術製品バージョン12の新しい公開情報",
                body.as_str(),
                "https://a.invalid/one",
            ),
            (
                2,
                "技術製品バージョン12の新しい公開情報",
                body.as_str(),
                "https://b.invalid/two",
            ),
            (
                3,
                "技術製品バージョン13の新しい公開情報",
                body.as_str(),
                "https://c.invalid/three",
            ),
            (4, "短文", "短文", "https://a.invalid/one?utm_source=test"),
            (5, "短文", "短文", "https://other.invalid/"),
        ];
        for (id, title, content, url) in rows {
            tx.execute(
                "INSERT INTO articles VALUES (?,?,?,?,1,1,1)",
                params![id, title, content, url],
            )
            .unwrap();
            index_article(&tx, id, title, content, url, "2026-11-07T00:00:00Z").unwrap();
        }
        let s = related_ids(&tx, 1, 20).unwrap();
        assert_eq!(s.total, 2);
        assert_eq!(
            s.members.iter().find(|x| x.article_id == 2).unwrap().reason,
            "content_title_time"
        );
        assert_eq!(
            s.members.iter().find(|x| x.article_id == 4).unwrap().reason,
            "canonical_url"
        );
        tx.execute("INSERT INTO article_story_exclusions VALUES(2)", [])
            .unwrap();
        assert_eq!(related_ids(&tx, 1, 20).unwrap().total, 1);
        assert!(related_ids(&tx, 2, 20).unwrap().excluded);
        tx.execute("DELETE FROM article_story_exclusions", [])
            .unwrap();
        tx.execute("INSERT INTO mute_rules VALUES(1,NULL,'title','短文')", [])
            .unwrap();
        assert_eq!(related_ids(&tx, 1, 20).unwrap().total, 1);
        assert_eq!(
            tx.query_row(
                "SELECT sum(is_read)+sum(is_starred) FROM articles",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            10
        );
    }
    #[test]
    fn url_versions_root_and_scan_cap_are_conservative() {
        assert!(canonical("https://example.invalid/?utm_source=foo").is_none());
        assert!(canonical("https://example.invalid/?id=12").is_some());
        let mut db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE articles(id INTEGER PRIMARY KEY,title TEXT,content TEXT,url TEXT,feed_id INTEGER); CREATE TABLE mute_rules(enabled INTEGER,feed_id INTEGER,field TEXT,term TEXT);").unwrap();
        let tx = db.transaction().unwrap();
        migrate_catchup(&tx).unwrap();
        for id in 1..=503 {
            let title = if id == 2 {
                "Release v2.0"
            } else {
                "Release v1.0"
            };
            tx.execute(
                "INSERT INTO articles VALUES(?,?, '', 'https://example.invalid/post',1)",
                params![id, title],
            )
            .unwrap();
            index_article(&tx, id, title, "", "https://example.invalid/post", "").unwrap();
        }
        let result = related_ids(&tx, 1, 20).unwrap();
        assert!(result.truncated);
        assert_eq!(result.members.len(), 20);
        // The conflicting release appears in the examined set when the set is small.
        tx.execute("DELETE FROM article_story_index WHERE article_id>3", [])
            .unwrap();
        let result = related_ids(&tx, 1, 20).unwrap();
        assert_eq!(result.total, 1);
        assert_eq!(result.members[0].article_id, 3);
    }
}
