use super::*;
use axum::{extract::RawQuery, routing::get};
use std::collections::{HashMap, HashSet};

// Bound expensive related expansions even for adversarially overlapping indexes.
const RELATED_EXPANSION_LIMIT: usize = 100;

/// Added only by the v10 migration, within its transaction.
pub fn migrate_catchup(tx: &rusqlite::Transaction<'_>) -> rusqlite::Result<()> {
    tx.execute_batch("CREATE TABLE catchup_preferences (id INTEGER PRIMARY KEY CHECK(id=1), revision INTEGER NOT NULL CHECK(revision>0), terms TEXT NOT NULL);\
        INSERT INTO catchup_preferences VALUES(1,1,'[]');\
        CREATE TABLE catchup_priority_feeds (feed_id INTEGER PRIMARY KEY REFERENCES feeds(id) ON DELETE CASCADE, position INTEGER NOT NULL UNIQUE);\
        CREATE TRIGGER catchup_priority_feed_removed AFTER DELETE ON catchup_priority_feeds WHEN NOT EXISTS(SELECT 1 FROM feeds WHERE id=OLD.feed_id) BEGIN UPDATE catchup_preferences SET revision=revision+1 WHERE id=1; END;\
        CREATE TABLE catchup_acknowledgements (article_id INTEGER NOT NULL REFERENCES articles(id) ON DELETE CASCADE, revision INTEGER NOT NULL CHECK(revision>0), fingerprint TEXT NOT NULL, action TEXT NOT NULL CHECK(action IN ('confirmed','skipped')), PRIMARY KEY(article_id,revision,fingerprint));")
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Preferences {
    revision: i64,
    terms: Vec<String>,
    priority_feed_ids: Vec<i64>,
}

fn preferences(db: &Connection) -> rusqlite::Result<Preferences> {
    let (revision, json): (i64, String) = db.query_row(
        "SELECT revision,terms FROM catchup_preferences WHERE id=1",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let terms = serde_json::from_str(&json).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(1, rusqlite::types::Type::Text, Box::new(e))
    })?;
    let mut stmt = db.prepare("SELECT feed_id FROM catchup_priority_feeds ORDER BY position")?;
    let priority_feed_ids = stmt
        .query_map([], |r| r.get(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(Preferences {
        revision,
        terms,
        priority_feed_ids,
    })
}
fn valid_preferences(p: &Preferences) -> bool {
    p.revision > 0
        && p.terms.len() <= 20
        && p.priority_feed_ids.len() <= 20
        && p.terms
            .iter()
            .all(|s| !s.trim().is_empty() && !s.contains('\0') && s.chars().count() <= 80)
        && p.terms.iter().collect::<HashSet<_>>().len() == p.terms.len()
        && p.priority_feed_ids
            .iter()
            .all(|id| *id > 0 && *id <= 9_007_199_254_740_991)
        && p.priority_feed_ids.iter().collect::<HashSet<_>>().len() == p.priority_feed_ids.len()
}
async fn get_interests(State(db): State<Db>) -> Result<Json<Preferences>, StatusCode> {
    let db = db.lock().map_err(collection::err)?;
    preferences(&db).map(Json).map_err(collection::err)
}
async fn put_interests(
    State(db): State<Db>,
    headers: HeaderMap,
    Json(input): Json<Preferences>,
) -> Result<Json<Preferences>, StatusCode> {
    if !origin_allowed(&headers) {
        return Err(StatusCode::FORBIDDEN);
    }
    if !valid_preferences(&input) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut db = db.lock().map_err(collection::err)?;
    let tx = db
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(collection::err)?;
    let current = preferences(&tx).map_err(collection::err)?;
    if current.revision != input.revision || current.revision == i64::MAX {
        return Err(StatusCode::CONFLICT);
    }
    for id in &input.priority_feed_ids {
        if !tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM feeds WHERE id=?1)",
                [id],
                |r| r.get::<_, bool>(0),
            )
            .map_err(collection::err)?
        {
            return Err(StatusCode::BAD_REQUEST);
        }
    }
    let json =
        serde_json::to_string(&input.terms).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    tx.execute(
        "UPDATE catchup_preferences SET revision=revision+1,terms=?1 WHERE id=1 AND revision=?2",
        params![json, input.revision],
    )
    .map_err(collection::err)?;
    tx.execute("DELETE FROM catchup_priority_feeds", [])
        .map_err(collection::err)?;
    for (pos, id) in input.priority_feed_ids.iter().enumerate() {
        tx.execute(
            "INSERT INTO catchup_priority_feeds(feed_id,position) VALUES(?1,?2)",
            params![id, pos as i64],
        )
        .map_err(collection::err)?;
    }
    let result = preferences(&tx).map_err(collection::err)?;
    tx.commit().map_err(collection::err)?;
    Ok(Json(result))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AckKey {
    article_id: i64,
    revision: i64,
    fingerprint: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AckInput {
    article_id: i64,
    revision: i64,
    fingerprint: String,
    action: String,
}
#[derive(Serialize)]
struct AckResponse {
    article_id: i64,
    event_key: String,
    revision: i64,
    fingerprint: String,
    action: String,
}
fn valid_ack(id: i64, revision: i64, fingerprint: &str) -> bool {
    id > 0
        && id <= 9_007_199_254_740_991
        && revision > 0
        && !fingerprint.is_empty()
        && fingerprint.len() <= 256
        && !fingerprint.contains('\0')
}
fn event_key(id: i64, revision: i64, fingerprint: &str) -> String {
    format!("{id}:{revision}:{fingerprint}")
}
async fn delete_ack(
    State(db): State<Db>,
    headers: HeaderMap,
    Json(input): Json<AckKey>,
) -> Result<StatusCode, StatusCode> {
    if !origin_allowed(&headers) {
        return Err(StatusCode::FORBIDDEN);
    }
    if !valid_ack(input.article_id, input.revision, &input.fingerprint) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut db = db.lock().map_err(collection::err)?;
    let tx = db
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(collection::err)?;
    if !tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM articles WHERE id=?1)",
            [input.article_id],
            |r| r.get::<_, bool>(0),
        )
        .map_err(collection::err)?
    {
        return Err(StatusCode::NOT_FOUND);
    }
    tx.execute("DELETE FROM catchup_acknowledgements WHERE article_id=?1 AND revision=?2 AND fingerprint=?3", params![input.article_id,input.revision,input.fingerprint]).map_err(collection::err)?;
    tx.commit().map_err(collection::err)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Clone, Debug)]
