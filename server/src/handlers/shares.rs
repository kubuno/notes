use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use kubuno_db::{new_id, params};
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

/// Instant `days` days from now. Split out because the expiry is computed from
/// three different branches of the ceiling rules below.
fn days_from_now(days: i64) -> chrono::DateTime<chrono::Utc> {
    chrono::Utc::now() + chrono::Duration::days(days)
}

pub async fn list(
    State(state): State<AppState>,
    Extension(user): Extension<NotesUser>,
    Path(note_id): Path<Uuid>,
) -> Result<Json<Value>> {
    let shares = state
        .db
        .fetch_all_as::<NoteShare>(
            "SELECT * FROM notes.shares WHERE note_id = $1 AND created_by = $2 ORDER BY created_at DESC",
            params![note_id, user.id],
        )
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
    // Instance policy first: no link may be minted while sharing is off.
    let cfg = state.instance();
    if !cfg.allow_public_sharing {
        return Err(NotesError::Forbidden);
    }

    // Vérifier ownership
    let exists = state
        .db
        .fetch_optional_scalar::<Uuid>(
            "SELECT id FROM notes.notes WHERE id = $1 AND owner_id = $2",
            params![note_id, user.id],
        )
        .await
        .map_err(NotesError::Database)?;

    if exists.is_none() {
        return Err(NotesError::NotFound(format!("Note {note_id}")));
    }

    let mut token_bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut token_bytes);
    let token = URL_SAFE_NO_PAD.encode(token_bytes);

    // Lifetime ceiling: an unbounded link, or one asking for longer than the
    // administrator allows, is clamped instead of refused — the user still gets
    // a link, it just does not outlive the policy. `0` means "no ceiling".
    let requested = dto.expires_in_days.filter(|d| *d > 0);
    let expires_at = match (requested, cfg.share_link_max_days) {
        (_, 0)                            => requested.map(days_from_now),
        (None, ceiling)                   => Some(days_from_now(ceiling)),
        (Some(d), ceiling) if d > ceiling => Some(days_from_now(ceiling)),
        (Some(d), _)                      => Some(days_from_now(d)),
    };

    // Id generated in Rust (no gen_random_uuid / RETURNING on MySQL); the row is
    // read back by id. Shares are not part of the delta sync.
    let id = new_id();
    let now = chrono::Utc::now();
    state
        .db
        .execute(
            "INSERT INTO notes.shares (id, note_id, created_by, token, expires_at, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6)",
            params![id, note_id, user.id, &token, expires_at, now],
        )
        .await
        .map_err(NotesError::Database)?;

    let share = state
        .db
        .fetch_one_as::<NoteShare>("SELECT * FROM notes.shares WHERE id = $1", params![id])
        .await
        .map_err(NotesError::Database)?;

    Ok((StatusCode::CREATED, Json(json!({ "share": share }))))
}

pub async fn delete(
    State(state): State<AppState>,
    Extension(user): Extension<NotesUser>,
    Path((note_id, share_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
    let rows = state
        .db
        .execute(
            "UPDATE notes.shares SET is_active = $1 WHERE id = $2 AND note_id = $3 AND created_by = $4",
            params![false, share_id, note_id, user.id],
        )
        .await
        .map_err(NotesError::Database)?;

    if rows == 0 {
        return Err(NotesError::NotFound(format!("Share {share_id}")));
    }

    Ok(StatusCode::NO_CONTENT)
}
