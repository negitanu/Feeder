use super::*;
use axum::{Extension, response::Response};
use html5ever::{
    tendril::StrTendril,
    tokenizer::{
        BufferQueue, TagKind, Token, TokenSink, TokenSinkResult, Tokenizer, TokenizerOpts,
    },
};
use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};
use url::Url;

#[derive(Default)]
struct Plans(Mutex<HashMap<String, Plan>>);
struct Plan {
    session: String,
    url: String,
    expires: Instant,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UrlInput {
    url: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Confirm {
    token: String,
    folder_id: Option<i64>,
    title: Option<String>,
}
#[derive(Serialize)]
struct Candidate {
    url: String,
    title: String,
}
#[derive(Serialize)]
struct Candidates {
    candidates: Vec<Candidate>,
}
#[derive(Serialize)]
struct Preview {
    token: String,
    url: String,
    title: String,
    entries: Vec<Candidate>,
    expires_in: u64,
}
pub(super) fn random() -> Result<String, StatusCode> {
    let mut bytes = [0u8; 32];
    getrandom::getrandom(&mut bytes).map_err(collection::err)?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}
fn session(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .filter_map(|v| v.trim().split_once('='))
        .find(|(k, _)| *k == "feeder_session")
        .map(|(_, v)| v.to_owned())
        .filter(|v| v.len() == 64 && v.bytes().all(|b| b.is_ascii_hexdigit()))
}
fn safe_url(base: &Url, value: &str) -> Option<String> {
    let mut u = base.join(value).ok()?;
    if !matches!(u.scheme(), "http" | "https")
        || !u.username().is_empty()
        || u.password().is_some()
        || u.host_str().is_none()
    {
        return None;
    }
    u.set_fragment(None);
    Some(u.to_string())
}
// TokenSink uses interior mutability because the tokenizer borrows its sink immutably.
#[derive(Default)]
struct LinkSink(std::cell::RefCell<Vec<(String, String)>>);
impl TokenSink for LinkSink {
    type Handle = ();
    fn process_token(&self, token: Token, _: u64) -> TokenSinkResult<()> {
        if let Token::TagToken(tag) = token
            && tag.kind == TagKind::StartTag
            && tag.name.as_ref().eq_ignore_ascii_case("link")
        {
            let mut rel = String::new();
            let mut kind = String::new();
            let mut href = String::new();
            let mut title = String::new();
            for a in tag.attrs {
                match a.name.local.as_ref() {
                    "rel" => rel = a.value.to_string(),
                    "type" => kind = a.value.to_string(),
                    "href" => href = a.value.to_string(),
                    "title" => title = a.value.to_string(),
                    _ => {}
                }
            }
            if rel
                .split_ascii_whitespace()
                .any(|v| v.eq_ignore_ascii_case("alternate"))
                && matches!(
                    kind.to_ascii_lowercase()
                        .split(';')
                        .next()
                        .unwrap_or("")
                        .trim(),
                    "application/rss+xml" | "application/atom+xml"
                )
                && !href.is_empty()
            {
                self.0.borrow_mut().push((href, title));
            }
        }
        TokenSinkResult::Continue
    }
}
fn alternates(body: &[u8], base: &Url) -> Vec<Candidate> {
    let sink = LinkSink::default();
    let tokenizer = Tokenizer::new(sink, TokenizerOpts::default());
    let queue = BufferQueue::default();
    queue.push_back(StrTendril::from(String::from_utf8_lossy(body).as_ref()));
    let _ = tokenizer.feed(&queue);
    tokenizer.end();
    let mut seen = HashSet::new();
    tokenizer
        .sink
        .0
        .into_inner()
        .into_iter()
        .filter_map(|(href, title)| {
            let url = safe_url(base, &href)?;
            if !seen.insert(url.clone()) {
                return None;
            }
            Some(Candidate {
                url,
                title: collection::plain(&title),
            })
        })
        .take(8)
        .collect()
}
async fn discover(
    headers: HeaderMap,
    Json(input): Json<UrlInput>,
) -> Result<Json<Candidates>, StatusCode> {
    if !origin_allowed(&headers) {
        return Err(StatusCode::FORBIDDEN);
    }
    let response = collection::fetch_body(&input.url).await?;
    let body = response.body.ok_or(StatusCode::BAD_GATEWAY)?;
    let candidates = if let Ok(feed) = collection::parse_feed(&body, &response.final_url) {
        vec![Candidate {
            url: response.final_url.to_string(),
            title: feed
                .title
                .as_ref()
                .map(|t| collection::plain(&t.content))
                .unwrap_or_default(),
        }]
    } else {
        alternates(&body, &response.final_url)
    };
    Ok(Json(Candidates { candidates }))
}
async fn preview(
    Extension(plans): Extension<Arc<Plans>>,
    headers: HeaderMap,
    Json(input): Json<UrlInput>,
) -> Result<Response, StatusCode> {
    if !origin_allowed(&headers) {
        return Err(StatusCode::FORBIDDEN);
    }
    let response = collection::fetch_body(&input.url).await?;
    let body = response.body.ok_or(StatusCode::BAD_GATEWAY)?;
    let feed = collection::parse_feed(&body, &response.final_url)?;
    let url = response.final_url.to_string();
    let title = feed
        .title
        .as_ref()
        .map(|t| collection::plain(&t.content))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| url.clone());
    let entries = feed
        .entries
        .iter()
        .take(5)
        .map(|e| Candidate {
            title: e
                .title
                .as_ref()
                .map(|t| collection::plain(&t.content))
                .unwrap_or_default(),
            url: e
                .links
                .first()
                .and_then(|l| safe_url(&response.final_url, &l.href))
                .unwrap_or_default(),
        })
        .collect();
    let token = random()?;
    let existing_session = session(&headers);
    let sid = match &existing_session {
        Some(v) => v.clone(),
        None => random()?,
    };
    {
        let mut p = plans.0.lock().map_err(collection::err)?;
        p.retain(|_, v| v.expires > Instant::now());
        if p.len() >= 128 {
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
        p.insert(
            token.clone(),
            Plan {
                session: sid.clone(),
                url: url.clone(),
                expires: Instant::now() + Duration::from_secs(900),
            },
        );
    }
    let mut response = Json(Preview {
        token,
        url,
        title,
        entries,
        expires_in: 900,
    })
    .into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        format!("feeder_session={sid}; HttpOnly; SameSite=Strict; Path=/; Max-Age=900")
            .parse()
            .map_err(collection::err)?,
    );
    Ok(response)
}
async fn confirm(
    State(db): State<Db>,
    Extension(plans): Extension<Arc<Plans>>,
    headers: HeaderMap,
    Json(input): Json<Confirm>,
) -> Result<Json<collection::Feed>, StatusCode> {
    if !origin_allowed(&headers) {
        return Err(StatusCode::FORBIDDEN);
    }
    if let Some(title) = input
        .title
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        collection::clean_name(title)?;
    }
    let sid = session(&headers).ok_or(StatusCode::FORBIDDEN)?;
    let url = {
        let mut p = plans.0.lock().map_err(collection::err)?;
        let plan = p.get(&input.token).ok_or(StatusCode::FORBIDDEN)?;
        if plan.session != sid || plan.expires <= Instant::now() {
            return Err(StatusCode::FORBIDDEN);
        }
        p.remove(&input.token).unwrap().url
    };
    Ok(Json(
        collection::add_named_feed(db, &url, input.folder_id, input.title.as_deref()).await?,
    ))
}
pub fn routes() -> Router<Db> {
    Router::new()
        .route("/api/discover", post(discover))
        .route("/api/preview", post(preview))
        .route("/api/preview/confirm", post(confirm))
        .layer(Extension(Arc::new(Plans::default())))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn discovery_resolves_deduplicates_and_limits() {
        let base = Url::parse("https://example.com/redirected/page").unwrap();
        let html = (0..12).map(|i| format!("<link rel='alternate' type='application/rss+xml' href='../feed{i}' title='Feed {i}'>")).collect::<String>() + "<link rel='alternate' type='application/atom+xml' href='../feed0'><link rel='alternate' type='application/rss+xml' href='javascript:bad'>";
        let links = alternates(html.as_bytes(), &base);
        assert_eq!(links.len(), 8);
        assert_eq!(links[0].url, "https://example.com/feed0");
    }
    #[tokio::test]
    async fn invalid_plans_never_write() {
        let dir = tempfile::tempdir().unwrap();
        let db = Arc::new(Mutex::new(open_db(&dir.path().join("db"), false).unwrap()));
        let plans = Arc::new(Plans::default());
        let sid = "a".repeat(64);
        let other = "b".repeat(64);
        plans.0.lock().unwrap().insert(
            "token".into(),
            Plan {
                session: sid.clone(),
                url: "https://example.com/feed".into(),
                expires: Instant::now() + Duration::from_secs(900),
            },
        );
        let headers = |v: &str| {
            let mut h = HeaderMap::new();
            h.insert(
                header::COOKIE,
                format!("feeder_session={v}").parse().unwrap(),
            );
            h
        };
        let invalid_title = confirm(
            State(db.clone()),
            Extension(plans.clone()),
            headers(&sid),
            Json(Confirm {
                token: "token".into(),
                folder_id: None,
                title: Some("x".repeat(513)),
            }),
        )
        .await;
        assert!(matches!(invalid_title, Err(StatusCode::BAD_REQUEST)));
        assert!(plans.0.lock().unwrap().contains_key("token"));
        for (token, session) in [("tampered", sid.as_str()), ("token", other.as_str())] {
            let result = confirm(
                State(db.clone()),
                Extension(plans.clone()),
                headers(session),
                Json(Confirm {
                    token: token.into(),
                    folder_id: None,
                    title: None,
                }),
            )
            .await;
            assert!(matches!(result, Err(StatusCode::FORBIDDEN)));
        }
        plans.0.lock().unwrap().get_mut("token").unwrap().expires =
            Instant::now() - Duration::from_secs(1);
        assert!(matches!(
            confirm(
                State(db.clone()),
                Extension(plans.clone()),
                headers(&sid),
                Json(Confirm {
                    token: "token".into(),
                    folder_id: None,
                    title: None
                })
            )
            .await,
            Err(StatusCode::FORBIDDEN)
        ));
        plans.0.lock().unwrap().insert(
            "once".into(),
            Plan {
                session: sid.clone(),
                url: "http://127.0.0.1/feed".into(),
                expires: Instant::now() + Duration::from_secs(900),
            },
        );
        assert!(
            confirm(
                State(db.clone()),
                Extension(plans.clone()),
                headers(&sid),
                Json(Confirm {
                    token: "once".into(),
                    folder_id: None,
                    title: None
                })
            )
            .await
            .is_err()
        );
        assert!(matches!(
            confirm(
                State(db.clone()),
                Extension(plans.clone()),
                headers(&sid),
                Json(Confirm {
                    token: "once".into(),
                    folder_id: None,
                    title: None
                })
            )
            .await,
            Err(StatusCode::FORBIDDEN)
        ));
        assert_eq!(
            db.lock()
                .unwrap()
                .query_row("SELECT count(*) FROM feeds", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
}
