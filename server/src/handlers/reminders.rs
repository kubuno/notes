use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    errors::{NotesError, Result},
    middleware::NotesUser,
    models::{CreateReminderDto, UpdateReminderDto},
    services::reminder_service,
    state::AppState,
};

pub async fn list(
    State(state): State<AppState>,
    Extension(user): Extension<NotesUser>,
    Path(note_id): Path<Uuid>,
) -> Result<Json<Value>> {
    let reminders = reminder_service::list_reminders(&state.db, note_id, user.id)
        .await
        .map_err(NotesError::Internal)?;
    Ok(Json(json!({ "reminders": reminders })))
}

pub async fn create(
    State(state): State<AppState>,
    Extension(user): Extension<NotesUser>,
    Path(note_id): Path<Uuid>,
    Json(dto): Json<CreateReminderDto>,
) -> Result<(StatusCode, Json<Value>)> {
    let reminder = reminder_service::create_reminder(&state.db, note_id, user.id, dto)
        .await
        .map_err(NotesError::Internal)?;
    Ok((StatusCode::CREATED, Json(json!({ "reminder": reminder }))))
}

pub async fn update(
    State(state): State<AppState>,
    Extension(user): Extension<NotesUser>,
    Path((note_id, rid)): Path<(Uuid, Uuid)>,
    Json(dto): Json<UpdateReminderDto>,
) -> Result<Json<Value>> {
    let reminder = reminder_service::update_reminder(&state.db, rid, note_id, user.id, dto)
        .await
        .map_err(NotesError::Internal)?
        .ok_or_else(|| NotesError::NotFound(format!("Reminder {rid}")))?;
    Ok(Json(json!({ "reminder": reminder })))
}

pub async fn delete(
    State(state): State<AppState>,
    Extension(user): Extension<NotesUser>,
    Path((note_id, rid)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
    let found = reminder_service::delete_reminder(&state.db, rid, note_id, user.id)
        .await
        .map_err(NotesError::Internal)?;
    if !found { return Err(NotesError::NotFound(format!("Reminder {rid}"))); }
    Ok(StatusCode::NO_CONTENT)
}
