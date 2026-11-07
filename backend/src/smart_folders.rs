use super::*;
use axum::routing::{get, patch};

pub fn migrate(tx: &rusqlite::Transaction<'_>) -> rusqlite::Result<()> {
    tx.execute("CREATE TABLE smart_folders (id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE, snapshot TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision > 0))", [])?;
    Ok(())
}

fn optional_string<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    String::deserialize(d).map(Some)
}
fn required_source<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Source>, D::Error> {
    Option::<Source>::deserialize(d)
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    kind: String,
    id: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Applied {
    order: String,
    scope: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    later: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    include_muted: Option<bool>,
    #[serde(
        default,
        deserialize_with = "optional_string",
        skip_serializing_if = "Option::is_none"
    )]
    any: Option<String>,
    #[serde(
        default,
        deserialize_with = "optional_string",
        skip_serializing_if = "Option::is_none"
    )]
    exclude: Option<String>,
    #[serde(
        default,
        deserialize_with = "optional_string",
        skip_serializing_if = "Option::is_none"
    )]
    published_from: Option<String>,
    #[serde(
        default,
        deserialize_with = "optional_string",
        skip_serializing_if = "Option::is_none"
    )]
    published_to: Option<String>,
    #[serde(
        default,
        deserialize_with = "optional_string",
        skip_serializing_if = "Option::is_none"
    )]
    feed_ids: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    filter: String,
    query: String,
    #[serde(deserialize_with = "required_source")]
    source: Option<Source>,
    applied: Applied,
}
impl Snapshot {
    fn validate_reader(&self, db: &Connection) -> Result<(), StatusCode> {
        let mut values =
            serde_json::to_value(&self.applied).map_err(|_| StatusCode::BAD_REQUEST)?;
        let values = values.as_object_mut().ok_or(StatusCode::BAD_REQUEST)?;
        values.insert("q".into(), self.query.clone().into());
        match self.filter.as_str() {
            "unread" => {
                values.insert("unread".into(), true.into());
            }
            "starred" => {
                values.insert("starred".into(), true.into());
            }
            "recent" => {
                values.insert("history".into(), true.into());
            }
            "later" => {
                values.insert("later".into(), true.into());
            }
            _ => {}
        }
        if let Some(source) = &self.source {
            values.insert(format!("{}_id", source.kind), source.id.into());
        }
        let filters: reader::Filters =
            serde_json::from_value(serde_json::Value::Object(values.clone()))
                .map_err(|_| StatusCode::BAD_REQUEST)?;
        reader::validate_filters(db, &filters)
    }
    fn valid(&self) -> bool {
        matches!(
            self.filter.as_str(),
            "all" | "unread" | "starred" | "recent" | "later"
        ) && self.query.encode_utf16().count() <= 1000
            && self.source.as_ref().is_none_or(|s| {
                matches!(s.kind.as_str(), "feed" | "folder")
                    && s.id > 0
                    && s.id <= 9_007_199_254_740_991
            })
            && matches!(self.applied.order.as_str(), "new" | "old" | "recent")
            && matches!(self.applied.scope.as_str(), "both" | "title" | "body")
            && [
                &self.applied.any,
                &self.applied.exclude,
                &self.applied.published_from,
                &self.applied.published_to,
                &self.applied.feed_ids,
            ]
            .into_iter()
            .all(|s| s.as_ref().is_none_or(|s| s.encode_utf16().count() <= 1000))
    }
}
fn valid_name(s: &str) -> bool {
    !s.trim().is_empty() && s == s.trim() && s.encode_utf16().count() <= 80
}
#[derive(Serialize)]
struct Item {
    id: i64,
    name: String,
    snapshot: Snapshot,
    revision: i64,
}
fn row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Item> {
    let json: String = r.get(2)?;
    let snapshot = serde_json::from_str(&json).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(2, rusqlite::types::Type::Text, Box::new(e))
    })?;
    Ok(Item {
        id: r.get(0)?,
        name: r.get(1)?,
        snapshot,
        revision: r.get(3)?,
    })
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Create {
    name: String,
    snapshot: Snapshot,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Update {
    name: Option<String>,
    snapshot: Option<Snapshot>,
    revision: i64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Revision {
    revision: i64,
}
fn sql_err(e: rusqlite::Error) -> StatusCode {
    if matches!(&e, rusqlite::Error::SqliteFailure(err, _) if err.code == rusqlite::ErrorCode::ConstraintViolation)
    {
        StatusCode::CONFLICT
    } else {
        collection::err(e)
    }
}
async fn list(State(db): State<Db>) -> Result<Json<Vec<Item>>, StatusCode> {
    let db = db.lock().map_err(collection::err)?;
    let mut stmt = db
        .prepare("SELECT id,name,snapshot,revision FROM smart_folders ORDER BY id")
        .map_err(sql_err)?;
    let items = stmt
        .query_map([], row)
        .map_err(sql_err)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(sql_err)?;
    Ok(Json(items))
}
async fn create(
    State(db): State<Db>,
    headers: HeaderMap,
    Json(input): Json<Create>,
) -> Result<(StatusCode, Json<Item>), StatusCode> {
    if !origin_allowed(&headers) {
        return Err(StatusCode::FORBIDDEN);
    }
    if !valid_name(&input.name) || !input.snapshot.valid() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut db = db.lock().map_err(collection::err)?;
    let tx = db.transaction().map_err(sql_err)?;
    input.snapshot.validate_reader(&tx)?;
    let count: i64 = tx
        .query_row("SELECT count(*) FROM smart_folders", [], |r| r.get(0))
        .map_err(sql_err)?;
    if count >= 100 {
        return Err(StatusCode::CONFLICT);
    }
    let snapshot = serde_json::to_string(&input.snapshot).map_err(|_| StatusCode::BAD_REQUEST)?;
    tx.execute(
        "INSERT INTO smart_folders(name,snapshot,revision) VALUES (?1,?2,1)",
        params![input.name, snapshot],
    )
    .map_err(sql_err)?;
    let item = tx
        .query_row(
            "SELECT id,name,snapshot,revision FROM smart_folders WHERE id=?1",
            [tx.last_insert_rowid()],
            row,
        )
        .map_err(sql_err)?;
    tx.commit().map_err(sql_err)?;
    Ok((StatusCode::CREATED, Json(item)))
}
async fn update(
    State(db): State<Db>,
    Path(id): Path<i64>,
    headers: HeaderMap,
    Json(input): Json<Update>,
) -> Result<Json<Item>, StatusCode> {
    if !origin_allowed(&headers) {
        return Err(StatusCode::FORBIDDEN);
    }
    if id < 1
        || input.revision < 1
        || (input.name.is_none() && input.snapshot.is_none())
        || input.name.as_deref().is_some_and(|s| !valid_name(s))
        || input.snapshot.as_ref().is_some_and(|s| !s.valid())
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut db = db.lock().map_err(collection::err)?;
    let tx = db.transaction().map_err(sql_err)?;
    let current = tx
        .query_row(
            "SELECT id,name,snapshot,revision FROM smart_folders WHERE id=?1",
            [id],
            row,
        )
        .optional()
        .map_err(sql_err)?
        .ok_or(StatusCode::NOT_FOUND)?;
    if current.revision != input.revision || current.revision == i64::MAX {
        return Err(StatusCode::CONFLICT);
    }
    if let Some(snapshot) = &input.snapshot {
        snapshot.validate_reader(&tx)?;
    }
    let name = input.name.unwrap_or(current.name);
    let snapshot = serde_json::to_string(&input.snapshot.unwrap_or(current.snapshot))
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    tx.execute("UPDATE smart_folders SET name=?1,snapshot=?2,revision=revision+1 WHERE id=?3 AND revision=?4", params![name,snapshot,id,input.revision]).map_err(sql_err)?;
    let item = tx
        .query_row(
            "SELECT id,name,snapshot,revision FROM smart_folders WHERE id=?1",
            [id],
            row,
        )
        .map_err(sql_err)?;
    tx.commit().map_err(sql_err)?;
    Ok(Json(item))
}
async fn remove(
    State(db): State<Db>,
    Path(id): Path<i64>,
    headers: HeaderMap,
    Json(input): Json<Revision>,
) -> Result<StatusCode, StatusCode> {
    if !origin_allowed(&headers) {
        return Err(StatusCode::FORBIDDEN);
    }
    if id < 1 || input.revision < 1 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let db = db.lock().map_err(collection::err)?;
    let changed = db
        .execute(
            "DELETE FROM smart_folders WHERE id=?1 AND revision=?2",
            params![id, input.revision],
        )
        .map_err(sql_err)?;
    if changed == 0 {
        let exists: bool = db
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM smart_folders WHERE id=?1)",
                [id],
                |r| r.get(0),
            )
            .map_err(sql_err)?;
        return Err(if exists {
            StatusCode::CONFLICT
        } else {
            StatusCode::NOT_FOUND
        });
    }
    Ok(StatusCode::NO_CONTENT)
}
pub fn routes() -> Router<Db> {
    Router::new()
        .route("/api/smart-folders", get(list).post(create))
        .route("/api/smart-folders/{id}", patch(update).delete(remove))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn snapshot_validation_and_migration() {
        let good = r#"{"filter":"all","query":"","source":{"kind":"feed","id":3},"applied":{"order":"new","scope":"both"}}"#;
        let value: Snapshot = serde_json::from_str(good).unwrap();
        assert!(value.valid());
        let mut bad = value.clone();
        bad.source.as_mut().unwrap().id = 0;
        assert!(!bad.valid());
        assert!(
            serde_json::from_str::<Snapshot>(&good.replace("\"query\"", "\"unknown\"")).is_err()
        );
        let mut db = Connection::open_in_memory().unwrap();
        let tx = db.transaction().unwrap();
        migrate(&tx).unwrap();
        tx.commit().unwrap();
        db.execute(
            "INSERT INTO smart_folders(name,snapshot,revision) VALUES (?1,?2,1)",
            params!["test", good],
        )
        .unwrap();
        assert!(
            db.execute(
                "INSERT INTO smart_folders(name,snapshot,revision) VALUES (?1,?2,1)",
                params!["test", good]
            )
            .is_err()
        );
    }
    #[tokio::test]
    async fn validates_reader_syntax_and_preserves_missing_sources() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_db(&dir.path().join("smart.db"), false).unwrap();
        conn.execute(
            "INSERT INTO feeds (url, title) VALUES ('https://example.invalid/feed', 'synthetic')",
            [],
        )
        .unwrap();
        let feed_id = conn.last_insert_rowid();
        let db = Arc::new(Mutex::new(conn));
        let mut snapshot: Snapshot = serde_json::from_str(&format!(r#"{{"filter":"later","query":"","source":{{"kind":"feed","id":{feed_id}}},"applied":{{"order":"new","scope":"both","later":true,"include_muted":false}}}}"#)).unwrap();
        let (_, Json(item)) = create(
            State(db.clone()),
            HeaderMap::new(),
            Json(Create {
                name: "valid".into(),
                snapshot: snapshot.clone(),
            }),
        )
        .await
        .ok()
        .unwrap();
        assert_eq!(
            serde_json::to_value(&item.snapshot).unwrap()["applied"]["include_muted"],
            false
        );
        for (field, value) in [
            ("any", "\"broken"),
            ("feed_ids", "1,1"),
            ("published_from", "2023-02-29"),
        ] {
            let mut json = serde_json::to_value(&snapshot).unwrap();
            json["applied"][field] = value.into();
            let invalid = serde_json::from_value(json).unwrap();
            assert_eq!(
                update(
                    State(db.clone()),
                    Path(item.id),
                    HeaderMap::new(),
                    Json(Update {
                        name: None,
                        snapshot: Some(invalid),
                        revision: 1
                    })
                )
                .await
                .err(),
                Some(StatusCode::BAD_REQUEST)
            );
        }
        snapshot.applied.published_from = Some("2024-12-01".into());
        snapshot.applied.published_to = Some("2024-01-01".into());
        assert_eq!(
            snapshot.validate_reader(&db.lock().unwrap()),
            Err(StatusCode::BAD_REQUEST)
        );
        db.lock()
            .unwrap()
            .execute("DELETE FROM feeds WHERE id=?", [feed_id])
            .unwrap();
        let renamed = update(
            State(db.clone()),
            Path(item.id),
            HeaderMap::new(),
            Json(Update {
                name: Some("renamed".into()),
                snapshot: None,
                revision: 1,
            }),
        )
        .await
        .ok()
        .unwrap()
        .0;
        assert_eq!(renamed.snapshot.source.unwrap().id, feed_id);
        assert_eq!(
            snapshot.validate_reader(&db.lock().unwrap()),
            Err(StatusCode::BAD_REQUEST)
        );
    }
    #[tokio::test]
    async fn stale_revisions_preserve_newer_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_db(&dir.path().join("smart.db"), false).unwrap();
        let db = Arc::new(Mutex::new(conn));
        let snapshot: Snapshot = serde_json::from_str(
            r#"{"filter":"all","query":"","source":null,"applied":{"order":"new","scope":"both"}}"#,
        )
        .unwrap();
        let (_, Json(item)) = create(
            State(db.clone()),
            HeaderMap::new(),
            Json(Create {
                name: "one".into(),
                snapshot,
            }),
        )
        .await
        .ok()
        .unwrap();
        let updated = update(
            State(db.clone()),
            Path(item.id),
            HeaderMap::new(),
            Json(Update {
                name: Some("two".into()),
                snapshot: None,
                revision: item.revision,
            }),
        )
        .await
        .ok()
        .unwrap()
        .0;
        assert_eq!(updated.revision, 2);
        assert_eq!(
            remove(
                State(db.clone()),
                Path(item.id),
                HeaderMap::new(),
                Json(Revision { revision: 1 })
            )
            .await
            .err(),
            Some(StatusCode::CONFLICT)
        );
        assert_eq!(
            update(
                State(db.clone()),
                Path(item.id),
                HeaderMap::new(),
                Json(Update {
                    name: Some("stale".into()),
                    snapshot: None,
                    revision: 1
                })
            )
            .await
            .err(),
            Some(StatusCode::CONFLICT)
        );
        assert_eq!(list(State(db)).await.ok().unwrap().0[0].name, "two");
    }
}
