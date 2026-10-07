//! Sync deltas for the local-first pull (notes / notebooks / labels) — same
//! contract as the office sub-modules (Msg 32): owner-scoped changes past
//! `cursor` (monotonic change_seq), live rows + tombstones, ordered, paginated.
//! `kind ∈ modified | trashed | deleted` (notebooks/labels have no trash → only
//! modified/deleted). Note changes carry their label assignments inline, and
//! `include=content` inlines the whole `.kbnot` envelope.
//!
//! The change feed comes from `kubuno_db::journal::changes_since` (the portable
//! `live UNION ALL tombstones`, replacing the PostgreSQL-only sequence/trigger
//! delta layer). The row bodies are reselected as typed structs and serialised
//! in Rust — no `to_jsonb`.

use axum::{
    extract::{Query, State},
    Extension, Json,
};
use kubuno_db::{journal, DbValue};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    errors::Result,
    middleware::NotesUser,
    models::{Label, Note, Notebook},
    services::content_files,
    state::AppState,
    sync,
};

#[derive(serde::Deserialize)]
pub struct DeltaQuery {
    #[serde(default)]
    cursor: i64,
    limit: Option<i64>,
    /// `include=content` → inline the `.kbnot` envelope in each note change.
    include: Option<String>,
}

/// Reselects live rows by id, portably (`id IN (...)`, never `= ANY`).
async fn fetch_by_ids<T: kubuno_db::FromAnyRow>(
    state: &AppState,
    table: &str,
    ids: &[Uuid],
) -> Result<Vec<T>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let list = state.db.backend().in_list(1, ids.len());
    let sql = format!("SELECT * FROM {table} WHERE id IN ({list})");
    let mut binds: Vec<DbValue> = Vec::with_capacity(ids.len());
    for id in ids {
        binds.push((*id).into());
    }
    Ok(state.db.fetch_all_as::<T>(&sql, binds).await?)
}

/// GET /notes/delta
pub async fn notes_delta(
    State(state): State<AppState>,
    Extension(user): Extension<NotesUser>,
    Query(q): Query<DeltaQuery>,
) -> Result<Json<Value>> {
    let limit = q.limit.unwrap_or(200).clamp(1, 500);
    let feed = journal::changes_since(
        &state.db, sync::NOTES_TABLE, sync::NOTE_TOMBSTONES, user.id, q.cursor, limit,
    )
    .await?;
    let has_more = feed.len() as i64 == limit;
    let new_cursor = feed.last().map(|c| c.change_seq).unwrap_or(q.cursor);
    let live_ids: Vec<Uuid> = feed.iter().filter(|c| !c.deleted).map(|c| c.id).collect();

    let items: Vec<Note> = fetch_by_ids(&state, "notes.notes", &live_ids).await?;

    // Label assignments ride along with each note.
    let links: Vec<(Uuid, Uuid)> = if live_ids.is_empty() {
        Vec::new()
    } else {
        let list = state.db.backend().in_list(1, live_ids.len());
        let sql = format!("SELECT note_id, label_id FROM notes.note_labels WHERE note_id IN ({list})");
        let mut binds: Vec<DbValue> = Vec::with_capacity(live_ids.len());
        for id in &live_ids {
            binds.push((*id).into());
        }
        state
            .db
            .fetch_all_as::<LinkRow>(&sql, binds)
            .await?
            .into_iter()
            .map(|r| (r.note_id, r.label_id))
            .collect()
    };
    let mut label_map: std::collections::HashMap<Uuid, Vec<Uuid>> = std::collections::HashMap::new();
    for (nid, lid) in links {
        label_map.entry(nid).or_default().push(lid);
    }
    let item_map: std::collections::HashMap<Uuid, &Note> = items.iter().map(|n| (n.id, n)).collect();

    // include=content → inline the .kbnot envelope (best-effort).
    let mut content_map: std::collections::HashMap<Uuid, Value> = std::collections::HashMap::new();
    if q.include.as_deref() == Some("content") {
        for n in &items {
            if let Some(fid) = n.file_id {
                if let Ok((content, html)) = content_files::read_note(&state, n.owner_id, fid).await {
                    content_map.insert(
                        n.id,
                        json!({ "version": 1, "content": content, "content_html": html }),
                    );
                }
            }
        }
    }

    let mut changes = Vec::with_capacity(feed.len());
    for c in &feed {
        if c.deleted {
            changes.push(json!({ "uuid": c.id, "kind": "deleted", "change_seq": c.change_seq }));
        } else if let Some(n) = item_map.get(&c.id) {
            let empty: Vec<Uuid> = Vec::new();
            let mut change = json!({
                "uuid": c.id,
                "kind": if n.is_trashed { "trashed" } else { "modified" },
                "change_seq": c.change_seq,
                "note": n,
                "labels": label_map.get(&c.id).unwrap_or(&empty),
            });
            if let Some(content) = content_map.get(&c.id) {
                change["content"] = content.clone();
            }
            changes.push(change);
        }
    }
    Ok(Json(json!({ "changes": changes, "cursor": new_cursor, "has_more": has_more })))
}

