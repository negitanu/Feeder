use super::*;
use axum::response::Response;
use quick_xml::{Reader, events::Event};
use std::{
    collections::{HashMap, HashSet},
    net::IpAddr,
};
use url::Url;

const MAX_XML: usize = 1024 * 1024;
const MAX_OUTLINES: usize = 1000;
const MAX_DEPTH: usize = 16;

#[derive(Clone, Serialize)]
struct Item {
    url: String,
    title: String,
    folder: Option<String>,
    status: &'static str,
}
#[derive(Serialize)]
struct Preview {
    items: Vec<Item>,
    valid: usize,
    new: usize,
    existing: usize,
    duplicates: usize,
    invalid: usize,
}
#[derive(Serialize)]
struct ImportResult {
    added: usize,
    existing: usize,
    duplicates: usize,
    invalid: usize,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ImportInput {
    xml: String,
}

pub(super) fn safe_url(raw: &str) -> Option<String> {
    let mut u = Url::parse(raw.trim()).ok()?;
    if !matches!(u.scheme(), "http" | "https")
        || !u.username().is_empty()
        || u.password().is_some()
        || u.host().is_none()
    {
        return None;
    }
    if let Some(h) = u.host_str() {
        let domain = h.trim_end_matches('.');
        if domain.eq_ignore_ascii_case("localhost")
            || domain.ends_with(".localhost")
            || domain.ends_with(".local")
        {
            return None;
        }
        if let Ok(ip) = h.trim_matches(['[', ']']).parse::<IpAddr>()
            && !collection::public_ip(ip)
        {
            return None;
        }
    }
    u.set_fragment(None);
    Some(u.to_string())
}
fn parse(xml: &str) -> Result<Vec<(String, String, Option<String>)>, StatusCode> {
    if xml.len() > MAX_XML || xml.contains('\0') {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }
    let upper = xml.to_ascii_uppercase();
    if upper.contains("<!DOCTYPE") || upper.contains("<!ENTITY") {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut folders: Vec<String> = Vec::new();
    let mut outline_stack: Vec<bool> = Vec::new();
    let mut elems: Vec<Vec<u8>> = Vec::new();
    let mut entries = Vec::new();
    let mut count = 0usize;
    let mut root = false;
    let mut body_seen = false;
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = e.name().as_ref().to_ascii_lowercase();
                if elems.is_empty() {
                    if name != b"opml" || root {
                        return Err(StatusCode::BAD_REQUEST);
                    }
                    root = true;
                }
                if name == b"body" && elems.len() == 1 {
                    if body_seen {
                        return Err(StatusCode::BAD_REQUEST);
                    }
                    body_seen = true;
                }
                elems.push(name.clone());
                if elems.len() > MAX_DEPTH {
                    return Err(StatusCode::BAD_REQUEST);
                }
                if name == b"outline" {
                    if !elems.iter().any(|e| e == b"body") {
                        return Err(StatusCode::BAD_REQUEST);
                    }
                    count += 1;
                    if count > MAX_OUTLINES || folders.len() >= MAX_DEPTH {
                        return Err(StatusCode::BAD_REQUEST);
                    }
                    let mut attrs = HashMap::new();
                    for a in e.attributes().with_checks(true) {
                        let a = a.map_err(|_| StatusCode::BAD_REQUEST)?;
                        let k = String::from_utf8_lossy(a.key.as_ref()).to_ascii_lowercase();
                        let v = a
                            .decoded_and_normalized_value(
                                quick_xml::XmlVersion::Implicit1_0,
                                reader.decoder(),
                            )
                            .map_err(|_| StatusCode::BAD_REQUEST)?
                            .into_owned();
                        attrs.insert(k, v);
                    }
                    let title = attrs
                        .get("title")
                        .or_else(|| attrs.get("text"))
                        .cloned()
                        .unwrap_or_default();
                    let url = attrs.get("xmlurl").or_else(|| attrs.get("url")).cloned();
                    let is_folder = url.is_none();
                    if let Some(url) = url {
                        entries.push((
                            url,
                            title,
                            if folders.is_empty() {
                                None
                            } else {
                                Some(folders.join("/"))
                            },
                        ));
                    } else {
                        folders.push(title);
                    }
                    outline_stack.push(is_folder);
                }
            }
            Ok(Event::Empty(e)) if e.name().as_ref().eq_ignore_ascii_case(b"outline") => {
                if !elems.iter().any(|e| e == b"body") || elems.len() + 1 > MAX_DEPTH {
                    return Err(StatusCode::BAD_REQUEST);
                }
                count += 1;
                if count > MAX_OUTLINES || folders.len() >= MAX_DEPTH {
                    return Err(StatusCode::BAD_REQUEST);
                }
                let mut attrs = HashMap::new();
                for a in e.attributes().with_checks(true) {
                    let a = a.map_err(|_| StatusCode::BAD_REQUEST)?;
                    let k = String::from_utf8_lossy(a.key.as_ref()).to_ascii_lowercase();
                    let v = a
                        .decoded_and_normalized_value(
                            quick_xml::XmlVersion::Implicit1_0,
                            reader.decoder(),
                        )
                        .map_err(|_| StatusCode::BAD_REQUEST)?
                        .into_owned();
                    attrs.insert(k, v);
                }
                if let Some(url) = attrs.get("xmlurl").or_else(|| attrs.get("url")) {
                    let title = attrs
                        .get("title")
                        .or_else(|| attrs.get("text"))
                        .cloned()
                        .unwrap_or_default();
                    entries.push((
                        url.clone(),
                        title,
                        if folders.is_empty() {
                            None
                        } else {
                            Some(folders.join("/"))
                        },
                    ));
                }
            }
            Ok(Event::Empty(e))
                if e.name().as_ref().eq_ignore_ascii_case(b"body") && elems.len() == 1 =>
            {
                if body_seen {
                    return Err(StatusCode::BAD_REQUEST);
                }
                body_seen = true;
            }
            Ok(Event::Empty(_)) if elems.is_empty() || elems.len() + 1 > MAX_DEPTH => {
                return Err(StatusCode::BAD_REQUEST);
            }
            Ok(Event::Text(e))
                if elems.is_empty() && !e.as_ref().iter().all(u8::is_ascii_whitespace) =>
            {
                return Err(StatusCode::BAD_REQUEST);
            }
            Ok(Event::CData(_)) if elems.is_empty() => return Err(StatusCode::BAD_REQUEST),
            Ok(Event::End(e)) => {
                let name = e.name().as_ref().to_ascii_lowercase();
                if elems.pop().as_deref() != Some(name.as_slice()) {
                    return Err(StatusCode::BAD_REQUEST);
                }
                if name == b"outline" && outline_stack.pop().unwrap_or(false) {
                    folders.pop();
                }
            }
            Ok(Event::DocType(_)) => return Err(StatusCode::BAD_REQUEST),
            Ok(Event::Eof) => break,
            Err(_) => return Err(StatusCode::BAD_REQUEST),
            _ => {}
        }
    }
    if !root || !body_seen || !elems.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    Ok(entries)
}
fn existing(db: &Connection) -> Result<HashSet<String>, StatusCode> {
    let mut st = db
        .prepare("SELECT url FROM feeds")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let rows = st
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(rows
        .filter_map(Result::ok)
        .filter_map(|u| safe_url(&u))
        .collect())
}
fn summarize(db: &Connection, xml: &str) -> Result<Preview, StatusCode> {
    let parsed = parse(xml)?;
    let already = existing(db)?;
    let mut seen = HashSet::new();
    let mut items = Vec::new();
    let (mut valid, mut new, mut ex, mut dup, mut invalid) = (0, 0, 0, 0, 0);
    for (raw, title, folder) in parsed {
        let Some(url) = safe_url(&raw).filter(|u| u.len() <= 8192) else {
            invalid += 1;
            items.push(Item {
                url: raw,
                title,
                folder,
                status: "invalid",
            });
            continue;
        };
        if title.trim().len() > 512 || folder.as_ref().is_some_and(|f| f.len() > 512) {
            invalid += 1;
            items.push(Item {
                url,
                title,
                folder,
                status: "invalid",
            });
            continue;
        }
        valid += 1;
        let status = if !seen.insert(url.clone()) {
            dup += 1;
            "duplicate"
        } else if already.contains(&url) {
            ex += 1;
            "existing"
        } else {
            new += 1;
            "new"
        };
        items.push(Item {
            url: url.clone(),
            title: if title.trim().is_empty() {
                url.clone()
            } else {
                title.trim().to_string()
            },
            folder,
            status,
        });
    }
    Ok(Preview {
        items,
        valid,
        new,
        existing: ex,
        duplicates: dup,
        invalid,
    })
}
async fn preview(
    State(db): State<Db>,
    headers: HeaderMap,
    Json(input): Json<ImportInput>,
) -> Result<Json<Preview>, StatusCode> {
    if !origin_allowed(&headers) {
        return Err(StatusCode::FORBIDDEN);
    }
    if input.xml.len() > MAX_XML {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }
    let db = db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(summarize(&db, &input.xml)?))
}
async fn import(
    State(db): State<Db>,
    headers: HeaderMap,
    Json(input): Json<ImportInput>,
) -> Result<Json<ImportResult>, StatusCode> {
    if !origin_allowed(&headers) {
        return Err(StatusCode::FORBIDDEN);
    }
    if input.xml.len() > MAX_XML {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }
    let mut db = db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let _preview = summarize(&db, &input.xml)?;
    let tx = db
        .transaction()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut folders = HashMap::new();
    let mut seen = HashSet::new();
    let old = existing(&tx)?;
    let parsed = parse(&input.xml)?;
    let (mut added, mut already, mut duplicates, mut invalid) = (0, 0, 0, 0);
    for (raw, title, folder) in parsed {
        let Some(url) = safe_url(&raw).filter(|u| u.len() <= 8192) else {
            invalid += 1;
            continue;
        };
        if title.trim().len() > 512 || folder.as_ref().is_some_and(|f| f.len() > 512) {
            invalid += 1;
            continue;
        }
        if !seen.insert(url.clone()) {
            duplicates += 1;
            continue;
        }
        if old.contains(&url) {
            already += 1;
            continue;
        }
        let folder_id = if let Some(path) = folder {
            let id = if let Some(id) = folders.get(&path) {
                *id
            } else {
                tx.execute("INSERT OR IGNORE INTO folders(name) VALUES (?)", [&path])
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                let id = tx
                    .query_row("SELECT id FROM folders WHERE name=?", [&path], |r| {
                        r.get::<_, i64>(0)
                    })
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                folders.insert(path, id);
                id
            };
            Some(id)
        } else {
            None
        };
        tx.execute(
            "INSERT INTO feeds(url,title,folder_id,paused) VALUES (?,?,?,0)",
            params![
                url,
                if title.trim().is_empty() {
                    url.as_str()
                } else {
                    title.trim()
                },
                folder_id
            ],
        )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        added += 1;
    }
    tx.commit().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(ImportResult {
        added,
        existing: already,
        duplicates,
        invalid,
    }))
}
async fn export(State(db): State<Db>) -> Result<Response, StatusCode> {
    let db = db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut st=db.prepare("SELECT f.url,f.title,f.paused,COALESCE(d.name,'') FROM feeds f LEFT JOIN folders d ON d.id=f.folder_id ORDER BY d.name,f.title").map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
    let rows = st
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, bool>(2)?,
                r.get::<_, String>(3)?,
            ))
        })
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut out = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<opml version=\"2.0\" xmlns:feeder=\"https://feeder.invalid/ns/opml\"><head><title>Feeder subscriptions</title></head><body>",
    );
    let mut folder = String::new();
    for row in rows {
        let (url, title, paused, name) = row.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if name != folder {
            if !folder.is_empty() {
                out.push_str("</outline>")
            }
            folder = name.clone();
            if !folder.is_empty() {
                out.push_str(&format!("<outline text=\"{}\">", esc(&folder)));
            }
        }
        out.push_str(&format!("<outline text=\"{}\" title=\"{}\" type=\"rss\" xmlUrl=\"{}\" htmlUrl=\"{}\" feeder:paused=\"{}\"/>",esc(&title),esc(&title),esc(&url),esc(&url),paused));
    }
    if !folder.is_empty() {
        out.push_str("</outline>")
    }
    out.push_str("</body></opml>");
    Response::builder()
        .header(header::CONTENT_TYPE, "text/x-opml; charset=utf-8")
        .header(
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"subscriptions.opml\"",
        )
        .body(axum::body::Body::from(out))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\'', "&apos;")
}
pub fn routes() -> Router<Db> {
    Router::new()
        .route("/api/opml/preview", post(preview))
        .route("/api/opml/import", post(import))
        .route("/api/opml/export", get(export))
}

