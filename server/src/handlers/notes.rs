use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use kubuno_db::{params, DbValue, JsonVec};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    errors::{NotesError, Result},
    events,
    middleware::NotesUser,
    models::{CreateNoteDto, ListNotesQuery, UpdateNoteDto},
    services::{backlink_service, note_service},
    state::AppState,
    sync,
};

pub async fn list(
    State(state): State<AppState>,
    Extension(user): Extension<NotesUser>,
    Query(q): Query<ListNotesQuery>,
) -> Result<Json<Value>> {
    let notes = note_service::list_notes(&state, user.id, q)
        .await
        .map_err(NotesError::Internal)?;
    Ok(Json(json!({ "notes": notes })))
}

pub async fn create(
    State(state): State<AppState>,
    Extension(user): Extension<NotesUser>,
    Json(dto): Json<CreateNoteDto>,
) -> Result<(StatusCode, Json<Value>)> {
    // Instance ceiling from the admin console; falls back to the compiled
    // default when the core has never been reached.
    let max = state.instance().max_note_size as usize;
    if dto.content.as_deref().map(|c| c.len()).unwrap_or(0) > max {
        return Err(NotesError::ContentTooLarge);
    }

    let content_clone = dto.content.clone().unwrap_or_default();
    let note = note_service::create_note(&state, user.id, dto)
        .await
        .map_err(NotesError::Internal)?;

    // Mettre à jour les backlinks en arrière-plan — seulement si les liens
    // bidirectionnels sont activés à l'échelle de l'instance.
    if state.instance().enable_bidirectional_links {
        let db2     = state.db.clone();
        let note_id = note.id;
        let uid     = user.id;
        tokio::spawn(async move {
            let _ = backlink_service::update_backlinks(note_id, uid, &content_clone, &db2).await;
        });
    }

    // Publier event (best-effort)
    {
        let ev     = events::note_created_event(note.id, user.id);
        let http   = state.http.clone();
        let core   = state.settings.core.url.clone();
        let secret = state.settings.core.internal_secret.clone();
        tokio::spawn(async move {
            let _ = events::publish_event(&http, &core, &secret, ev).await;
        });
    }

    Ok((StatusCode::CREATED, Json(json!({ "note": note }))))
}

pub async fn get(
    State(state): State<AppState>,
    Extension(user): Extension<NotesUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    let mut note = note_service::get_note(&state, id, user.id)
        .await
        .map_err(NotesError::Internal)?
        .ok_or_else(|| NotesError::NotFound(format!("Note {id}")))?;
    // Titre = nom du fichier .kbnot (sans extension) ; self-heal si renommé ailleurs.
    if let Some(fid) = note.file_id {
        if let Some(fname) = crate::services::content_files::file_name(&state, user.id, fid).await {
            let stem = crate::services::content_files::strip_ext(&fname);
            if !stem.is_empty() && note.title.as_deref() != Some(stem.as_str()) {
                // A title change is a note change → fresh change_seq in the same tx.
                let mut tx = state.db.begin().await?;
                let seq = sync::next_note_seq(&mut tx).await?;
                tx.execute(
                    "UPDATE notes.notes SET title = $1, change_seq = $2 WHERE id = $3",
                    params![&stem, seq, id],
                )
                .await?;
                tx.commit().await?;
                note.title = Some(stem);
            }
        }
    }
    Ok(Json(json!({ "note": note })))
}

#[derive(serde::Deserialize)]
pub struct OpenByFileDto {
    pub file_id: Uuid,
}

pub async fn open_by_file(
    State(state): State<AppState>,
    Extension(user): Extension<NotesUser>,
    Json(dto): Json<OpenByFileDto>,
) -> Result<Json<Value>> {
    let id = state.db.fetch_optional_scalar::<Uuid>(
        "SELECT id FROM notes.notes WHERE file_id = $1 AND owner_id = $2 AND is_trashed = $3",
        params![dto.file_id, user.id, false],
    )
    .await?
    .ok_or_else(|| NotesError::NotFound(format!("Aucune note liée au fichier {}", dto.file_id)))?;

    let note = note_service::get_note(&state, id, user.id)
        .await
        .map_err(NotesError::Internal)?
        .ok_or_else(|| NotesError::NotFound(format!("Note {id}")))?;
    Ok(Json(json!({ "note": note })))
}