/// GET /notebooks/delta
pub async fn notebooks_delta(
    State(state): State<AppState>,
    Extension(user): Extension<NotesUser>,
    Query(q): Query<DeltaQuery>,
) -> Result<Json<Value>> {
    let limit = q.limit.unwrap_or(200).clamp(1, 500);
    let feed = journal::changes_since(
        &state.db, sync::NOTEBOOKS_TABLE, sync::NOTEBOOK_TOMBSTONES, user.id, q.cursor, limit,
    )
    .await?;
    let has_more = feed.len() as i64 == limit;
    let new_cursor = feed.last().map(|c| c.change_seq).unwrap_or(q.cursor);
    let live_ids: Vec<Uuid> = feed.iter().filter(|c| !c.deleted).map(|c| c.id).collect();
    let items: Vec<Notebook> = fetch_by_ids(&state, "notes.notebooks", &live_ids).await?;
    let item_map: std::collections::HashMap<Uuid, &Notebook> = items.iter().map(|n| (n.id, n)).collect();
    let changes: Vec<Value> = feed
        .iter()
        .filter_map(|c| {
            if c.deleted {
                Some(json!({ "uuid": c.id, "kind": "deleted", "change_seq": c.change_seq }))
            } else {
                item_map.get(&c.id).map(|n| {
                    json!({ "uuid": c.id, "kind": "modified", "change_seq": c.change_seq, "notebook": n })
                })
            }
        })
        .collect();
    Ok(Json(json!({ "changes": changes, "cursor": new_cursor, "has_more": has_more })))
}

/// GET /labels/delta
pub async fn labels_delta(
    State(state): State<AppState>,
    Extension(user): Extension<NotesUser>,
    Query(q): Query<DeltaQuery>,
) -> Result<Json<Value>> {
    let limit = q.limit.unwrap_or(200).clamp(1, 500);
    let feed = journal::changes_since(
        &state.db, sync::LABELS_TABLE, sync::LABEL_TOMBSTONES, user.id, q.cursor, limit,
    )
    .await?;
    let has_more = feed.len() as i64 == limit;
    let new_cursor = feed.last().map(|c| c.change_seq).unwrap_or(q.cursor);
    let live_ids: Vec<Uuid> = feed.iter().filter(|c| !c.deleted).map(|c| c.id).collect();
    let items: Vec<Label> = fetch_by_ids(&state, "notes.labels", &live_ids).await?;
    let item_map: std::collections::HashMap<Uuid, &Label> = items.iter().map(|l| (l.id, l)).collect();
    let changes: Vec<Value> = feed
        .iter()
        .filter_map(|c| {
            if c.deleted {
                Some(json!({ "uuid": c.id, "kind": "deleted", "change_seq": c.change_seq }))
            } else {
                item_map.get(&c.id).map(|l| {
                    json!({ "uuid": c.id, "kind": "modified", "change_seq": c.change_seq, "label": l })
                })
            }
        })
        .collect();
    Ok(Json(json!({ "changes": changes, "cursor": new_cursor, "has_more": has_more })))
}

#[derive(sqlx::FromRow)]
struct LinkRow {
    note_id: Uuid,
    label_id: Uuid,
}