#[cfg(test)]
mod tests {
    use super::*;
    const XML: &str = "<opml version='2.0'><body><outline text='Tech'><outline text='Feed' xmlUrl='https://example.org/rss#top'/></outline></body></opml>";
    #[test]
    fn rejects_invalid_xml_roots_and_literal_ipv6() {
        for xml in [
            "<opml><body/></opml><extra/>",
            "text<opml><body/></opml>",
            "<opml><body/><body/></opml>",
            "<opml><body><outline xmlUrl='https://example.org' xmlUrl='https://other.org'/></body></opml>",
        ] {
            assert!(parse(xml).is_err(), "{xml}");
        }
        let deep = format!(
            "<opml><body>{}<outline xmlUrl='https://example.org'/>{}</body></opml>",
            "<x>".repeat(14),
            "</x>".repeat(14)
        );
        assert!(parse(&deep).is_err());
        for url in [
            "http://[::1]/",
            "http://[fc00::1]/",
            "http://[::ffff:8.8.8.8]/",
        ] {
            assert!(safe_url(url).is_none(), "{url}");
        }
        let paired = parse("<opml><body><outline text='Folder'><outline xmlUrl='https://example.org/a'></outline><outline xmlUrl='https://example.org/b'/></outline></body></opml>").unwrap();
        assert_eq!(paired[1].2.as_deref(), Some("Folder"));
    }
    #[test]
    fn bounds_xml_and_normalizes_urls() {
        let p = parse(XML).unwrap();
        assert_eq!(p[0].2.as_deref(), Some("Tech"));
        assert_eq!(
            safe_url(&p[0].0).as_deref(),
            Some("https://example.org/rss")
        );
        assert!(parse("<!DOCTYPE x><opml><body/></opml>").is_err());
        assert!(safe_url("http://127.0.0.1/feed").is_none());
        assert!(safe_url("http://localhost/feed").is_none());
    }
    #[tokio::test]
    async fn bulk_import_preserves_existing_and_nested_folders() {
        let dir = tempfile::tempdir().unwrap();
        let db = Arc::new(Mutex::new(
            open_db(&dir.path().join("opml.db"), false).unwrap(),
        ));
        let mut xml = String::from("<opml><body><outline text='Tech'><outline text='Security'>");
        for i in 0..950 {
            xml.push_str(&format!(
                "<outline text='Feed {i}' xmlUrl='https://example.invalid/feed/{i}'/>"
            ));
        }
        xml.push_str("<outline text='Duplicate' xmlUrl='https://example.invalid/feed/0#fragment'/></outline></outline></body></opml>");
        let preview = preview(
            State(db.clone()),
            HeaderMap::new(),
            Json(ImportInput { xml: xml.clone() }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!((preview.new, preview.duplicates), (950, 1));
        assert_eq!(
            db.lock()
                .unwrap()
                .query_row("SELECT count(*) FROM feeds", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            0
        );
        let result = import(
            State(db.clone()),
            HeaderMap::new(),
            Json(ImportInput { xml: xml.clone() }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!((result.added, result.duplicates), (950, 1));
        {
            let conn = db.lock().unwrap();
            assert_eq!(conn.query_row("SELECT count(*) FROM feeds f JOIN folders d ON d.id=f.folder_id WHERE d.name='Tech/Security' AND f.paused=0", [], |r| r.get::<_, i64>(0)).unwrap(), 950);
        }
        let second = import(State(db), HeaderMap::new(), Json(ImportInput { xml }))
            .await
            .unwrap()
            .0;
        assert_eq!(
            (second.added, second.existing, second.duplicates),
            (0, 950, 1)
        );
    }
    #[tokio::test]
    async fn preview_and_import_are_explicit_and_collection_ready() {
        use tower::ServiceExt;
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute(SCHEMA, []).unwrap();
        {
            let tx = conn.transaction().unwrap();
            collection::migrate(&tx).unwrap();
            tx.commit().unwrap();
        }
        let state = Arc::new(Mutex::new(conn));
        let app = routes().with_state(state.clone());
        let body = serde_json::json!({"xml":XML}).to_string();
        let request = || {
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/opml/preview")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(body.clone()))
                .unwrap()
        };
        let response = app.clone().oneshot(request()).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let response = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/opml/import")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(body.clone()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        {
            let db = state.lock().unwrap();
            assert!(
                !db.query_row("SELECT paused FROM feeds", [], |r| r.get::<_, bool>(0))
                    .unwrap()
            );
            assert!(
                db.query_row("SELECT enabled FROM collection_settings", [], |r| r
                    .get::<_, bool>(0))
                    .unwrap()
            );
            db.execute("UPDATE feeds SET paused=1", []).unwrap();
            db.execute("UPDATE collection_settings SET enabled=0", [])
                .unwrap();
        }
        let duplicate = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/opml/import")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(body.clone()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(duplicate.status(), StatusCode::OK);
        {
            let db = state.lock().unwrap();
            assert!(
                db.query_row("SELECT paused FROM feeds", [], |r| r.get::<_, bool>(0))
                    .unwrap()
            );
            assert!(
                !db.query_row("SELECT enabled FROM collection_settings", [], |r| r
                    .get::<_, bool>(0))
                    .unwrap()
            );
        }
        let response = app.clone().oneshot(request()).await.unwrap();
        let bytes = axum::body::to_bytes(response.into_body(), MAX_XML)
            .await
            .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value["new"], 0);
        assert_eq!(value["existing"], 1);
        let db = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/opml/export")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(db.status(), StatusCode::OK);
    }
}
