//! Sync deltas for the local-first pull (notes / notebooks / labels) — same
//! contract as the office sub-modules (Msg 32): owner-scoped changes past
//! `cursor` (monotonic change_seq), live rows + tombstones, ordered, paginated.
//! `kind ∈ modified | trashed | deleted` (notebooks/labels have no trash → only
//! modified/deleted). Note changes carry their label assignments inline, and
//! `include=content` inlines the whole `.kbnot` envelope.

use axum::{
    extract::{Query, State},
    Extension, Json,
};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    errors::Result,
    middleware::NotesUser,
    models::{Label, Note, Notebook},
    services::content_files,
    state::AppState,
};

#[derive(serde::Deserialize)]
pub struct DeltaQuery {
    #[serde(default)]
    cursor: i64,
    limit: Option<i64>,
    /// `include=content` → inline the `.kbnot` envelope in each note change.
    include: Option<String>,
}

async fn union_rows(
    state: &AppState,
    user: Uuid,
    live: &str,
    tomb: &str,
    cursor: i64,
    limit: i64,
) -> Result<Vec<(Uuid, i64, String)>> {
    let rows: Vec<(Uuid, i64, String)> = sqlx::query_as(&format!(
        r#"SELECT id, change_seq, 'live'::text AS src FROM {live}
               WHERE owner_id = $1 AND change_seq > $2
           UNION ALL
           SELECT id, change_seq, 'tomb'::text AS src FROM {tomb}
               WHERE owner_id = $1 AND change_seq > $2
           ORDER BY change_seq
           LIMIT $3"#
    ))
    .bind(user)
    .bind(cursor)
    .bind(limit)
    .fetch_all(&state.db)
    .await?;
    Ok(rows)
}

/// GET /notes/delta
pub async fn notes_delta(
    State(state): State<AppState>,
    Extension(user): Extension<NotesUser>,
    Query(q): Query<DeltaQuery>,
) -> Result<Json<Value>> {
    let limit = q.limit.unwrap_or(200).clamp(1, 500);
    let rows = union_rows(&state, user.id, "notes", "note_tombstones", q.cursor, limit).await?;
    let has_more = rows.len() as i64 == limit;
    let new_cursor = rows.last().map(|r| r.1).unwrap_or(q.cursor);
    let live_ids: Vec<Uuid> = rows.iter().filter(|r| r.2 == "live").map(|r| r.0).collect();

    let items: Vec<Note> = if live_ids.is_empty() {
        Vec::new()
    } else {
        sqlx::query_as::<_, Note>("SELECT * FROM notes WHERE id = ANY($1)")
            .bind(&live_ids)
            .fetch_all(&state.db)
            .await?
    };
    // Label assignments ride along with each note.
    let links: Vec<(Uuid, Uuid)> = if live_ids.is_empty() {
        Vec::new()
    } else {
        sqlx::query_as("SELECT note_id, label_id FROM note_labels WHERE note_id = ANY($1)")
            .bind(&live_ids)
            .fetch_all(&state.db)
            .await?
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

    let mut changes = Vec::with_capacity(rows.len());
    for (id, seq, src) in &rows {
        if src == "tomb" {
            changes.push(json!({ "uuid": id, "kind": "deleted", "change_seq": seq }));
        } else if let Some(n) = item_map.get(id) {
            let empty: Vec<Uuid> = Vec::new();
            let mut change = json!({
                "uuid": id,
                "kind": if n.is_trashed { "trashed" } else { "modified" },
                "change_seq": seq,
                "note": n,
                "labels": label_map.get(id).unwrap_or(&empty),
            });
            if let Some(content) = content_map.get(id) {
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
    let rows = union_rows(&state, user.id, "notebooks", "notebook_tombstones", q.cursor, limit).await?;
    let has_more = rows.len() as i64 == limit;
    let new_cursor = rows.last().map(|r| r.1).unwrap_or(q.cursor);
    let live_ids: Vec<Uuid> = rows.iter().filter(|r| r.2 == "live").map(|r| r.0).collect();
    let items: Vec<Notebook> = if live_ids.is_empty() {
        Vec::new()
    } else {
        sqlx::query_as::<_, Notebook>("SELECT * FROM notebooks WHERE id = ANY($1)")
            .bind(&live_ids)
            .fetch_all(&state.db)
            .await?
    };
    let item_map: std::collections::HashMap<Uuid, &Notebook> = items.iter().map(|n| (n.id, n)).collect();
    let changes: Vec<Value> = rows
        .iter()
        .filter_map(|(id, seq, src)| {
            if src == "tomb" {
                Some(json!({ "uuid": id, "kind": "deleted", "change_seq": seq }))
            } else {
                item_map.get(id).map(|n| {
                    json!({ "uuid": id, "kind": "modified", "change_seq": seq, "notebook": n })
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
    let rows = union_rows(&state, user.id, "labels", "label_tombstones", q.cursor, limit).await?;
    let has_more = rows.len() as i64 == limit;
    let new_cursor = rows.last().map(|r| r.1).unwrap_or(q.cursor);
    let live_ids: Vec<Uuid> = rows.iter().filter(|r| r.2 == "live").map(|r| r.0).collect();
    let items: Vec<Label> = if live_ids.is_empty() {
        Vec::new()
    } else {
        sqlx::query_as::<_, Label>("SELECT * FROM labels WHERE id = ANY($1)")
            .bind(&live_ids)
            .fetch_all(&state.db)
            .await?
    };
    let item_map: std::collections::HashMap<Uuid, &Label> = items.iter().map(|l| (l.id, l)).collect();
    let changes: Vec<Value> = rows
        .iter()
        .filter_map(|(id, seq, src)| {
            if src == "tomb" {
                Some(json!({ "uuid": id, "kind": "deleted", "change_seq": seq }))
            } else {
                item_map.get(id).map(|l| {
                    json!({ "uuid": id, "kind": "modified", "change_seq": seq, "label": l })
                })
            }
        })
        .collect();
    Ok(Json(json!({ "changes": changes, "cursor": new_cursor, "has_more": has_more })))
}
