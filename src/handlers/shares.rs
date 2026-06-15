use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::RngCore;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    errors::{NotesError, Result},
    middleware::NotesUser,
    state::AppState,
};

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct NoteShare {
    pub id:               Uuid,
    pub note_id:          Uuid,
    pub created_by:       Uuid,
    pub token:            String,
    pub permission:       String,
    pub expires_at:       Option<chrono::DateTime<chrono::Utc>>,
    pub view_count:       i32,
    pub is_active:        bool,
    pub last_accessed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at:       chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize)]
pub struct CreateShareDto {
    pub expires_in_days: Option<i64>,
}

pub async fn list(
    State(state): State<AppState>,
    Extension(user): Extension<NotesUser>,
    Path(note_id): Path<Uuid>,
) -> Result<Json<Value>> {
    let shares = sqlx::query_as::<_, NoteShare>(
        "SELECT * FROM shares WHERE note_id = $1 AND created_by = $2 ORDER BY created_at DESC",
    )
    .bind(note_id)
    .bind(user.id)
    .fetch_all(&state.db)
    .await
    .map_err(NotesError::Database)?;

    Ok(Json(json!({ "shares": shares })))
}

pub async fn create(
    State(state): State<AppState>,
    Extension(user): Extension<NotesUser>,
    Path(note_id): Path<Uuid>,
    Json(dto): Json<CreateShareDto>,
) -> Result<(StatusCode, Json<Value>)> {
    // Vérifier ownership
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM notes WHERE id = $1 AND owner_id = $2)",
    )
    .bind(note_id)
    .bind(user.id)
    .fetch_one(&state.db)
    .await
    .map_err(NotesError::Database)?;

    if !exists {
        return Err(NotesError::NotFound(format!("Note {note_id}")));
    }

    let mut token_bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut token_bytes);
    let token = URL_SAFE_NO_PAD.encode(token_bytes);

    let expires_at = dto.expires_in_days.map(|d| {
        chrono::Utc::now() + chrono::Duration::days(d)
    });

    let share = sqlx::query_as::<_, NoteShare>(
        r#"INSERT INTO shares (note_id, created_by, token, expires_at)
           VALUES ($1, $2, $3, $4)
           RETURNING *"#,
    )
    .bind(note_id)
    .bind(user.id)
    .bind(&token)
    .bind(expires_at)
    .fetch_one(&state.db)
    .await
    .map_err(NotesError::Database)?;

    Ok((StatusCode::CREATED, Json(json!({ "share": share }))))
}

pub async fn delete(
    State(state): State<AppState>,
    Extension(user): Extension<NotesUser>,
    Path((note_id, share_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
    let rows = sqlx::query(
        "UPDATE shares SET is_active = FALSE WHERE id = $1 AND note_id = $2 AND created_by = $3",
    )
    .bind(share_id)
    .bind(note_id)
    .bind(user.id)
    .execute(&state.db)
    .await
    .map_err(NotesError::Database)?
    .rows_affected();

    if rows == 0 {
        return Err(NotesError::NotFound(format!("Share {share_id}")));
    }

    Ok(StatusCode::NO_CONTENT)
}
