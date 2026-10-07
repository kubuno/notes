use axum::{
    extract::{Path, State},
    Json,
};
use kubuno_db::params;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{errors::{NotesError, Result}, state::AppState};

#[derive(sqlx::FromRow)]
struct ShareRow {
    id:         Uuid,
    note_id:    Uuid,
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub async fn get_shared_note(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Json<Value>> {
    // Turning link sharing off must also close the links already handed out,
    // otherwise the policy only applies to people who have not shared yet. The
    // answer is the same 404 an unknown token gets: a closed instance must not
    // reveal that the token was valid.
    if !state.instance().allow_public_sharing {
        return Err(NotesError::NotFound("Partage introuvable".into()));
    }

    // Récupérer le partage
    let share = state
        .db
        .fetch_optional_as::<ShareRow>(
            "SELECT id, note_id, expires_at FROM notes.shares WHERE token = $1 AND is_active = $2",
            params![&token, true],
        )
        .await
        .map_err(NotesError::Database)?
        .ok_or_else(|| NotesError::NotFound("Partage introuvable".into()))?;

    if let Some(exp) = share.expires_at {
        if exp < chrono::Utc::now() {
            return Err(NotesError::Forbidden);
        }
    }

    // Incrémenter view_count (les partages ne font pas partie de la sync delta).
    let _ = state
        .db
        .execute(
            "UPDATE notes.shares SET view_count = view_count + 1, last_accessed_at = $1 WHERE id = $2",
            params![chrono::Utc::now(), share.id],
        )
        .await;

    // Récupérer la note
    let note = state
        .db
        .fetch_optional_as::<crate::models::Note>(
            "SELECT * FROM notes.notes WHERE id = $1 AND is_trashed = $2",
            params![share.note_id, false],
        )
        .await
        .map_err(NotesError::Database)?
        .ok_or_else(|| NotesError::NotFound("Note introuvable".into()))?;

    // Contenu HTML lu depuis le fichier .kbnot.
    let content_html = match note.file_id {
        Some(fid) => crate::services::content_files::read_note(&state, note.owner_id, fid)
            .await
            .map(|(_, html)| html)
            .unwrap_or_default(),
        None => String::new(),
    };

    Ok(Json(json!({
        "note": {
            "id":           note.id,
            "title":        note.title,
            "content_html": content_html,
            "note_type":    note.note_type,
            "color":        note.color,
            "created_at":   note.created_at,
            "updated_at":   note.updated_at,
        }
    })))
}
