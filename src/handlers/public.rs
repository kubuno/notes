use axum::{
    extract::{Path, State},
    Json,
};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{errors::{NotesError, Result}, state::AppState};

struct ShareRow {
    id:         Uuid,
    note_id:    Uuid,
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for ShareRow {
    fn from_row(row: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        Ok(Self {
            id:         row.try_get("id")?,
            note_id:    row.try_get("note_id")?,
            expires_at: row.try_get("expires_at")?,
        })
    }
}

pub async fn get_shared_note(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Json<Value>> {
    // Récupérer le partage
    let share = sqlx::query_as::<_, ShareRow>(
        "SELECT id, note_id, expires_at FROM shares WHERE token = $1 AND is_active = TRUE",
    )
    .bind(&token)
    .fetch_optional(&state.db)
    .await
    .map_err(NotesError::Database)?
    .ok_or_else(|| NotesError::NotFound("Partage introuvable".into()))?;

    if let Some(exp) = share.expires_at {
        if exp < chrono::Utc::now() {
            return Err(NotesError::Forbidden);
        }
    }

    // Incrémenter view_count
    let _ = sqlx::query("UPDATE shares SET view_count = view_count + 1, last_accessed_at = NOW() WHERE id = $1")
        .bind(share.id)
        .execute(&state.db)
        .await;

    // Récupérer la note
    let note = sqlx::query_as::<_, crate::models::Note>(
        "SELECT * FROM notes WHERE id = $1 AND is_trashed = FALSE",
    )
    .bind(share.note_id)
    .fetch_optional(&state.db)
    .await
    .map_err(NotesError::Database)?
    .ok_or_else(|| NotesError::NotFound("Note introuvable".into()))?;

    // Contenu HTML lu depuis le fichier .kbnot.
    let content_html = match note.file_id {
        Some(fid) => crate::services::content_files::read_note(&state, note.owner_id, fid).await
            .map(|(_, html)| html).unwrap_or_default(),
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