struct Ranked {
    id: i64,
    host: String,
    terms: Vec<String>,
    priority: bool,
    updated: bool,
    event_time: f64,
    has_related: bool,
}
fn rank(mut candidates: Vec<Ranked>, limit: usize) -> Vec<Ranked> {
    candidates.sort_by(|a, b| {
        (!a.terms.is_empty())
            .cmp(&(!b.terms.is_empty()))
            .reverse()
            .then_with(|| b.priority.cmp(&a.priority))
            .then_with(|| b.updated.cmp(&a.updated))
            .then_with(|| b.event_time.total_cmp(&a.event_time))
            .then_with(|| b.id.cmp(&a.id))
    });
    let mut hosts: HashMap<String, usize> = HashMap::new();
    candidates
        .into_iter()
        .filter(|c| {
            let count = hosts.entry(c.host.clone()).or_default();
            if *count >= 2 {
                false
            } else {
                *count += 1;
                true
            }
        })
        .take(limit)
        .collect()
}
fn matched_terms(title: &str, content: &str, terms: &[String]) -> Vec<String> {
    let title = title.to_lowercase();
    let content = content.to_lowercase();
    terms
        .iter()
        .filter(|t| {
            let t = t.to_lowercase();
            title.contains(&t) || content.contains(&t)
        })
        .cloned()
        .collect()
}

async fn put_ack(
    State(db): State<Db>,
    headers: HeaderMap,
    Json(input): Json<AckInput>,
) -> Result<Json<AckResponse>, StatusCode> {
    if !origin_allowed(&headers) {
        return Err(StatusCode::FORBIDDEN);
    }
    if !valid_ack(input.article_id, input.revision, &input.fingerprint)
        || !matches!(input.action.as_str(), "confirmed" | "skipped")
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut db = db.lock().map_err(collection::err)?;
    let tx = db
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(collection::err)?;
    if !tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM articles WHERE id=?1)",
            [input.article_id],
            |r| r.get::<_, bool>(0),
        )
        .map_err(collection::err)?
    {
        return Err(StatusCode::NOT_FOUND);
    }
    let current =
        crate::article_revisions::current_state(&tx, input.article_id).map_err(collection::err)?;
    if !current
        .is_some_and(|s| s.current_revision == input.revision && s.fingerprint == input.fingerprint)
    {
        return Err(StatusCode::CONFLICT);
    }
    tx.execute("INSERT INTO catchup_acknowledgements(article_id,revision,fingerprint,action) VALUES(?1,?2,?3,?4) ON CONFLICT(article_id,revision,fingerprint) DO UPDATE SET action=excluded.action", params![input.article_id,input.revision,input.fingerprint,input.action]).map_err(collection::err)?;
    tx.commit().map_err(collection::err)?;
    Ok(Json(AckResponse {
        article_id: input.article_id,
        event_key: event_key(input.article_id, input.revision, &input.fingerprint),
        revision: input.revision,
        fingerprint: input.fingerprint,
        action: input.action,
    }))
}

