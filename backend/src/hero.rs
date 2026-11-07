use super::*;
use axum::{body::Body, response::Response};
use html5ever::{
    tendril::StrTendril,
    tokenizer::{
        BufferQueue, TagKind, Token, TokenSink, TokenSinkResult, Tokenizer, TokenizerOpts,
    },
};
use std::{
    collections::HashMap,
    sync::OnceLock,
    time::{Duration, Instant},
};
use url::Url;

// Cache both misses and hits. The URL is part of the key so edited articles never reuse old images.
type Image = (String, Vec<u8>);
struct Entry {
    expires: Instant,
    image: Option<Image>,
}
static CACHE: OnceLock<Mutex<HashMap<(i64, String), Entry>>> = OnceLock::new();
static LIMIT: OnceLock<tokio::sync::Semaphore> = OnceLock::new();

pub(super) fn candidate(id: i64, raw: &str) -> Option<String> {
    let url = Url::parse(raw).ok()?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return None;
    }
    Some(format!("/api/articles/{id}/hero"))
}

#[derive(Default)]
struct Meta(std::cell::RefCell<Vec<(String, String)>>);
impl TokenSink for Meta {
    type Handle = ();
    fn process_token(&self, token: Token, _: u64) -> TokenSinkResult<()> {
        if let Token::TagToken(tag) = token
            && tag.kind == TagKind::StartTag
            && tag.name.as_ref() == "meta"
        {
            let mut name = String::new();
            let mut content = String::new();
            for attr in tag.attrs {
                match attr.name.local.as_ref() {
                    "property" | "name" => name = attr.value.to_string(),
                    "content" => content = attr.value.to_string(),
                    _ => {}
                }
            }
            if (name.eq_ignore_ascii_case("og:image") || name.eq_ignore_ascii_case("twitter:image"))
                && !content.is_empty()
            {
                self.0.borrow_mut().push((name, content));
            }
        }
        TokenSinkResult::Continue
    }
}
fn metadata(body: &[u8], base: &Url) -> Option<String> {
    let tokenizer = Tokenizer::new(Meta::default(), TokenizerOpts::default());
    let queue = BufferQueue::default();
    queue.push_back(StrTendril::from(String::from_utf8_lossy(body).as_ref()));
    let _ = tokenizer.feed(&queue);
    tokenizer.end();
    let items = tokenizer.sink.0.into_inner();
    for name in ["og:image", "twitter:image"] {
        for (_, content) in items
            .iter()
            .filter(|(key, _)| key.eq_ignore_ascii_case(name))
        {
            if let Ok(url) = base.join(content.trim())
                && candidate(1, url.as_str()).is_some()
            {
                return Some(url.to_string());
            }
        }
    }
    None
}
fn raster(content_type: Option<&str>, bytes: &[u8]) -> Option<&'static str> {
    let kind = content_type?.split(';').next()?.trim();
    if kind.eq_ignore_ascii_case("image/png") && bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if kind.eq_ignore_ascii_case("image/jpeg") && bytes.starts_with(b"\xff\xd8\xff") {
        Some("image/jpeg")
    } else if kind.eq_ignore_ascii_case("image/gif")
        && (bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a"))
    {
        Some("image/gif")
    } else if kind.eq_ignore_ascii_case("image/webp")
        && bytes.len() >= 12
        && &bytes[..4] == b"RIFF"
        && &bytes[8..12] == b"WEBP"
    {
        Some("image/webp")
    } else {
        None
    }
}
async fn load(url: &str) -> Option<Image> {
    let page = collection::fetch_body(url).await.ok()?;
    let kind = page.content_type.as_deref()?.split(';').next()?.trim();
    if !kind.eq_ignore_ascii_case("text/html")
        && !kind.eq_ignore_ascii_case("application/xhtml+xml")
    {
        return None;
    }
    let image_url = metadata(page.body.as_deref()?, &page.final_url)?;
    let image = collection::fetch_body(&image_url).await.ok()?;
    let body = image.body?;
    let kind = raster(image.content_type.as_deref(), &body)?;
    Some((kind.to_owned(), body))
}
fn response(image: Option<Image>) -> Result<Response, StatusCode> {
    let (kind, body) = image.ok_or(StatusCode::NOT_FOUND)?;
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, kind)
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .header(header::CACHE_CONTROL, "private, no-store")
        .body(Body::from(body))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
pub(super) async fn get(State(db): State<Db>, Path(id): Path<i64>) -> Result<Response, StatusCode> {
    let url: String = {
        let db = db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        db.query_row("SELECT url FROM articles WHERE id=?", [id], |r| r.get(0))
            .optional()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::NOT_FOUND)?
    };
    candidate(id, &url).ok_or(StatusCode::NOT_FOUND)?;
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let key = (id, url.clone());
    if let Some(image) = cache
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .get(&key)
        .filter(|e| e.expires > Instant::now())
        .map(|e| e.image.clone())
    {
        return response(image);
    }
    let _permit = LIMIT
        .get_or_init(|| tokio::sync::Semaphore::new(4))
        .try_acquire()
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    // Recheck in case another request populated the cache after the first lookup.
    if let Some(image) = cache
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .get(&key)
        .filter(|e| e.expires > Instant::now())
        .map(|e| e.image.clone())
    {
        return response(image);
    }
    let image = tokio::time::timeout(Duration::from_secs(15), load(&url))
        .await
        .ok()
        .flatten();
    let mut entries = cache
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    entries.retain(|_, entry| entry.expires > Instant::now());
    if entries.len() >= 16
        && let Some(oldest) = entries
            .iter()
            .min_by_key(|(_, entry)| entry.expires)
            .map(|(key, _)| key.clone())
    {
        entries.remove(&oldest);
    }
    entries.insert(
        key,
        Entry {
            expires: Instant::now() + Duration::from_secs(300),
            image: image.clone(),
        },
    );
    response(image)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn metadata_and_image_validation() {
        let base = Url::parse("https://example.org/path/page").unwrap();
        assert_eq!(metadata(br#"<meta name="twitter:image" content="/fallback"><meta property="og:image" content="../hero.png">"#, &base).as_deref(), Some("https://example.org/hero.png"));
        assert!(
            metadata(
                br#"<meta property="og:image" content="http://user:pass@example.org/x">"#,
                &base
            )
            .is_none()
        );
        assert_eq!(
            raster(Some("image/png"), b"\x89PNG\r\n\x1a\nrest"),
            Some("image/png")
        );
        assert_eq!(raster(Some("image/svg+xml"), b"<svg/>"), None);
        assert_eq!(raster(Some("image/png"), b"<svg/>"), None);
        assert!(candidate(3, "http://u:p@example.org").is_none());
    }
}