pub async fn update(
    State(state): State<AppState>,
    Extension(user): Extension<NotesUser>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateNoteDto>,
) -> Result<Json<Value>> {
    // Instance ceiling from the admin console; falls back to the compiled
    // default when the core has never been reached.
    let max = state.instance().max_note_size as usize;
    if dto.content.as_deref().map(|c| c.len()).unwrap_or(0) > max {
        return Err(NotesError::ContentTooLarge);
    }

    let content_clone = dto.content.clone();
    let new_title = dto.title.clone();
    let note = note_service::update_note(&state, id, user.id, dto)
        .await
        .map_err(NotesError::Internal)?
        .ok_or_else(|| NotesError::NotFound(format!("Note {id}")))?;

    // Titre modifié → renommer le fichier .kbnot (titre = nom). Best-effort.
    if let (Some(t), Some(fid)) = (new_title.as_ref(), note.file_id) {
        if !t.trim().is_empty() {
            crate::services::content_files::rename_content_file(&state, user.id, fid, t, "kbnot").await;
        }
    }

    if let Some(content) = content_clone {
        if state.instance().enable_bidirectional_links {
            let db2 = state.db.clone();
            let uid = user.id;
            tokio::spawn(async move {
                let _ = backlink_service::update_backlinks(id, uid, &content, &db2).await;
            });
        }
    }

    Ok(Json(json!({ "note": note })))
}

pub async fn trash(
    State(state): State<AppState>,
    Extension(user): Extension<NotesUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    let found = note_service::trash_note(&state.db, id, user.id)
        .await
        .map_err(NotesError::Internal)?;
    if !found { return Err(NotesError::NotFound(format!("Note {id}"))); }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn restore(
    State(state): State<AppState>,
    Extension(user): Extension<NotesUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    let found = note_service::restore_note(&state.db, id, user.id)
        .await
        .map_err(NotesError::Internal)?;
    if !found { return Err(NotesError::NotFound(format!("Note {id}"))); }
    Ok(Json(json!({ "message": "Note restaurée" })))
}

pub async fn delete_permanent(
    State(state): State<AppState>,
    Extension(user): Extension<NotesUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    let found = note_service::delete_note(&state, id, user.id)
        .await
        .map_err(NotesError::Internal)?;
    if !found { return Err(NotesError::NotFound(format!("Note {id}"))); }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn empty_trash(
    State(state): State<AppState>,
    Extension(user): Extension<NotesUser>,
) -> Result<Json<Value>> {
    let deleted = note_service::empty_trash(&state, user.id)
        .await
        .map_err(NotesError::Internal)?;
    Ok(Json(json!({ "deleted": deleted })))
}

pub async fn duplicate(
    State(state): State<AppState>,
    Extension(user): Extension<NotesUser>,
    Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<Value>)> {
    let note = note_service::duplicate_note(&state, id, user.id)
        .await
        .map_err(NotesError::Internal)?
        .ok_or_else(|| NotesError::NotFound(format!("Note {id}")))?;
    Ok((StatusCode::CREATED, Json(json!({ "note": note }))))
}

pub async fn backlinks(
    State(state): State<AppState>,
    Extension(user): Extension<NotesUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    // Bidirectional links disabled instance-wide: no backlinks to show.
    if !state.instance().enable_bidirectional_links {
        return Ok(Json(json!({ "backlinks": [] })));
    }

    // Vérifier que la note appartient à l'utilisateur
    note_service::get_note(&state, id, user.id)
        .await
        .map_err(NotesError::Internal)?
        .ok_or_else(|| NotesError::NotFound(format!("Note {id}")))?;

    // The `mentioned_by` backlink array lives in a JSON-array column; read it as
    // a Vec<Uuid> and fetch those notes with a portable `id IN (...)`.
    let mentioned_by: Vec<Uuid> = state
        .db
        .fetch_optional_as::<MentionedByRow>(
            "SELECT mentioned_by FROM notes.notes WHERE id = $1",
            params![id],
        )
        .await
        .map_err(NotesError::Database)?
        .map(|r| r.mentioned_by.into_inner())
        .unwrap_or_default();

    let mut backlink_notes = if mentioned_by.is_empty() {
        vec![]
    } else {
        // owner_id = $1, is_trashed = $2, then the id list from $3.
        let list = state.db.backend().in_list(3, mentioned_by.len());
        let sql = format!(
            "SELECT * FROM notes.notes \
             WHERE owner_id = $1 AND is_trashed = $2 AND id IN ({list})"
        );
        let mut binds: Vec<DbValue> = params![user.id, false];
        for m in &mentioned_by {
            binds.push((*m).into());
        }
        state
            .db
            .fetch_all_as::<crate::models::Note>(&sql, binds)
            .await
            .map_err(NotesError::Database)?
    };
    // Aperçu pour l'affichage (contenu complet dans le fichier .kbnot).
    for n in &mut backlink_notes { n.content = n.preview.clone(); }

    Ok(Json(json!({ "backlinks": backlink_notes })))
}

/// The `mentioned_by` JSON-array column, read to resolve a note's backlinks.
/// `JsonVec` decodes the array on all three engines (no `#[sqlx(json)]`: the
/// wrapper carries its own Decode).
#[derive(sqlx::FromRow)]
struct MentionedByRow {
    mentioned_by: JsonVec<Uuid>,
}