#[derive(Debug)]
struct Query {
    hours: i64,
    limit: usize,
    feed_id: Option<i64>,
    folder_id: Option<i64>,
}
fn parse_query(raw: Option<String>) -> Result<Query, StatusCode> {
    let raw = raw.unwrap_or_default();
    if raw.len() > 256 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut q = Query {
        hours: 24,
        limit: 10,
        feed_id: None,
        folder_id: None,
    };
    let mut seen = HashSet::new();
    for (k, v) in url::form_urlencoded::parse(raw.as_bytes()) {
        if !seen.insert(k.to_string()) {
            return Err(StatusCode::BAD_REQUEST);
        }
        match k.as_ref() {
            "hours" => q.hours = v.parse().map_err(|_| StatusCode::BAD_REQUEST)?,
            "limit" => q.limit = v.parse().map_err(|_| StatusCode::BAD_REQUEST)?,
            "feed_id" => q.feed_id = Some(v.parse().map_err(|_| StatusCode::BAD_REQUEST)?),
            "folder_id" => q.folder_id = Some(v.parse().map_err(|_| StatusCode::BAD_REQUEST)?),
            _ => return Err(StatusCode::BAD_REQUEST),
        }
    }
    if !matches!(q.hours, 24 | 168)
        || !matches!(q.limit, 10 | 20)
        || q.feed_id.is_some() && q.folder_id.is_some()
        || q.feed_id
            .or(q.folder_id)
            .is_some_and(|v| !(1..=9_007_199_254_740_991).contains(&v))
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    Ok(q)
}
#[derive(Serialize)]
struct Reason {
    code: &'static str,
    value: serde_json::Value,
}
#[derive(Serialize)]
struct Item {
    article: Article,
    event_key: String,
    fingerprint: String,
    revision: i64,
    kind: &'static str,
    reasons: Vec<Reason>,
    related_count: usize,
}
#[derive(Serialize)]
struct Catchup {
    generated_at: String,
    window_start: String,
    window_end: String,
    limit: usize,
    policy_version: &'static str,
    scanned: usize,
    eligible_count: usize,
    truncated: bool,
    collection_incomplete: bool,
    items: Vec<Item>,
}

async fn list(State(db): State<Db>, RawQuery(raw): RawQuery) -> Result<Json<Catchup>, StatusCode> {
    let q = parse_query(raw)?;
    let db = db.lock().map_err(collection::err)?;
    for (table, id) in [("feeds", q.feed_id), ("folders", q.folder_id)] {
        if let Some(id) = id
            && !db
                .query_row(
                    &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE id=?1)"),
                    [id],
                    |r| r.get::<_, bool>(0),
                )
                .map_err(collection::err)?
        {
            return Err(StatusCode::BAD_REQUEST);
        }
    }
    let (end, start): (String, String) = db
        .query_row(
            "SELECT strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now',?1)",
            [format!("-{} hours", q.hours)],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(collection::err)?;
    let prefs = preferences(&db).map_err(collection::err)?;
    let incomplete = crate::collection_runtime::is_incomplete(&db, q.feed_id, q.folder_id)
        .map_err(collection::err)?;
    let sql = format!(
        r#"SELECT articles.id,articles.title,articles.content,articles.feed_id,feeds.url,
        CASE WHEN state.changed_at IS NOT NULL AND visits.visited_at IS NOT NULL AND julianday(state.changed_at)>julianday(visits.visited_at) AND julianday(state.changed_at) BETWEEN julianday(?1) AND julianday(?2) THEN 1 ELSE 0 END AS updated,
        CASE WHEN state.changed_at IS NOT NULL AND visits.visited_at IS NOT NULL AND julianday(state.changed_at)>julianday(visits.visited_at) AND julianday(state.changed_at) BETWEEN julianday(?1) AND julianday(?2) THEN julianday(state.changed_at) ELSE julianday(articles.published) END AS event_time,
        CASE WHEN story.article_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM article_story_exclusions ex WHERE ex.article_id=articles.id) AND (
          (story.canonical_url IS NOT NULL AND EXISTS(SELECT 1 FROM article_story_index other WHERE other.canonical_url=story.canonical_url AND other.article_id<>articles.id)) OR
          (story.body_fingerprint IS NOT NULL AND EXISTS(SELECT 1 FROM article_story_index other WHERE other.body_fingerprint=story.body_fingerprint AND other.title_key=story.title_key AND other.article_id<>articles.id AND abs((julianday(other.published)-julianday(story.published))*24)<=48))
        ) THEN 1 ELSE 0 END AS has_related
      FROM articles JOIN article_revision_state state ON state.article_id=articles.id
      JOIN feeds ON feeds.id=articles.feed_id LEFT JOIN article_visits visits ON visits.article_id=articles.id LEFT JOIN article_story_index story ON story.article_id=articles.id
      WHERE (?3 IS NULL OR articles.feed_id=?3) AND (?4 IS NULL OR feeds.folder_id=?4)
        AND NOT ({})
        AND NOT EXISTS(SELECT 1 FROM catchup_acknowledgements ack WHERE ack.article_id=articles.id AND ack.revision=state.current_revision AND ack.fingerprint=state.fingerprint)
        AND ((state.changed_at IS NOT NULL AND visits.visited_at IS NOT NULL AND julianday(state.changed_at)>julianday(visits.visited_at) AND julianday(state.changed_at) BETWEEN julianday(?1) AND julianday(?2))
          OR (articles.is_read=0 AND state.ingestion_kind='normal' AND julianday(articles.published) BETWEEN julianday(?1) AND julianday(?2)))
      ORDER BY event_time DESC,articles.id DESC LIMIT 5001"#,
        crate::mute_rules::HIDDEN_SQL
    );
    let mut stmt = db.prepare(&sql).map_err(collection::err)?;
    let mut rows = stmt
        .query(params![start, end, q.feed_id, q.folder_id])
        .map_err(collection::err)?;
    let mut candidates = Vec::new();
    let mut truncated = false;
    while let Some(r) = rows.next().map_err(collection::err)? {
        if candidates.len() == 5000 {
            truncated = true;
            break;
        }
        let id: i64 = r.get(0).map_err(collection::err)?;
        let title: String = r.get(1).map_err(collection::err)?;
        let content: String = r.get(2).map_err(collection::err)?;
        let feed_id: Option<i64> = r.get(3).map_err(collection::err)?;
        let feed_url: String = r.get(4).map_err(collection::err)?;
        let host = url::Url::parse(&feed_url)
            .ok()
            .and_then(|u| u.host_str().map(|h| h.to_ascii_lowercase()))
            .unwrap_or_else(|| "(unknown)".into());
        candidates.push(Ranked {
            id,
            host,
            terms: matched_terms(&title, &content, &prefs.terms),
            priority: feed_id.is_some_and(|id| prefs.priority_feed_ids.contains(&id)),
            updated: r.get(5).map_err(collection::err)?,
            event_time: r.get(6).map_err(collection::err)?,
            has_related: r.get(7).map_err(collection::err)?,
        });
    }
    drop(rows);
    drop(stmt);
    let scanned = candidates.len();
    // Grouping precedes the host cap; related members remain independent reader records.
    candidates.sort_by(|a, b| {
        (!b.terms.is_empty())
            .cmp(&(!a.terms.is_empty()))
            .then_with(|| b.priority.cmp(&a.priority))
            .then_with(|| b.updated.cmp(&a.updated))
            .then_with(|| b.event_time.total_cmp(&a.event_time))
            .then_with(|| b.id.cmp(&a.id))
    });
    let mut covered = HashSet::new();
    let mut ungrouped = Vec::new();
    let mut counts = HashMap::new();
    let mut expansions = 0;
    for candidate in candidates {
        if covered.contains(&candidate.id) {
            continue;
        }
        if candidate.has_related {
            if expansions == RELATED_EXPANSION_LIMIT {
                truncated = true;
                break;
            }
            expansions += 1;
            let related = crate::story_groups::related_ids(&db, candidate.id, 1000)
                .map_err(collection::err)?;
            for member in related.members {
                covered.insert(member.article_id);
            }
            if related.truncated {
                truncated = true;
            }
            counts.insert(candidate.id, related.total);
        }
        ungrouped.push(candidate);
    }
    let eligible_count = ungrouped.len();
    let selected = rank(ungrouped, q.limit);
    let mut items = Vec::new();
    for c in selected {
        let article = lookup(&db, c.id)?;
        let state = crate::article_revisions::current_state(&db, c.id)
            .map_err(collection::err)?
            .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
        let mut reasons = Vec::new();
        for term in c.terms {
            reasons.push(Reason {
                code: "interest_term",
                value: term.into(),
            });
        }
        if c.priority {
            reasons.push(Reason {
                code: "priority_feed",
                value: article.feed_id.into(),
            });
        }
        reasons.push(Reason {
            code: if c.updated {
                "updated_since_visit"
            } else {
                "recent_unread"
            },
            value: serde_json::Value::Null,
        });
        reasons.push(Reason {
            code: "source_diversity",
            value: c.host.into(),
        });
        items.push(Item {
            article,
            event_key: event_key(c.id, state.current_revision, &state.fingerprint),
            fingerprint: state.fingerprint,
            revision: state.current_revision,
            kind: if c.updated { "updated" } else { "new" },
            reasons,
            related_count: *counts.get(&c.id).unwrap_or(&0),
        });
    }
    Ok(Json(Catchup {
        generated_at: end.clone(),
        window_start: start,
        window_end: end,
        limit: q.limit,
        policy_version: "1",
        scanned,
        eligible_count,
        truncated,
        collection_incomplete: incomplete,
        items,
    }))
}
pub fn routes() -> Router<Db> {
    Router::new()
        .route("/api/catchup", get(list))
        .route(
            "/api/catchup/interests",
            get(get_interests).put(put_interests),
        )
        .route(
            "/api/catchup/acknowledgements",
            axum::routing::put(put_ack).delete(delete_ack),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn setup() -> Db {
        let mut db = Connection::open_in_memory().unwrap();
        db.execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE folders(id INTEGER PRIMARY KEY); CREATE TABLE feeds(id INTEGER PRIMARY KEY,url TEXT,title TEXT,folder_id INTEGER,paused INTEGER DEFAULT 0,last_error TEXT,last_checked TEXT); CREATE TABLE articles(id INTEGER PRIMARY KEY,title TEXT NOT NULL,content TEXT NOT NULL,url TEXT NOT NULL,published TEXT NOT NULL,is_read INTEGER NOT NULL DEFAULT 0,is_starred INTEGER NOT NULL DEFAULT 0,feed_id INTEGER REFERENCES feeds(id) ON DELETE CASCADE,external_id TEXT, UNIQUE(feed_id,external_id)); CREATE TABLE article_visits(article_id INTEGER PRIMARY KEY,visited_at TEXT); CREATE TABLE article_read_later(article_id INTEGER PRIMARY KEY); CREATE TABLE collection_settings(id INTEGER PRIMARY KEY,enabled INTEGER,interval_seconds INTEGER); INSERT INTO collection_settings VALUES(1,1,3600); CREATE TABLE collection_health(feed_id INTEGER PRIMARY KEY,consecutive_failures INTEGER,last_success TEXT); CREATE TABLE feed_poll_intervals(feed_id INTEGER PRIMARY KEY,interval_seconds INTEGER); CREATE TABLE social_sources(feed_id INTEGER PRIMARY KEY,provider TEXT,server TEXT,remote_id TEXT); CREATE TABLE collection_runtime(feed_id INTEGER PRIMARY KEY,next_retry INTEGER,more_pending INTEGER,gap_possible INTEGER); INSERT INTO folders VALUES(1); INSERT INTO feeds(id,url,title,folder_id,last_checked) VALUES(1,'https://one.test/feed','one',1,datetime('now')),(2,'https://two.test/feed','two',1,datetime('now'));").unwrap();
        let tx = db.transaction().unwrap();
        crate::mute_rules::migrate(&tx).unwrap();
        crate::article_revisions::migrate_catchup(&tx).unwrap();
        crate::story_groups::migrate_catchup(&tx).unwrap();
        migrate_catchup(&tx).unwrap();
        tx.commit().unwrap();
        Arc::new(Mutex::new(db))
    }
    #[test]
    fn ranking_literal_unicode_and_host_cap() {
        let terms = vec!["AI".to_owned(), "日本語".to_owned(), "100%".to_owned()];
        assert_eq!(matched_terms("日本語 AI", "100%", &terms), terms);
        assert!(matched_terms("100x and 人工知能", "", &["100%".into(), "AI".into()]).is_empty());
        let r = |id, host: &str, term, priority, updated| Ranked {
            id,
            host: host.into(),
            terms: if term { vec!["x".into()] } else { vec![] },
            priority,
            updated,
            event_time: id as f64,
            has_related: false,
        };
        let ranked = rank(
            vec![
                r(1, "a", true, false, false),
                r(2, "a", true, true, false),
                r(3, "a", true, false, true),
                r(4, "b", false, true, true),
            ],
            10,
        );
        assert_eq!(
            ranked.iter().map(|x| x.id).collect::<Vec<_>>(),
            vec![2, 3, 4]
        );
        assert!(parse_query(Some("hours=24&hours=168".into())).is_err());
        assert!(parse_query(Some("feed_id=1&folder_id=1".into())).is_err());
    }
    #[tokio::test]
    async fn settings_conflict_and_feed_removal() {
        let db = setup();
        let p = Preferences {
            revision: 1,
            terms: vec!["日本語".into(), "AI".into()],
            priority_feed_ids: vec![1],
        };
        let result = put_interests(State(db.clone()), HeaderMap::new(), Json(p))
            .await
            .unwrap()
            .0;
        assert_eq!(result.revision, 2);
        assert_eq!(
            put_interests(
                State(db.clone()),
                HeaderMap::new(),
                Json(Preferences {
                    revision: 1,
                    terms: vec![],
                    priority_feed_ids: vec![]
                })
            )
            .await
            .err(),
            Some(StatusCode::CONFLICT)
        );
        assert_eq!(
            put_interests(
                State(db.clone()),
                HeaderMap::new(),
                Json(Preferences {
                    revision: 2,
                    terms: vec!["あ".repeat(81)],
                    priority_feed_ids: vec![]
                })
            )
            .await
            .err(),
            Some(StatusCode::BAD_REQUEST)
        );
        {
            let db = db.lock().unwrap();
            db.execute("DELETE FROM feeds WHERE id=1", []).unwrap();
        }
        let p = get_interests(State(db)).await.unwrap().0;
        assert_eq!(p.revision, 3);
        assert!(p.priority_feed_ids.is_empty());
        assert_eq!(p.terms.len(), 2);
    }
    #[tokio::test]
    async fn acknowledgement_is_versioned_and_non_destructive() {
        let db = setup();
        {
            let db = db.lock().unwrap();
            db.execute_batch("INSERT INTO articles(id,title,content,url,published,feed_id,is_read,is_starred) VALUES(1,'t','body','https://a.test','2026-01-01',1,1,1); INSERT INTO article_visits VALUES(1,'2026-01-01'); INSERT INTO article_read_later VALUES(1); INSERT INTO article_revision_state VALUES(1,2,'new','2026-01-01','normal','2026-01-02',NULL,0);").unwrap();
        }
        let input = |revision, fingerprint: &str| AckInput {
            article_id: 1,
            revision,
            fingerprint: fingerprint.into(),
            action: "confirmed".into(),
        };
        assert_eq!(
            put_ack(State(db.clone()), HeaderMap::new(), Json(input(1, "old")))
                .await
                .err(),
            Some(StatusCode::CONFLICT)
        );
        let _ = put_ack(State(db.clone()), HeaderMap::new(), Json(input(2, "new")))
            .await
            .ok()
            .unwrap();
        for _ in 0..2 {
            delete_ack(
                State(db.clone()),
                HeaderMap::new(),
                Json(AckKey {
                    article_id: 1,
                    revision: 1,
                    fingerprint: "old".into(),
                }),
            )
            .await
            .unwrap();
        }
        let c = db.lock().unwrap();
        assert_eq!(
            c.query_row("SELECT count(*) FROM catchup_acknowledgements", [], |r| r
                .get::<_, i64>(
                0
            ))
            .unwrap(),
            1
        );
        assert_eq!(c.query_row("SELECT is_read+is_starred+(SELECT count(*) FROM article_visits)+(SELECT count(*) FROM article_read_later) FROM articles WHERE id=1",[],|r|r.get::<_,i64>(0)).unwrap(),4);
    }
    #[tokio::test]
    async fn selection_backfill_mute_update_ack_and_groups() {
        let db = setup();
        {
            let c = db.lock().unwrap();
            c.execute_batch("INSERT INTO articles(id,title,content,url,published,feed_id,is_read) VALUES
             (1,'日本語 AI','body','https://a.test/1',datetime('now','-1 hour'),1,0),
             (2,'initial','body','https://a.test/2',datetime('now','-1 hour'),1,0),
             (3,'recovery','body','https://a.test/3',datetime('now','-1 hour'),1,0),
             (4,'muted','body','https://a.test/4',datetime('now','-1 hour'),2,0),
             (5,'future','body','https://a.test/5',datetime('now','+1 hour'),2,0),
             (6,'unknown','body','https://a.test/6','not-a-date',2,0),
             (7,'read changed','body','https://a.test/7',datetime('now','-5 days'),2,1),
             (8,'copy','body','https://a.test/1',datetime('now','-1 hour'),2,0);
             INSERT INTO article_revision_state SELECT id,1,'fp'||id,datetime('now'),CASE id WHEN 2 THEN 'initial' WHEN 3 THEN 'recovery' ELSE 'normal' END,CASE WHEN id=7 THEN datetime('now','-5 minutes') ELSE NULL END,NULL,0 FROM articles;
             INSERT INTO article_visits VALUES(7,datetime('now','-1 hour'));
             INSERT INTO mute_rules(name,term,field,enabled,revision) VALUES('hide','muted','title',1,1);
             INSERT INTO article_story_index(article_id,canonical_url,body_fingerprint,title_key,published,source_key) SELECT id,url,NULL,title,published,NULL FROM articles;").unwrap();
        }
        let result = list(State(db.clone()), RawQuery(None)).await.unwrap().0;
        assert_eq!(result.scanned, 3);
        assert_eq!(result.eligible_count, 2);
        assert_eq!(result.items.len(), 2);
        assert!(
            result
                .items
                .iter()
                .any(|i| i.article.id == 7 && i.kind == "updated")
        );
        assert!(result.items.iter().any(|i| i.related_count == 1));
        let chosen = result.items.iter().find(|i| i.article.id == 7).unwrap();
        let _ = put_ack(
            State(db.clone()),
            HeaderMap::new(),
            Json(AckInput {
                article_id: 7,
                revision: chosen.revision,
                fingerprint: chosen.fingerprint.clone(),
                action: "skipped".into(),
            }),
        )
        .await
        .unwrap();
        assert!(
            !list(State(db.clone()), RawQuery(None))
                .await
                .unwrap()
                .0
                .items
                .iter()
                .any(|i| i.article.id == 7)
        );
        {
            let c = db.lock().unwrap();
            c.execute("UPDATE article_revision_state SET current_revision=2,fingerprint='fp7-new',changed_at=datetime('now','-1 minute') WHERE article_id=7",[]).unwrap();
        }
        assert_eq!(
            put_ack(
                State(db.clone()),
                HeaderMap::new(),
                Json(AckInput {
                    article_id: 7,
                    revision: 1,
                    fingerprint: "fp7".into(),
                    action: "confirmed".into()
                })
            )
            .await
            .err(),
            Some(StatusCode::CONFLICT)
        );
        let again = list(State(db.clone()), RawQuery(None)).await.unwrap().0;
        assert!(
            again
                .items
                .iter()
                .any(|i| i.article.id == 7 && i.revision == 2 && i.kind == "updated")
        );
        {
            let c = db.lock().unwrap();
            c.execute("INSERT INTO article_story_exclusions VALUES(8)", [])
                .unwrap();
        }
        let independent = list(State(db.clone()), RawQuery(None)).await.unwrap().0;
        assert_eq!(independent.eligible_count, 3);
        let c = db.lock().unwrap();
        assert_eq!(
            c.query_row("SELECT is_read FROM articles WHERE id=7", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            c.query_row("SELECT count(*) FROM article_visits", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
    }
    #[tokio::test]
    async fn bounded_scan_and_real_host_cap() {
        let db = setup();
        {
            let c = db.lock().unwrap();
            c.execute_batch("WITH RECURSIVE n(x) AS (VALUES(1) UNION ALL SELECT x+1 FROM n WHERE x<5001) INSERT INTO articles(id,title,content,url,published,feed_id) SELECT x,'article','body','https://example.test/'||x,datetime('now','-1 hour'),CASE WHEN x%2=0 THEN 1 ELSE 2 END FROM n; INSERT INTO article_revision_state SELECT id,1,'fp'||id,datetime('now'),'normal',NULL,NULL,0 FROM articles; INSERT INTO article_story_index(article_id,canonical_url,body_fingerprint,title_key,published,source_key) SELECT id,url,NULL,title,published,NULL FROM articles;").unwrap();
        }
        let started = std::time::Instant::now();
        let result = list(State(db), RawQuery(Some("hours=24&limit=20".into())))
            .await
            .unwrap()
            .0;
        eprintln!("5001 synthetic candidates: {:?}", started.elapsed());
        assert_eq!(result.scanned, 5000);
        assert_eq!(result.eligible_count, 5000);
        assert!(result.truncated);
        assert_eq!(result.items.len(), 4);
        for feed in [1, 2] {
            assert_eq!(
                result
                    .items
                    .iter()
                    .filter(|i| i.article.feed_id == Some(feed))
                    .count(),
                2
            );
        }
    }
    #[tokio::test]
    async fn http_validates_payload_and_stale_version() {
        use tower::ServiceExt;
        let db = setup();
        {
            let c = db.lock().unwrap();
            c.execute_batch("INSERT INTO articles(id,title,content,url,published,feed_id) VALUES(1,'title','body','https://a.test',datetime('now'),1); INSERT INTO article_revision_state VALUES(1,2,'current',datetime('now'),'normal',NULL,NULL,0);").unwrap();
        }
        let app = routes().with_state(db.clone());
        let req = |method: &str, uri: &str, body: &str| {
            axum::http::Request::builder()
                .method(method)
                .uri(uri)
                .header("content-type", "application/json")
                .body(axum::body::Body::from(body.to_string()))
                .unwrap()
        };
        let response = app
            .clone()
            .oneshot(req(
                "PUT",
                "/api/catchup/acknowledgements",
                r#"{"article_id":1,"revision":1,"fingerprint":"old","action":"confirmed"}"#,
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);
        let response = app
            .clone()
            .oneshot(req(
                "PUT",
                "/api/catchup/interests",
                r#"{"revision":1,"terms":[],"priority_feed_ids":[999]}"#,
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let response = app
            .oneshot(req("GET", "/api/catchup?limit=100", ""))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
    #[tokio::test]
    async fn expensive_related_expansion_has_a_transparent_bound() {
        let db = setup();
        {
            let c = db.lock().unwrap();
            c.execute_batch("WITH RECURSIVE n(x) AS (VALUES(1) UNION ALL SELECT x+1 FROM n WHERE x<202) INSERT INTO articles(id,title,content,url,published,feed_id,is_read) SELECT x,'article','body','https://example.test/'||((x+1)/2),datetime('now','-1 hour'),1,(x%2=0) FROM n; INSERT INTO article_revision_state SELECT id,1,'fp'||id,datetime('now'),'normal',NULL,NULL,0 FROM articles; INSERT INTO article_story_index(article_id,canonical_url,body_fingerprint,title_key,published,source_key) SELECT id,url,NULL,title,published,NULL FROM articles;").unwrap();
        }
        let result = list(State(db), RawQuery(None)).await.unwrap().0;
        assert_eq!(result.scanned, 101);
        assert_eq!(result.eligible_count, 100);
        assert!(result.truncated);
        assert_eq!(result.items.len(), 2);
    }
    #[test]
    fn acknowledgement_and_revision_change_are_serialized() {
        let db = setup();
        {
            let c = db.lock().unwrap();
            c.execute_batch("INSERT INTO articles(id,title,content,url,published,feed_id) VALUES(1,'title','body','https://a.test',datetime('now'),1); INSERT INTO article_revision_state VALUES(1,1,'old',datetime('now'),'normal',NULL,NULL,0);").unwrap();
        }
        let barrier = Arc::new(std::sync::Barrier::new(2));
        let write_db = db.clone();
        let write_barrier = barrier.clone();
        let update = std::thread::spawn(move || {
            write_barrier.wait();
            let c = write_db.lock().unwrap();
            c.execute("UPDATE article_revision_state SET current_revision=2,fingerprint='new' WHERE article_id=1",[]).unwrap();
        });
        let ack_db = db.clone();
        let ack = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .build()
                .unwrap();
            barrier.wait();
            rt.block_on(put_ack(
                State(ack_db),
                HeaderMap::new(),
                Json(AckInput {
                    article_id: 1,
                    revision: 1,
                    fingerprint: "old".into(),
                    action: "confirmed".into(),
                }),
            ))
            .map(|_| ())
            .err()
        });
        update.join().unwrap();
        let status = ack.join().unwrap();
        assert!(status.is_none() || status == Some(StatusCode::CONFLICT));
        let c = db.lock().unwrap();
        assert_eq!(c.query_row("SELECT count(*) FROM catchup_acknowledgements WHERE revision=2 OR fingerprint='new'",[],|r|r.get::<_,i64>(0)).unwrap(),0);
    }
    #[test]
    fn competing_connections_recheck_revision_after_writer_commits() {
        let original = setup();
        let file = tempfile::NamedTempFile::new().unwrap();
        let mut dest = Connection::open(file.path()).unwrap();
        {
            let src = original.lock().unwrap();
            rusqlite::backup::Backup::new(&src, &mut dest)
                .unwrap()
                .run_to_completion(100, std::time::Duration::from_millis(1), None)
                .unwrap();
        }
        dest.pragma_update(None, "journal_mode", "WAL").unwrap();
        dest.busy_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        let second = Connection::open(file.path()).unwrap();
        second
            .busy_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        let barrier = Arc::new(std::sync::Barrier::new(2));
        let worker = |conn: Connection, barrier: Arc<std::sync::Barrier>, term: &'static str| {
            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .build()
                    .unwrap();
                barrier.wait();
                rt.block_on(put_interests(
                    State(Arc::new(Mutex::new(conn))),
                    HeaderMap::new(),
                    Json(Preferences {
                        revision: 1,
                        terms: vec![term.into()],
                        priority_feed_ids: vec![],
                    }),
                ))
                .map(|_| ())
                .err()
            })
        };
        let a = worker(dest, barrier.clone(), "one");
        let b = worker(second, barrier, "two");
        let results = [a.join().unwrap(), b.join().unwrap()];
        assert_eq!(results.iter().filter(|x| x.is_none()).count(), 1);
        assert_eq!(
            results
                .iter()
                .filter(|x| **x == Some(StatusCode::CONFLICT))
                .count(),
            1
        );
        let check = Connection::open(file.path()).unwrap();
        assert_eq!(preferences(&check).unwrap().revision, 2);
    }
}
